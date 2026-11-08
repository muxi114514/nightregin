//! 三档锻体里靠 DLL 改内存参数表的词条（同 tiered_items：regulation 里只放标记，DLL 动手）。
//! 持有标记时把下列字段写成新值，回到圆桌还原；参数表只在本机生效，不影响队友。出击中只改不还（标记偶尔不在列表里时不来回切），
//! 可叠加的锻体按身上的份数算（每拿一份，效果列表里多一个同号标记）。
//! - 送葬者4：恍惚持续 8 → 12 秒、强化恍惚 15 → 22.5 秒
//! - 学者1：道具升级要的经验全改 0，进道具栏就是满级
//! - 学者3：观察第三级挂给敌人的易伤换成锻体版 SCHOLAR_WEAKNESS（被打后转成 3 秒不消失的易伤，行在 relics 的 tiered::params）
//! - 学者4：观察速度 ×2（用户实测 ×1.5 有效后要求提到 +100%）
//! - 复仇者1：死灵术发动概率 35% → 100%
//! - 复仇者3：家人倒下后等待复活时的回血（每 0.6 / 1.5 / 2 秒回 1%）翻倍，复活等待减半
//! - 守护者1：延长版旋风（遗物版，蓄力 7 秒 / 不蓄力 4 秒，含挡魔法的那颗与延长版冰风暴）寿命翻倍。锻体本身同遗物一样换成延长版，
//!   只靠这一点和遗物一起带时毫无区别，所以再翻倍
//! - 守护者2：技艺旋风（含遗物版与冰风暴）的攻击行加上「战技」分类 112，吃战技攻击力加成
//! - 守护者5：持续释放绝招时每 0.2 秒扣的量表 1 → 0.5
//! - 蓝2 / 蓝8（可叠加）：护符栏每份 +1（上限 4）、道具栏每份 +1（上限 8）
//! - 蓝14（可叠加）：黑夜雨的普通伤害阶段（原版 110 秒，之后 20 秒加重再即死）每份 +60 秒

use crate::param::layout::{Table, atk, bullet, player_common, sp_effect};
use crate::tiered_cold_mirage::{GOLD_MIRAGE, MIRAGE_READY, MIRAGE_USED};
use crate::tiered_guardian::{GUARDIAN_LONG_SKILL, ICE_LONG_BODIES};

pub const UNDERTAKER_LONG_TRANCE: i32 = 9_971_800;
pub const SCHOLAR_LASTING_WEAKNESS: i32 = 9_971_805;
/// 学者3 的易伤：同原版 708020（10 秒，被打即结束），结束时换成 9971807（3 秒、被打不消失）
pub const SCHOLAR_WEAKNESS: i32 = 9_971_806;
pub const SCHOLAR_FAST_ANALYSIS: i32 = 9_971_810;
pub const REVENANT_SURE_NECROMANCY: i32 = 9_971_815;
pub const REVENANT_FAST_REVIVE: i32 = 9_971_820;
pub const GUARDIAN_SKILL_AS_ART: i32 = 9_971_825;
pub const GUARDIAN_CHEAP_HOLD: i32 = 9_971_830;
pub const SCHOLAR_MAX_LEVEL: i32 = 9_971_845;
pub const TALISMAN_SLOT: i32 = 9_971_040;
pub const ITEM_SLOT: i32 = 9_971_050;
pub const LONG_NIGHT_TIDE: i32 = 9_971_060;
/// 执行者8：不改参数表，DLL 按角色身上的这个标记选动作（src\crucible_sword.rs）
pub const EXECUTOR_CRUCIBLE: i32 = 9_971_865;

/// 守护者冰风暴（守护者7）的攻击行，同样算技艺
pub const GUARDIAN_ICE_ATK: u32 = 900_997_119;
/// 「战技」攻击分类
const SKILL_ART_CATEGORY: u8 = 112;
const MAX_TALISMAN_SLOTS: u8 = 4;
const MAX_ITEM_SLOTS: u8 = 8;
const NIGHT_TIDE_EXTRA_SECONDS: f32 = 60.0;

