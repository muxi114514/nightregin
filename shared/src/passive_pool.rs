//! 初始武器的随机通用词条：按武器类别选普通掉落用的 501 系列效力 1 抽奖表，再按当前角色过滤后加权随机。
//! 类别 -> 表族没有直接字段，按 EquipParamCustomWeapon 中同类别武器第一栏最常用的 501 表族推出
//! （如盾 -> 5013、法杖 -> 5014、圣印记 -> 5015、弓 -> 5016）。

use std::collections::HashMap;

use crate::param::layout::{attach_effect, attach_effect_table, custom_weapon};
use crate::param::{ParamFile, field};
use crate::weapons::WeaponInfo;

/// 通用词条表：501FF00TT，FF 为表族，TT 为档位（00 / 100 / 200 对应效力 1 / 2 / 3）
const GENERAL_TABLES: std::ops::Range<u32> = 501_000_000..502_000_000;
const FAMILY_STEP: u32 = 100_000;

#[derive(Debug, Default, Clone)]
pub struct PassivePool {
    /// 效力 1 的通用表 -> [(词条, 权重)]
    tables: HashMap<u32, Vec<(u32, u32)>>,
    /// wepType -> 效力 1 的通用表
    table_by_type: HashMap<u16, u32>,
    /// 词条 -> 可出现的角色位（第 heroId-1 位）
    allow: HashMap<u32, u16>,
}

impl PassivePool {
    pub fn build(attach: &ParamFile, tables: &ParamFile, custom: &ParamFile, weapons: &WeaponInfo) -> Self {
        let allow = attach
            .rows()
            .filter_map(|r| {
                let lo = field::u8(r.data, attach_effect::ALLOW_HEROES)? as u16;
                let hi = field::u8(r.data, attach_effect::ALLOW_HEROES + 1)? as u16 & 0x3;
                Some((r.id, lo | hi << 8))
            })
            .collect();
        let entries = tables.rows().filter_map(|r| {
            let id = field::i32(r.data, attach_effect_table::ATTACH_EFFECT_ID)?;
            let weight = field::u16(r.data, attach_effect_table::CHANCE_WEIGHT)?;
            Some((r.id, u32::try_from(id).ok()?, weight as u32))
        });
        let custom_rows = custom.rows().filter_map(|r| {
            let target = u32::try_from(field::i32(r.data, custom_weapon::TARGET_WEAPON_ID)?).ok()?;
            let table = u32::try_from(field::i32(r.data, custom_weapon::ATTACH_TABLE_1)?).ok()?;
            Some((weapons.wep_type(target)?, table))
        });
        Self::from_parts(entries, custom_rows, allow)
    }

    /// entries：(表 ID, 词条, 权重)；custom：(wepType, 第一栏表 ID)
    pub fn from_parts(
        entries: impl IntoIterator<Item = (u32, u32, u32)>,
        custom: impl IntoIterator<Item = (u16, u32)>,
        allow: HashMap<u32, u16>,
    ) -> Self {
        let mut tables: HashMap<u32, Vec<(u32, u32)>> = HashMap::new();
        for (table, id, weight) in entries {
            if GENERAL_TABLES.contains(&table) && table % FAMILY_STEP == 0 && weight > 0 {
                tables.entry(table).or_default().push((id, weight));
            }
        }
        let mut votes: HashMap<u16, HashMap<u32, u32>> = HashMap::new();
        for (wep_type, table) in custom {
            if GENERAL_TABLES.contains(&table) {
                *votes.entry(wep_type).or_default().entry(table - table % FAMILY_STEP).or_default() += 1;
            }
        }
        let table_by_type = votes
            .into_iter()
            .filter_map(|(t, v)| v.into_iter().max_by_key(|&(table, n)| (n, std::cmp::Reverse(table))).map(|(table, _)| (t, table)))
            .filter(|(_, table)| tables.contains_key(table))
            .collect();
        Self { tables, table_by_type, allow }
    }

    /// 武器对应的效力 1 通用表
    pub fn table_for(&self, weapon_id: u32, weapons: &WeaponInfo) -> Option<u32> {
        self.table_by_type.get(&weapons.wep_type(weapon_id)?).copied()
    }

    /// 按权重抽一条该角色可用的词条；random 为任意随机数
    pub fn roll(&self, weapon_id: u32, weapons: &WeaponInfo, hero_id: u32, random: u64) -> Option<u32> {
        let table = self.tables.get(&self.table_for(weapon_id, weapons)?)?;
        let bit = 1u16 << hero_id.checked_sub(1).filter(|&b| b < 10)?;
        let allowed: Vec<(u32, u32)> = table.iter().copied().filter(|(id, _)| self.allow.get(id).is_some_and(|m| m & bit != 0)).collect();
        let total: u64 = allowed.iter().map(|&(_, w)| w as u64).sum();
        if total == 0 {
            return None;
        }
        let mut pick = random % total;
        allowed.into_iter().find(|&(_, w)| {
            let hit = pick < w as u64;
            pick = pick.saturating_sub(w as u64);
            hit
        })
        .map(|(id, _)| id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> (PassivePool, WeaponInfo) {
        let weapons = WeaponInfo::from_rows([(1000000, 1, 0), (30000000, 65, 0)]);
        let entries = [
            (501000000, 8100100, 100),
            (501000000, 8100200, 300),
            (501000000, 8100101, 0),
            (501000100, 8100101, 100),
            (501300000, 8220000, 100),
        ];
        let custom = [(1, 501000000), (1, 501000100), (1, 501200000), (65, 501300000), (65, 808110400)];
        let all = 0x3FF;
        let allow = HashMap::from([(8100100, all), (8100200, 1 << 7), (8100101, all), (8220000, all)]);
        (PassivePool::from_parts(entries, custom, allow), weapons)
    }

    #[test]
    fn picks_family_table_by_majority() {
        let (pool, w) = sample();
        assert_eq!(pool.table_for(1000000, &w), Some(501000000));
        assert_eq!(pool.table_for(1000002, &w), Some(501000000));
        assert_eq!(pool.table_for(30000000, &w), Some(501300000));
    }

    #[test]
    fn rolls_by_weight_and_hero() {
        let (pool, w) = sample();
        // 执行者（heroId 8）两条都能抽：权重 100 / 300
        assert_eq!(pool.roll(1000000, &w, 8, 0), Some(8100100));
        assert_eq!(pool.roll(1000000, &w, 8, 99), Some(8100100));
        assert_eq!(pool.roll(1000000, &w, 8, 100), Some(8100200));
        assert_eq!(pool.roll(1000000, &w, 8, 399), Some(8100200));
        // 追踪者只能抽第一条
        assert_eq!(pool.roll(1000000, &w, 1, 399), Some(8100100));
        assert_eq!(pool.roll(1000000, &w, 11, 0), None);
        assert_eq!(pool.roll(4000000, &w, 1, 0), None);
    }
}
