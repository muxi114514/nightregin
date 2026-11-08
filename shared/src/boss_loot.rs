//! 首领掉落球的第三个选项（ItemLotParam_enemy）：原版为局内词条表（类别 7）或徘徊赐福（道具 700），权重合计 1000。
//! 在空条目里加入锻造石【３】，权重按该行合计的百分比从局内词条按比例扣除（余数从最大的那张扣），徘徊赐福不变。
//! 已含锻造石【３】的行跳过，重复执行不会叠加。

use crate::param::layout::item_lot::{CATEGORY, ENTRIES, ITEM_ID, NUM, WEIGHT};
use crate::param::{ParamFile, field};

pub const WENDING_GRACE: i32 = 700;
pub const SMITHING_STONE_3: i32 = 10102;
const GOODS: i32 = 1;
const ITEM_TABLE: i32 = 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Entry {
    pub id: i32,
    pub category: i32,
    pub weight: u16,
}

/// 一处改写，偏移相对表开头
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LotEdit {
    I32(usize, i32),
    U16(usize, u16),
    U8(usize, u8),
}

pub fn entries(row: &[u8]) -> Option<[Entry; ENTRIES]> {
    let mut out = [Entry::default(); ENTRIES];
    for (i, e) in out.iter_mut().enumerate() {
        *e = Entry {
            id: field::i32(row, ITEM_ID + i * 4)?,
            category: field::i32(row, CATEGORY + i * 4)?,
            weight: field::u16(row, WEIGHT + i * 2)?,
        };
    }
    Some(out)
}

/// 单行加入锻造石【３】：返回新条目；不是第三选项行、已加过或没有空位时为 None
pub fn add_stone(entries: &[Entry; ENTRIES], percent: u16) -> Option<[Entry; ENTRIES]> {
    let has = |category: i32, id: i32| entries.iter().any(|e| e.category == category && e.id == id);
    if percent == 0 || !has(GOODS, WENDING_GRACE) || has(GOODS, SMITHING_STONE_3) {
        return None;
    }
    let tables: Vec<usize> = (0..ENTRIES).filter(|&i| entries[i].category == ITEM_TABLE && entries[i].weight > 0).collect();
    let free = entries.iter().position(|e| e.id <= 0 && e.weight == 0)?;
    let pool: u32 = tables.iter().map(|&i| entries[i].weight as u32).sum();
    let total: u32 = entries.iter().map(|e| e.weight as u32).sum();
    // 每张局内词条表至少留 1
    let take = (total * percent as u32 / 100).min(pool.saturating_sub(tables.len() as u32));
    let largest = *tables.iter().max_by_key(|&&i| entries[i].weight)?;
    let mut out = *entries;
    let mut taken = 0;
    for &i in &tables {
        let d = take * entries[i].weight as u32 / pool;
        out[i].weight -= d as u16;
        taken += d;
    }
    out[largest].weight -= (take - taken) as u16;
    out[free] = Entry { id: SMITHING_STONE_3, category: GOODS, weight: take as u16 };
    (take > 0).then_some(out)
}

/// 整张表的改写：(行 ID, 改写列表)
pub fn plan(table: &ParamFile, percent: u16) -> Vec<(u32, Vec<LotEdit>)> {
    table
        .rows()
        .filter_map(|r| {
            let old = entries(r.data)?;
            let new = add_stone(&old, percent)?;
            let mut edits = Vec::new();
            for i in 0..ENTRIES {
                if old[i].id != new[i].id {
                    edits.push(LotEdit::I32(r.offset + ITEM_ID + i * 4, new[i].id));
                    edits.push(LotEdit::I32(r.offset + CATEGORY + i * 4, new[i].category));
                    edits.push(LotEdit::U8(r.offset + NUM + i, 1));
                }
                if old[i].weight != new[i].weight {
                    edits.push(LotEdit::U16(r.offset + WEIGHT + i * 2, new[i].weight));
                }
            }
            Some((r.id, edits))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(items: &[(i32, i32, u16)]) -> [Entry; ENTRIES] {
        let mut out = [Entry::default(); ENTRIES];
        for (e, &(id, category, weight)) in out.iter_mut().zip(items) {
            *e = Entry { id, category, weight };
        }
        out
    }

    #[test]
    fn takes_weight_from_power_tables() {
        let new = add_stone(&row(&[(4200200, 7, 950), (700, 1, 50)]), 10).unwrap();
        assert_eq!(new[0].weight, 850);
        assert_eq!(new[1], Entry { id: 700, category: 1, weight: 50 });
        assert_eq!(new[2], Entry { id: SMITHING_STONE_3, category: 1, weight: 100 });
        // 两张表按比例扣，合计仍为 1000
        let new = add_stone(&row(&[(4200200, 7, 800), (700, 1, 50), (4200300, 7, 150)]), 10).unwrap();
        assert_eq!((new[0].weight, new[1].weight, new[2].weight, new[3].weight), (715, 50, 135, 100));
    }

    #[test]
    fn skips_other_rows_and_repeats() {
        assert!(add_stone(&row(&[(2000003, 7, 900), (2000004, 7, 100)]), 10).is_none());
        let once = add_stone(&row(&[(4200200, 7, 950), (700, 1, 50)]), 10).unwrap();
        assert!(add_stone(&once, 10).is_none());
        assert!(add_stone(&row(&[(4200200, 7, 950), (700, 1, 50)]), 0).is_none());
        // 概率过大时每张表至少留 1
        let new = add_stone(&row(&[(4200200, 7, 950), (700, 1, 50)]), 100).unwrap();
        assert_eq!((new[0].weight, new[2].weight), (1, 949));
    }
}