/// 复仇者三个家人倒下后等待复活时的效果（海伦 / 弗雷德里克 / 塞巴斯蒂安）
const FAMILY_INACTIVE: [u32; 3] = [705_061, 705_071, 705_081];
/// 守护者持续释放绝招时每 0.2 秒扣量表的效果
const ULT_HOLD_DRAIN: [u32; 1] = [701_515];
/// 遗物延长版旋风：蓄力（伤害 + 发射器、挡魔法）、不蓄力（同），加上延长版冰风暴
const LONG_WHIRLWIND_BULLETS: [u32; 6] = [30_901_111, 30_901_113, 30_901_116, 30_901_118, ICE_LONG_BODIES[0], ICE_LONG_BODIES[1]];
/// 守护者技艺旋风各版本（普通 / 蓄力、遗物延长、遗物衍生）的攻击行，原版分类 122 技艺、第二分类为空
const WHIRLWIND_ATKS: [u32; 14] = [
    600_000_050, 600_000_055, 600_000_080, 600_000_081, 600_000_082, 600_000_083, 600_000_084, 600_000_085, 600_000_086,
    600_000_087, 600_000_089, 600_000_090, 600_000_091, GUARDIAN_ICE_ATK,
];
/// 黑夜雨普通伤害阶段（第一 / 第二次缩圈），到时换成加重伤害
const NIGHT_TIDE_DAMAGE: [u32; 2] = [99_810, 99_811];
const COMMON_ROW: [u32; 1] = [player_common::ROW];

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Change {
    /// f32 乘以倍数
    ScaleF32(f32),
    /// s32 乘以倍数（四舍五入）
    ScaleI32(f32),
    SetF32(f32),
    SetI32(i32),
    SetU8(u8),
    /// u8 每份加 per，不超过 max（原值已超过就不动）
    AddU8 { per: u8, max: u8 },
    /// f32 每份加这么多
    AddF32(f32),
    /// 连续若干字节清零
    Zero(usize),
}

impl Change {
    /// 字段字节数
    pub fn width(self) -> usize {
        match self {
            Change::SetU8(_) | Change::AddU8 { .. } => 1,
            Change::Zero(len) => len,
            _ => 4,
        }
    }

    /// 原值（小端）按身上的份数改成新值；原值长度不对时为 None。不叠加的改法与份数无关
    pub fn apply(self, original: &[u8], stacks: u32) -> Option<Vec<u8>> {
        if original.len() != self.width() {
            return None;
        }
        let word = || [original[0], original[1], original[2], original[3]];
        Some(match self {
            Change::ScaleF32(k) => (f32::from_le_bytes(word()) * k).to_le_bytes().to_vec(),
            Change::ScaleI32(k) => ((i32::from_le_bytes(word()) as f32 * k).round() as i32).to_le_bytes().to_vec(),
            Change::SetF32(v) => v.to_le_bytes().to_vec(),
            Change::SetI32(v) => v.to_le_bytes().to_vec(),
            Change::SetU8(v) => vec![v],
            Change::AddU8 { per, max } => {
                let added = u32::from(original[0]) + u32::from(per) * stacks;
                vec![added.min(u32::from(max.max(original[0]))) as u8]
            }
            Change::AddF32(per) => (f32::from_le_bytes(word()) + per * stacks as f32).to_le_bytes().to_vec(),
            Change::Zero(len) => vec![0; len],
        })
    }
}

/// 持有 marker 时把 table 里 rows 各行 offset 处的字段按 change 改写
pub struct Override {
    pub marker: i32,
    pub table: Table,
    pub rows: &'static [u32],
    pub offset: usize,
    pub change: Change,
}

const fn common(marker: i32, offset: usize, change: Change) -> Override {
    Override { marker, table: player_common::TABLE, rows: &COMMON_ROW, offset, change }
}

