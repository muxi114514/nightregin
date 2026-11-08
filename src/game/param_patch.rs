//! 按字段改写游戏内存里的参数表：启动时（表还是原值）记下地址与原值，之后总从原值计算新值，还原时逐字节写回。
//! 只影响本机。写入只能在游戏主线程（每帧任务）里做。

use log::warn;
use nr_stat_alloc_shared::param::ParamFile;
use nr_stat_alloc_shared::param::layout::Table;
use nr_stat_alloc_shared::tiered_params::Change;

use super::memory::{read_bytes, write_bytes};
use super::params::ParamRepository;

/// 一个字段：游戏内存中的地址、原值与改法
pub struct PatchField {
    address: usize,
    original: Vec<u8>,
    change: Change,
}

impl PatchField {
    pub fn new(address: usize, original: Vec<u8>, change: Change) -> Self {
        Self { address, original, change }
    }

    /// 按份数写入新值（从原值计算，重复写不会累积）
    pub fn write(&self, stacks: u32) -> bool {
        self.change.apply(&self.original, stacks).is_some_and(|b| write_bytes(self.address, &b))
    }

    pub fn restore(&self) -> bool {
        write_bytes(self.address, &self.original)
    }
}

/// 在同类型名的各张表里找要改的行；AtkParam_Pc 与 _Npc 共用类型名，取含这些行最多的那张
pub fn locate_rows(repository: &ParamRepository, table: Table, rows: &[u32], offset: usize, change: Change) -> Vec<PatchField> {
    let width = change.width();
    repository
        .table_locations(table.struct_name)
        .into_iter()
        .filter_map(|(address, size)| {
            let bytes = read_bytes(address, size as usize)?;
            let file = ParamFile::parse(&bytes, table.row_size).map_err(|e| warn!("{}: {e}", table.struct_name)).ok()?;
            let fields: Vec<PatchField> = file
                .rows()
                .filter(|r| rows.contains(&r.id))
                .filter_map(|r| {
                    let original = r.data.get(offset..offset + width)?.to_vec();
                    Some(PatchField::new(address + r.offset + offset, original, change))
                })
                .collect();
            Some(fields)
        })
        .max_by_key(Vec::len)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use nr_stat_alloc_shared::tiered_params::Change;

    use super::*;

    fn field(buf: &mut [u8; 4], change: Change) -> PatchField {
        PatchField::new(buf.as_mut_ptr() as usize, buf.to_vec(), change)
    }

    #[test]
    fn write_scales_from_original() {
        let mut buf = 2.0f32.to_le_bytes();
        let f = field(&mut buf, Change::ScaleF32(1.5));
        assert!(f.write(1));
        assert_eq!(f32::from_le_bytes(buf), 3.0);
    }

    #[test]
    fn write_twice_does_not_compound() {
        let mut buf = 2.0f32.to_le_bytes();
        let f = field(&mut buf, Change::ScaleF32(1.5));
        assert!(f.write(1) && f.write(1));
        assert_eq!(f32::from_le_bytes(buf), 3.0);
    }

    #[test]
    fn restore_puts_back_original_bytes() {
        let mut buf = 1.718f32.to_le_bytes();
        let before = buf;
        let f = field(&mut buf, Change::ScaleF32(1.25));
        assert!(f.write(1));
        assert_ne!(buf, before);
        assert!(f.restore());
        assert_eq!(buf, before);
    }
}
