//! 静默之域的诅咒表与抽取（纯数据，不碰游戏内存）。编号 1~16 由房主经大厅数据发给队友，改动要同步升 wire 的版本。

use std::collections::HashSet;

use crate::param::layout::{chaos_rank, sp_effect};

pub type CurseId = u8;
pub const NO_CURSE: CurseId = 0;

/// 敌人强度表（ChaosMatchingCorrectParam）的行范围：全部威胁分组、夜王与最终 Boss
const ALL_ENEMIES: (u32, u32) = (7700, 7781);
const NIGHT_BOSSES: (u32, u32) = (7750, 7769);
const HP_RATE: &[usize] = &[sp_effect::MAX_HP_RATE];

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CurseEffect {
    /// 给角色挂 SpEffect
    Player { sp_effect: i32 },
    /// 按倍率改敌人强度效果的字段；rows 是强度表的行范围（含两端）
    Enemy { rows: (u32, u32), fields: &'static [usize], scale: f32 },
    /// 深度表 5 行的一组权重：favored 设 100，同组其他设 0
    World { fields: [usize; 3], favored: usize },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Curse {
    pub id: CurseId,
    pub effect: CurseEffect,
}

const fn player(id: CurseId, sp_effect: i32) -> Curse {
    Curse { id, effect: CurseEffect::Player { sp_effect } }
}

const fn enemy(id: CurseId, rows: (u32, u32), fields: &'static [usize], scale: f32) -> Curse {
    Curse { id, effect: CurseEffect::Enemy { rows, fields, scale } }
}

const fn world(id: CurseId, fields: [usize; 3], favored: usize) -> Curse {
    Curse { id, effect: CurseEffect::World { fields, favored } }
}

pub const POOL: [Curse; 16] = [
    player(1, 6_820_300),
    player(2, 6_820_100),
    player(3, 6_850_500),
    player(4, 6_840_100),
    player(5, 6_840_200),
    player(6, 6_840_000),
    player(7, 6_850_700),
    player(8, 6_851_200),
    player(9, 6_850_600),
    player(10, 6_851_700),
    enemy(11, ALL_ENEMIES, HP_RATE, 1.2),
    enemy(12, ALL_ENEMIES, &sp_effect::ATTACK_RATES, 1.15),
    enemy(13, NIGHT_BOSSES, HP_RATE, 1.25),
    world(14, chaos_rank::MAP_WEIGHTS, chaos_rank::MAP_WEIGHTS[0]),
    world(15, chaos_rank::MAP_WEIGHTS, chaos_rank::MAP_WEIGHTS[1]),
    world(16, chaos_rank::CATACLYSM_WEIGHTS, chaos_rank::CATACLYSM_WEIGHTS[2]),
];

pub fn find(id: CurseId) -> Option<&'static Curse> {
    POOL.iter().find(|c| c.id == id)
}

/// 等概率抽一条；池空时为 NO_CURSE
pub fn roll(available: &[CurseId], random: u64) -> CurseId {
    if available.is_empty() {
        return NO_CURSE;
    }
    available[(random % available.len() as u64) as usize]
}

/// 诊断配置指定了可用的诅咒就用它，否则随机
pub fn pick(diag: CurseId, available: &[CurseId], random: u64) -> CurseId {
    if diag != NO_CURSE && available.contains(&diag) { diag } else { roll(available, random) }
}

/// 强度表里只被 rows 范围内的行引用的效果（升序去重）：多个分组共用的效果不能改，否则会连带别的敌人
pub fn enemy_effect_rows(table: &[(u32, [i32; 5])], rows: (u32, u32)) -> Vec<u32> {
    let in_range = |id: u32| (rows.0..=rows.1).contains(&id);
    let effects = |inside: bool| table.iter().filter(move |(id, _)| in_range(*id) == inside).flat_map(|(_, e)| e.iter().copied()).filter(|&e| e > 0);
    let shared: HashSet<i32> = effects(false).collect();
    let mut out: Vec<u32> = effects(true).filter(|e| !shared.contains(e)).map(|e| e as u32).collect();
    out.sort_unstable();
    out.dedup();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enemy_of(id: CurseId) -> ((u32, u32), &'static [usize], f32) {
        match find(id).unwrap().effect {
            CurseEffect::Enemy { rows, fields, scale } => (rows, fields, scale),
            other => panic!("{other:?}"),
        }
    }

    fn world_of(id: CurseId) -> ([usize; 3], usize) {
        match find(id).unwrap().effect {
            CurseEffect::World { fields, favored } => (fields, favored),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn pool_ids_are_one_to_sixteen() {
        let ids: Vec<CurseId> = POOL.iter().map(|c| c.id).collect();
        assert_eq!(ids, (1..=16).collect::<Vec<CurseId>>());
    }

    #[test]
    fn player_curses_match_spec() {
        let want = [6820300, 6820100, 6850500, 6840100, 6840200, 6840000, 6850700, 6851200, 6850600, 6851700];
        for (i, sp) in want.into_iter().enumerate() {
            assert_eq!(POOL[i].effect, CurseEffect::Player { sp_effect: sp }, "curse {}", i + 1);
        }
    }

    #[test]
    fn enemy_curses_match_spec() {
        assert_eq!(enemy_of(11), ((7700, 7781), &[16usize][..], 1.2));
        assert_eq!(enemy_of(12), ((7700, 7781), &[72usize, 76, 80, 84, 480][..], 1.15));
        assert_eq!(enemy_of(13), ((7750, 7769), &[16usize][..], 1.25));
    }

    #[test]
    fn world_curses_match_spec() {
        assert_eq!(world_of(14), ([2, 3, 4], 2));
        assert_eq!(world_of(15), ([2, 3, 4], 3));
        assert_eq!(world_of(16), ([5, 6, 7], 7));
    }

    #[test]
    fn find_rejects_unknown_ids() {
        assert!(find(NO_CURSE).is_none());
        assert!(find(17).is_none());
        assert_eq!(find(16).map(|c| c.id), Some(16));
    }

    #[test]
    fn roll_picks_from_available_evenly() {
        let available = [3, 9];
        assert_eq!(roll(&available, 0), 3);
        assert_eq!(roll(&available, 1), 9);
        assert_eq!(roll(&available, 2), 3);
    }

    #[test]
    fn roll_with_empty_pool_is_none() {
        assert_eq!(roll(&[], 7), NO_CURSE);
    }

    #[test]
    fn diag_override_wins_when_available() {
        assert_eq!(pick(9, &[3, 9], 0), 9);
    }

    #[test]
    fn diag_override_ignored_when_unavailable() {
        assert_eq!(pick(14, &[3, 9], 1), 9);
        assert_eq!(pick(NO_CURSE, &[3, 9], 0), 3);
    }

    fn effects(first: i32) -> [i32; 5] {
        [first, first + 1, first + 2, first + 3, first + 4]
    }

    #[test]
    fn enemy_rows_are_deduplicated() {
        let table: Vec<(u32, [i32; 5])> = (7700..=7704).map(|id| (id, effects(877001))).collect();
        assert_eq!(enemy_effect_rows(&table, (7700, 7781)), vec![877001, 877002, 877003, 877004, 877005]);
    }

    #[test]
    fn night_boss_rows_skip_shared_effects() {
        let table = [(7705, effects(877551)), (7750, effects(877501)), (7755, effects(877551))];
        assert_eq!(enemy_effect_rows(&table, (7750, 7769)), vec![877501, 877502, 877503, 877504, 877505]);
    }

    #[test]
    fn negative_effect_ids_are_skipped() {
        let table = [(7700, [-1, 0, 877001, -1, 877002])];
        assert_eq!(enemy_effect_rows(&table, (7700, 7781)), vec![877001, 877002]);
    }
}
