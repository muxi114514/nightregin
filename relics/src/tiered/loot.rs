//! 掉落行（ItemLotParam_enemy / _map）改指新锻体池：每行 8 个条目，类别 7 的条目引用掉落表。
//! 1. 按旧池累计该行各档的权重（原版一级 / 二级混放的行自然变成蓝紫混合，如红皮野外首领 90 / 10）
//! 2. 必出金：全部换成金；指定的行与按敌人参数判断的深夜模式红皮（蓝 90%）、入侵据点（蓝 50%）按比例重分蓝紫，
//!    后两者只改原本只出蓝的行，同一掉落首行也被某个敌人当普通掉落引用时不改
//! 3. 各档按 tiered::split 拆成条目写回：先用原锻体条目的位置，不够再占空条目并照抄原条目的其余字段（累积、旗标、数量等）
//! 4. 罪人入侵的掉落球补第三个锻体选项（invader.rs），宝箱怪前两个选项另定（mimic.rs）

use std::collections::HashSet;

use nr_stat_alloc_patcher::param_write;
use nr_stat_alloc_patcher::regulation::Regulation;
use nr_stat_alloc_shared::param::layout::item_lot::{CATEGORY, ENTRIES, ITEM_ID, NUM, WEIGHT};
use nr_stat_alloc_shared::param::{ParamFile, field};

use crate::insert;
use crate::layout::{ITEM_LOT_ENEMY, ITEM_LOT_MAP, NPC, Table, npc};

use super::{GOLD, RETIRED, TABLES, Tier, invader, mimic, split, tier_of};

const TABLE_CATEGORY: i32 = 7;

/// 必出金：3 万卢恩古老监牢第二选项（含红皮）、火山熔岩土龙（含红皮）、山顶冰龙、隐城艾丝提第三选项
const GOLD_LOTS: &[u32] = &[6_430_001, 6_430_011, 6_122_102, 6_122_112, 6_121_002, 6_124_002];
/// 指定的蓝紫混合（蓝的千分比）：2 万卢恩监牢第二选项 90%，野外二档首领第三选项 70%
const FIXED_MIX: &[(u32, u32)] = &[(6_420_001, 900), (6_420_011, 900), (6_100_102, 700), (6_100_112, 700)];
const VARIANT_BLUE: u32 = 900;
const CONDEMNED_BLUE: u32 = 500;
/// 掉落首行之后的第二、三选项
const OPTION_OFFSETS: [u32; 2] = [1, 2];

// 每个条目的其余字段（占用空条目时照抄）
const CUMULATE_POINT: usize = 80;
const FLAG_ID: usize = 96;
const ACTION_A: usize = 152;
const ACTION_B: usize = 184;
/// u16：低 8 位 enableLuck、高 8 位 cumulateReset，每个条目一位
pub(super) const ENTRY_BITS: usize = 146;

#[derive(Default)]
pub struct Report {
    pub rows: usize,
    pub gold: usize,
    pub fixed: usize,
    pub variant: usize,
    pub condemned: usize,
    pub invader: usize,
    pub mimic: usize,
}

/// 返回两张掉落表的新内容
pub fn apply(reg: &Regulation, report: &mut Report) -> Result<Vec<(&'static str, Vec<u8>)>, String> {
    let bases = LotBases::from(&insert::open(reg, &NPC)?);
    let mut out = Vec::new();
    for table in [&ITEM_LOT_ENEMY, &ITEM_LOT_MAP] {
        out.push((table.file, rewrite(reg, table, &bases, report).map_err(|e| format!("{}: {e}", table.file))?));
    }
    Ok(out)
}