pub const OVERRIDES: &[Override] = &[
    common(UNDERTAKER_LONG_TRANCE, player_common::TRANCE_DURATION, Change::ScaleF32(1.5)),
    common(UNDERTAKER_LONG_TRANCE, player_common::TRANCE_ENHANCE_DURATION, Change::ScaleF32(1.5)),
    common(SCHOLAR_MAX_LEVEL, player_common::BAGCRAFT_XP, Change::Zero(player_common::BAGCRAFT_XP_LEN)),
    common(SCHOLAR_LASTING_WEAKNESS, player_common::ANALYSE_TARGET_LEVEL3_EFFECT, Change::SetI32(SCHOLAR_WEAKNESS)),
    common(SCHOLAR_FAST_ANALYSIS, player_common::ANALYSE_SPEED, Change::ScaleI32(2.0)),
    common(REVENANT_SURE_NECROMANCY, player_common::NECROMANCY_CHANCE, Change::SetF32(100.0)),
    common(TALISMAN_SLOT, player_common::ACC_SLOT_NUM, Change::AddU8 { per: 1, max: MAX_TALISMAN_SLOTS }),
    common(ITEM_SLOT, player_common::ITEM_SLOTS, Change::AddU8 { per: 1, max: MAX_ITEM_SLOTS }),
    common(GOLD_MIRAGE, player_common::COLD_MIRAGE_READY_EFFECT, Change::SetI32(MIRAGE_READY)),
    common(GOLD_MIRAGE, player_common::COLD_MIRAGE_USED_EFFECT, Change::SetI32(MIRAGE_USED)),
    Override { marker: REVENANT_FAST_REVIVE, table: sp_effect::TABLE, rows: &FAMILY_INACTIVE, offset: sp_effect::CHANGE_HP_RATE, change: Change::ScaleF32(2.0) },
    Override { marker: GUARDIAN_LONG_SKILL, table: bullet::TABLE, rows: &LONG_WHIRLWIND_BULLETS, offset: bullet::LIFE, change: Change::ScaleF32(2.0) },
    Override { marker: GUARDIAN_SKILL_AS_ART, table: atk::TABLE, rows: &WHIRLWIND_ATKS, offset: atk::SUB_CATEGORY_2, change: Change::SetU8(SKILL_ART_CATEGORY) },
    Override { marker: GUARDIAN_CHEAP_HOLD, table: sp_effect::TABLE, rows: &ULT_HOLD_DRAIN, offset: sp_effect::ULTIMATE_ART_GAUGE, change: Change::ScaleF32(0.5) },
    Override {
        marker: LONG_NIGHT_TIDE,
        table: sp_effect::TABLE,
        rows: &NIGHT_TIDE_DAMAGE,
        offset: sp_effect::EFFECT_ENDURANCE,
        change: Change::AddF32(NIGHT_TIDE_EXTRA_SECONDS),
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changes_keep_width() {
        assert_eq!(Change::ScaleF32(1.5).apply(&8.0f32.to_le_bytes(), 1), Some(12.0f32.to_le_bytes().to_vec()));
        assert_eq!(Change::ScaleI32(1.5).apply(&100i32.to_le_bytes(), 1), Some(150i32.to_le_bytes().to_vec()));
        assert_eq!(Change::ScaleF32(2.0).apply(&(-1.0f32).to_le_bytes(), 3), Some((-2.0f32).to_le_bytes().to_vec()));
        assert_eq!(Change::SetU8(112).apply(&[0], 1), Some(vec![112]));
        assert_eq!(Change::SetU8(112).apply(&[0, 0, 0, 0], 1), None);
        assert_eq!(Change::Zero(4).apply(&[1, 2, 3, 4], 1), Some(vec![0; 4]));
    }

    #[test]
    fn stacking_changes_grow_with_stacks_and_cap() {
        let slots = Change::AddU8 { per: 1, max: 4 };
        assert_eq!(slots.apply(&[2], 1), Some(vec![3]));
        assert_eq!(slots.apply(&[2], 5), Some(vec![4]));
        // 原值已超过上限（别的模组改过）时不降
        assert_eq!(slots.apply(&[6], 1), Some(vec![6]));
        assert_eq!(Change::AddF32(60.0).apply(&110.0f32.to_le_bytes(), 2), Some(230.0f32.to_le_bytes().to_vec()));
    }

    #[test]
    fn fields_fit_rows() {
        for o in OVERRIDES {
            assert!(o.offset + o.change.width() <= o.table.row_size, "{} @{}", o.marker, o.offset);
            assert!(!o.rows.is_empty());
        }
    }
}
