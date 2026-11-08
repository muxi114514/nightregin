//! 首领球的锻体候选池（ItemTableParam）：同一表 ID 在文件里占连续多行，每行一个候选（类别 1 + 锻体 ID + 权重），
//! 游戏按表 ID 找到第一行后顺序读完同 ID 的行，所以只能按行增删，并保持每组连续。
//! 把候选从若干池移到另一个池：从来源池删掉该候选的行，在目标池最后一行之后插一行（复制目标池第一行再换 ID，权重与其余字段同池内其他候选）；
//! 没有目标池就只删除。已处理过的不重复处理，重复运行结果不变。

use nr_stat_alloc_patcher::param_write::OwnedRow;
use nr_stat_alloc_shared::param::{ParamFile, field};

use crate::layout::item_table::{CATEGORY, ITEM_ID};
use crate::spec::NAME_PREFIX;

pub const BUFF_CATEGORY: i32 = 1;

pub struct PoolMove {
    /// 锻体（PermanentBuffParam）ID
    pub buff: u32,
    pub from: &'static [u32],
    /// None：只从来源池删除
    pub to: Option<u32>,
}

#[derive(Default)]
pub struct Report {
    pub removed: usize,
    pub added: usize,
}

/// 在 ItemTableParam 的全部行上改（调用方写出），便于与 tiered::pools 接力
pub fn apply(rows: &mut Vec<OwnedRow>, moves: &[PoolMove], report: &mut Report) -> Result<(), String> {
    for m in moves {
        let before = rows.len();
        rows.retain(|r| !(m.from.contains(&r.id) && is_buff(&r.data, m.buff)));
        report.removed += before - rows.len();
        let Some(to) = m.to else { continue };
        if rows.iter().any(|r| r.id == to && is_buff(&r.data, m.buff)) {
            continue;
        }
        let first = rows.iter().position(|r| r.id == to).ok_or(format!("pool {to} not found"))?;
        let last = rows.iter().rposition(|r| r.id == to).unwrap_or(first);
        let mut data = rows[first].data.clone();
        data[ITEM_ID..ITEM_ID + 4].copy_from_slice(&(m.buff as i32).to_le_bytes());
        let name = Some(format!("{NAME_PREFIX}Permanent buff {}", m.buff));
        rows.insert(last + 1, OwnedRow { id: to, data, name });
        report.added += 1;
    }
    for m in moves {
        for id in m.from.iter().copied().chain(m.to) {
            check_contiguous(rows, id)?;
        }
    }
    Ok(())
}

/// 输出自检：来源池没有该候选，有目标池时目标池有
pub fn verify(file: &ParamFile, moves: &[PoolMove]) -> Result<(), String> {
    for m in moves {
        let has = |pool: u32| file.rows().any(|r| r.id == pool && is_buff(r.data, m.buff));
        if m.to.is_some_and(|to| !has(to)) || m.from.iter().any(|&p| has(p)) {
            return Err(format!("self-check: permanent buff {} not moved from {:?} to {:?}", m.buff, m.from, m.to));
        }
    }
    Ok(())
}

fn is_buff(row: &[u8], buff: u32) -> bool {
    field::i32(row, CATEGORY) == Some(BUFF_CATEGORY) && field::i32(row, ITEM_ID) == Some(buff as i32)
}

/// 同一池的行必须连续，否则游戏只读得到第一段
pub fn check_contiguous(rows: &[OwnedRow], id: u32) -> Result<(), String> {
    let first = rows.iter().position(|r| r.id == id);
    let last = rows.iter().rposition(|r| r.id == id);
    match (first, last) {
        (Some(a), Some(b)) if rows[a..=b].iter().any(|r| r.id != id) => Err(format!("pool {id} rows are not contiguous")),
        _ => Ok(()),
    }
}
