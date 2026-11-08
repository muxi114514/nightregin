//! 宝箱怪的掉落球（MMV 的 ItemLotParam_enemy 6900000~6900002，所有宝箱怪共用）：
//! MMV 原本第一选项是属性转换（表 2010）、第二选项是角色锻体（表 2000）、第三选项是武器。
//! 这里改成第一选项必出本角色专属，第二选项纯紫池 / 本角色专属各半，武器不动。
//! 比例直接写条目，不走 tiered::split：紫色档的拆分会混入金和专属，这里要的是纯紫池。

use nr_stat_alloc_patcher::param_write::OwnedRow;
use nr_stat_alloc_shared::param::ParamFile;
use nr_stat_alloc_shared::param::layout::item_lot::ENTRIES;

use super::loot::{table_at, write_parts};
use super::{PURPLE, ROLE};

/// (掉落行, 条目（表, 权重）)
const OPTIONS: [(u32, &[(u32, u32)]); 2] = [(6_900_000, &[(ROLE, 1000)]), (6_900_001, &[(PURPLE, 500), (ROLE, 500)])];

/// 改写前两个选项，返回改了几行；MMV 更新后对不上（缺行或行里没有表条目）直接报错
pub fn apply(rows: &mut [OwnedRow]) -> Result<usize, String> {
    for (id, parts) in OPTIONS {
        let row = rows.iter_mut().find(|r| r.id == id).ok_or(format!("mimic lot {id} not found"))?;
        let slots: Vec<usize> = (0..ENTRIES).filter(|&i| table_at(&row.data, i).is_some()).collect();
        if slots.is_empty() {
            return Err(format!("mimic lot {id} has no table entries"));
        }
        write_parts(&mut row.data, &slots, parts).map_err(|e| format!("mimic lot {id}: {e}"))?;
    }
    Ok(OPTIONS.len())
}

/// 输出自检：两个选项正好是规定的池
pub fn verify(file: &ParamFile) -> Result<(), String> {
    for (id, parts) in OPTIONS {
        let row = file.row(id).ok_or(format!("self-check: mimic lot {id} missing"))?;
        let tables: Vec<u32> = (0..ENTRIES).filter_map(|i| table_at(row, i)).collect();
        if !tables.iter().copied().eq(parts.iter().map(|p| p.0)) {
            return Err(format!("self-check: mimic lot {id} drops {tables:?}"));
        }
    }
    Ok(())
}
