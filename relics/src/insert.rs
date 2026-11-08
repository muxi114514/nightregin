//! 把 RowSpec 写进参数表。游戏加载时用 std::map 按 ID 重建有序索引（重复 ID 取先出现的行），文件内顺序不影响查找：
//! - 有序表：新行插在比它大的最小 ID 之前，表仍然有序
//! - 已经乱序或有重复 ID 的表（MMV 的 AtkParam_Pc、空城包的多张表）：原有行一字节不动，新行追加在末尾
//! 同 ID 已有本工具写的行就原地替换，content::IN_PLACE 登记的原版行原地改写，其余被占用则报错。

use nr_stat_alloc_patcher::param_write::{self, OwnedRow};
use nr_stat_alloc_patcher::regulation::Regulation;
use nr_stat_alloc_shared::param::ParamFile;

use crate::layout::Table;
use crate::spec::{NAME_PREFIX, Patch, RowSpec};

#[derive(Default)]
pub struct Report {
    pub added: usize,
    pub replaced: usize,
    pub in_place: usize,
    pub patched: usize,
}

pub fn open<'a>(reg: &'a Regulation, t: &Table) -> Result<ParamFile<'a>, String> {
    let bytes = reg.param(t.file).ok_or(format!("{} not found in regulation", t.file))?;
    let file = ParamFile::parse(bytes, t.row_size).map_err(|e| format!("{}: {e}", t.file))?;
    if file.struct_name() != Some(t.struct_name) {
        return Err(format!("{} has unexpected type {:?}", t.file, file.struct_name()));
    }
    Ok(file)
}

/// 返回该表新的文件内容；in_place 为该表允许原地覆盖的原版行，patches 在全部新行写好后再改字段
pub fn apply(file: &ParamFile, specs: &[&RowSpec], patches: &[&Patch], in_place: &[u32], report: &mut Report) -> Result<Vec<u8>, String> {
    let mut rows = param_write::rows_of(file);
    let sorted = rows.windows(2).all(|w| w[0].id < w[1].id);
    let mut taken = Vec::new();
    for spec in specs {
        let mut data = file.row(spec.template).ok_or(format!("template {} not found", spec.template))?.to_vec();
        for set in spec.sets {
            set.apply(&mut data);
        }
        let name = Some(format!("{NAME_PREFIX}{}", spec.name));
        match rows.iter().position(|r| r.id == spec.id) {
            Some(i) if rows[i].name.as_deref().is_some_and(|n| n.starts_with(NAME_PREFIX)) => {
                rows[i] = OwnedRow { id: spec.id, data, name };
                report.replaced += 1;
            }
            Some(i) if in_place.contains(&spec.id) => {
                rows[i] = OwnedRow { id: spec.id, data, name };
                report.in_place += 1;
            }
            Some(_) => taken.push(spec.id),
            None if !sorted => {
                rows.push(OwnedRow { id: spec.id, data, name });
                report.added += 1;
            }
            None => {
                let at = insert_position(&rows, spec.id)?;
                rows.insert(at, OwnedRow { id: spec.id, data, name });
                report.added += 1;
            }
        }
    }
    if !taken.is_empty() {
        return Err(format!("rows used by another mod: {taken:?}"));
    }
    for patch in patches {
        let row = rows.iter_mut().find(|r| r.id == patch.id).ok_or(format!("patch target {} not found", patch.id))?;
        for set in patch.sets {
            set.apply(&mut row.data);
        }
        report.patched += 1;
    }
    param_write::write_ordered(file, &rows)
}

/// 比 id 大的最小 ID 所在位置；要求它前一行比 id 小，保证二分查找能找到新行
fn insert_position(rows: &[OwnedRow], id: u32) -> Result<usize, String> {
    let next = rows.iter().enumerate().filter(|(_, r)| r.id > id).min_by_key(|(_, r)| r.id).map(|(i, _)| i);
    let at = next.unwrap_or(rows.len());
    if at > 0 && rows[at - 1].id >= id {
        return Err(format!("cannot place row {id}: neighbours {} / {:?} are out of order", rows[at - 1].id, rows.get(at).map(|r| r.id)));
    }
    Ok(at)
}
