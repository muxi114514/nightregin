//! 声明式的新增行：以某行为模板复制、换 ID 和行名、改若干字段。新增词条只需在 content/ 下追加列表

use crate::layout::Table;

pub struct RowSpec {
    pub table: &'static Table,
    pub template: u32,
    pub id: u32,
    /// 写入时自动加上 NAME_PREFIX
    pub name: &'static str,
    pub sets: &'static [Set],
}

/// 按开关改写已写入的行（其他行定义新增的或原版行）的个别字段，在同表全部 RowSpec 之后按 ID 定位
pub struct Patch {
    pub table: &'static Table,
    pub id: u32,
    pub sets: &'static [Set],
}

#[derive(Clone, Copy)]
pub enum Set {
    I32(usize, i32),
    U16(usize, u16),
    U8(usize, u8),
    F32(usize, f32),
    /// 位域：(字节偏移, 位, 值)
    Bit(usize, u8, bool),
}

/// 本工具写入的行名前缀：重复运行时据此替换旧行，同 ID 但不是本工具写的行视为冲突
pub const NAME_PREFIX: &str = "[MMVRelic] ";

impl Set {
    pub fn apply(self, row: &mut [u8]) {
        let (offset, bytes) = match self {
            Set::I32(o, v) => (o, v.to_le_bytes().to_vec()),
            Set::U16(o, v) => (o, v.to_le_bytes().to_vec()),
            Set::U8(o, v) => (o, vec![v]),
            Set::F32(o, v) => (o, v.to_le_bytes().to_vec()),
            Set::Bit(o, bit, on) => {
                let mask = 1u8 << bit;
                row[o] = if on { row[o] | mask } else { row[o] & !mask };
                return;
            }
        };
        row[offset..offset + bytes.len()].copy_from_slice(&bytes);
    }

    pub fn offset(self) -> usize {
        match self {
            Set::I32(o, _) | Set::U16(o, _) | Set::U8(o, _) | Set::F32(o, _) | Set::Bit(o, _, _) => o,
        }
    }
}
