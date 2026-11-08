//! 罪人入侵（黑夜刺客、黑夜堕落者等十种，NpcParam 600030x10~x70 的 rewardItemLot_1 / _2）的掉落球：
//! 原本只有首行（本角色武器）与 +1 行（随机武器）两个选项。游戏按首行起连续存在的行出选项（首领球 +2 行就是锻体），
//! 这里在 +2 补一行锻体：紫池与该罪人对应角色的专属池（hero_role，不限拾取者角色）各半，以 +1 行为模板沿用同一个球的其余字段。

use nr_stat_alloc_patcher::param_write::OwnedRow;
use nr_stat_alloc_shared::param::layout::item_lot::{CATEGORY, ENTRIES};
use nr_stat_alloc_shared::param::{ParamFile, field};

use crate::spec::NAME_PREFIX;

use super::loot::{ENTRY_BITS, table_at, write_parts};
use super::{HEROES, PURPLE, hero_role};

/// 掉落首行 = FIRST + (heroId - 1) × HERO_STEP + 档位 × VARIANT_STEP（十种罪人按 heroId 顺序；三档只是武器稀有度权重不同）
const FIRST: u32 = 13_000_000;
const HERO_STEP: u32 = 10_000;
const VARIANT_STEP: u32 = 10;
const VARIANTS: u32 = 3;
const BUFF_OFFSET: u32 = 2;
/// 紫池 : 罪人角色专属
const PURPLE_WEIGHT: u32 = 500;
const HERO_WEIGHT: u32 = 500;

/// (掉落首行, 罪人对应的 heroId)
fn bases() -> impl Iterator<Item = (u32, u8)> {
    HEROES.flat_map(|h| (0..VARIANTS).map(move |v| (FIRST + (h as u32 - 1) * HERO_STEP + v * VARIANT_STEP, h)))
}

fn parts(hero: u8) -> [(u32, u32); 2] {
    [(PURPLE, PURPLE_WEIGHT), (hero_role(hero), HERO_WEIGHT)]
}

/// 每个入侵掉落补上锻体选项行，返回补了几行；MMV 更新后对不上（缺首行 / +1 行，或 +2 已被别人占用）直接报错
pub fn add_options(rows: &mut Vec<OwnedRow>) -> Result<usize, String> {
    let mut added = 0;
    for (base, hero) in bases() {
        let id = base + BUFF_OFFSET;
        if let Some(i) = rows.iter().position(|r| r.id == id) {
            if !rows[i].name.as_deref().is_some_and(|n| n.starts_with(NAME_PREFIX)) {
                return Err(format!("invader lot {id} already exists"));
            }
            rows.remove(i);
        }
        if !rows.iter().any(|r| r.id == base) {
            return Err(format!("invader lot {base} not found"));
        }
        let second = rows.iter().position(|r| r.id == base + 1).ok_or(format!("invader lot {} not found", base + 1))?;
        let mut data = rows[second].data.clone();
        let used: Vec<usize> = (0..ENTRIES).filter(|&i| field::i32(&data, CATEGORY + i * 4).unwrap_or(0) != 0).collect();
        if used.is_empty() {
            return Err(format!("invader lot {} has no entries", base + 1));
        }
        write_parts(&mut data, &used, &parts(hero)).map_err(|e| format!("invader lot {id}: {e}"))?;
        // 锻体条目不受观察力影响、不重置累积（同首领锻体行）
        data[ENTRY_BITS..ENTRY_BITS + 2].fill(0);
        rows.insert(second + 1, OwnedRow { id, data, name: Some(format!("{NAME_PREFIX}Invader Buff {hero}")) });
        added += 1;
    }
    Ok(added)
}

/// 输出自检：每个入侵掉落的 +2 行都在，且正好是紫池与该罪人角色的专属池
pub fn verify(file: &ParamFile) -> Result<(), String> {
    for (base, hero) in bases() {
        let id = base + BUFF_OFFSET;
        let row = file.row(id).ok_or(format!("self-check: invader buff lot {id} missing"))?;
        let tables: Vec<u32> = (0..ENTRIES).filter_map(|i| table_at(row, i)).collect();
        if tables != parts(hero).map(|p| p.0) {
            return Err(format!("self-check: invader buff lot {id} drops {tables:?}"));
        }
    }
    Ok(())
}