/// 输出自检：不再引用旧池，必出金的行只有金色锻体与传说武器，入侵掉落都有锻体选项，宝箱怪选项正确
pub fn verify(reg: &Regulation) -> Result<(), String> {
    let enemy = insert::open(reg, &ITEM_LOT_ENEMY)?;
    invader::verify(&enemy)?;
    mimic::verify(&enemy)?;
    for table in [&ITEM_LOT_ENEMY, &ITEM_LOT_MAP] {
        let file = insert::open(reg, table)?;
        for r in file.rows() {
            let tables: Vec<u32> = (0..ENTRIES).filter_map(|i| table_at(r.data, i)).collect();
            if tables.iter().any(|&t| tier_of(t).is_some() || RETIRED.contains(&t)) {
                return Err(format!("self-check: {} {} still uses an old pool", table.file, r.id));
            }
            if GOLD_LOTS.contains(&r.id) && tables.iter().any(|&t| TABLES.contains(&t) && t != GOLD) {
                return Err(format!("self-check: {} {} should only drop gold", table.file, r.id));
            }
        }
    }
    Ok(())
}

/// 敌人参数里各种掉落的首行：深夜模式与入侵据点只算没有被当作普通掉落的
struct LotBases {
    variant: HashSet<u32>,
    condemned: HashSet<u32>,
}

impl LotBases {
    fn from(npcs: &ParamFile) -> LotBases {
        let collect = |offset: usize| -> HashSet<u32> {
            npcs.rows().filter_map(|r| field::i32(r.data, offset)).filter(|&v| v > 0).map(|v| v as u32).collect()
        };
        let normal = collect(npc::REWARD_LOT);
        let variant: HashSet<u32> = collect(npc::CHAOS_REWARD_LOT).difference(&normal).copied().collect();
        let condemned = collect(npc::CONDEMNED_REWARD_LOT).into_iter().filter(|b| !normal.contains(b) && !variant.contains(b)).collect();
        LotBases { variant, condemned }
    }

    /// 该行属于深夜红皮 / 入侵据点掉落的第二、三选项时返回（蓝的千分比，是否红皮）
    fn blue_share(&self, lot: u32) -> Option<(u32, bool)> {
        let base = |set: &HashSet<u32>| OPTION_OFFSETS.iter().any(|&k| lot.checked_sub(k).is_some_and(|b| set.contains(&b)));
        if base(&self.variant) {
            Some((VARIANT_BLUE, true))
        } else if base(&self.condemned) {
            Some((CONDEMNED_BLUE, false))
        } else {
            None
        }
    }
}

/// 一行里各档的权重
#[derive(Default)]
struct Shares {
    blue: u32,
    purple: u32,
    role: u32,
    gold: u32,
}

impl Shares {
    fn total(&self) -> u32 {
        self.blue + self.purple + self.role + self.gold
    }

    /// 只保留蓝紫，按蓝的千分比重分
    fn mixed(total: u32, blue_permille: u32) -> Shares {
        let blue = total * blue_permille / 1000;
        Shares { blue, purple: total - blue, ..Shares::default() }
    }
}

fn rewrite(reg: &Regulation, table: &Table, bases: &LotBases, report: &mut Report) -> Result<Vec<u8>, String> {
    let file = insert::open(reg, table)?;
    let mut rows = param_write::rows_of(&file);
    for row in rows.iter_mut() {
        let data = &mut row.data;
        let tiered: Vec<(usize, Tier)> = (0..ENTRIES).filter_map(|i| table_at(data, i).and_then(tier_of).map(|t| (i, t))).collect();
        if tiered.is_empty() {
            continue;
        }
        let mut shares = Shares::default();
        for &(i, tier) in &tiered {
            let w = weight_at(data, i);
            match tier {
                Tier::Blue => shares.blue += w,
                Tier::Purple => shares.purple += w,
                Tier::Role => shares.role += w,
                Tier::Gold => shares.gold += w,
            }
        }
        if GOLD_LOTS.contains(&row.id) {
            shares = Shares { gold: shares.total(), ..Shares::default() };
            report.gold += 1;
        } else if let Some(&(_, blue)) = FIXED_MIX.iter().find(|(id, _)| *id == row.id) {
            shares = Shares::mixed(shares.total(), blue);
            report.fixed += 1;
        } else if let Some((blue, variant)) = bases.blue_share(row.id).filter(|_| shares.blue == shares.total()) {
            shares = Shares::mixed(shares.total(), blue);
            if variant { report.variant += 1 } else { report.condemned += 1 }
        }
        let mut parts: Vec<(u32, u32)> = Vec::new();
        for (tier, w) in [(Tier::Blue, shares.blue), (Tier::Purple, shares.purple), (Tier::Role, shares.role), (Tier::Gold, shares.gold)] {
            for (t, pw) in split(tier, w) {
                match parts.iter_mut().find(|p| p.0 == t) {
                    Some(p) => p.1 += pw,
                    None => parts.push((t, pw)),
                }
            }
        }
        let slots: Vec<usize> = tiered.iter().map(|&(i, _)| i).collect();
        write_parts(data, &slots, &parts).map_err(|e| format!("lot {}: {e}", row.id))?;
        report.rows += 1;
    }
    if std::ptr::eq(table, &ITEM_LOT_ENEMY) {
        report.invader = invader::add_options(&mut rows)?;
        report.mimic = mimic::apply(&mut rows)?;
    }
    param_write::write_ordered(&file, &rows)
}

/// 先占原锻体条目的位置，不够再用空条目（照抄第一个锻体条目的其余字段），多出来的原条目清空
pub(super) fn write_parts(row: &mut [u8], slots: &[usize], parts: &[(u32, u32)]) -> Result<(), String> {
    let first = slots[0];
    let free: Vec<usize> =
        (0..ENTRIES).filter(|&i| !slots.contains(&i) && weight_at(row, i) == 0 && field::i32(row, ITEM_ID + i * 4).unwrap_or(0) <= 0).collect();
    let mut free = free.into_iter();
    for (k, &(table, w)) in parts.iter().enumerate() {
        let i = match slots.get(k) {
            Some(&i) => i,
            None => {
                let i = free.next().ok_or("not enough free entries")?;
                copy_entry(row, first, i);
                i
            }
        };
        set_entry(row, i, TABLE_CATEGORY, table, w as u16);
    }
    for &i in slots.iter().skip(parts.len()) {
        set_entry(row, i, 0, 0, 0);
    }
    Ok(())
}

pub(super) fn table_at(row: &[u8], i: usize) -> Option<u32> {
    (field::i32(row, CATEGORY + i * 4) == Some(TABLE_CATEGORY)).then(|| field::i32(row, ITEM_ID + i * 4).unwrap_or(0) as u32)
}

fn weight_at(row: &[u8], i: usize) -> u32 {
    field::u16(row, WEIGHT + i * 2).unwrap_or(0) as u32
}

fn set_entry(row: &mut [u8], i: usize, category: i32, item: u32, weight: u16) {
    row[ITEM_ID + i * 4..ITEM_ID + i * 4 + 4].copy_from_slice(&(item as i32).to_le_bytes());
    row[CATEGORY + i * 4..CATEGORY + i * 4 + 4].copy_from_slice(&category.to_le_bytes());
    row[WEIGHT + i * 2..WEIGHT + i * 2 + 2].copy_from_slice(&weight.to_le_bytes());
}

/// 照抄条目 from 除物品 / 类别 / 权重以外的字段到 to
fn copy_entry(row: &mut [u8], from: usize, to: usize) {
    for (offset, size) in [(CUMULATE_POINT, 2), (FLAG_ID, 4), (NUM, 1), (ACTION_A, 4), (ACTION_B, 4)] {
        let src = offset + from * size;
        let dst = offset + to * size;
        let bytes = row[src..src + size].to_vec();
        row[dst..dst + size].copy_from_slice(&bytes);
    }
    let bits = u16::from_le_bytes([row[ENTRY_BITS], row[ENTRY_BITS + 1]]);
    let mut out = bits;
    for shift in [0, 8] {
        let on = bits >> (shift + from) & 1;
        out = (out & !(1 << (shift + to))) | (on << (shift + to));
    }
    row[ENTRY_BITS..ENTRY_BITS + 2].copy_from_slice(&out.to_le_bytes());
}
