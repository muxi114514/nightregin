//! 「血量偏低」两条紫锻体（DLL 实现，见 shared::tiered_low_hp 与 src\tiered_effects\low_hp.rs）。这里只放标记与 DLL 施加的扣血效果。
//! - 提升血量偏低时的血量判定：类别 10 可叠加，DLL 按份数改「血量偏低时」效果的判定线
//! - 血量限制在判定线：DLL 按超出的点数施加扣血效果（照原版一次性回复效果，回复改成扣 2^i 点）

use nr_stat_alloc_shared::tiered_low_hp::{HP_CAP, HP_DRAIN_COUNT, HP_DRAIN_FIRST, THRESHOLD_UP};

use crate::layout::{SP_EFFECT, sp_effect as sp};
use crate::spec::{RowSpec, Set};

use super::pools::Candidate;
use super::rows::{ICON_HP, MARKER_TEMPLATE, NO_SYNC, RARITY_PURPLE, SAVE, STACKING, buff_rows, icon, text};

const UP: u32 = THRESHOLD_UP as u32;
const CAP: u32 = HP_CAP as u32;

/// 遗物「在图腾石碑附近击杀敌人回血」的回复效果：施加时一次性回复（原带条件状态 2042，清掉）
const ONE_SHOT_TEMPLATE: u32 = 7_090_303;

/// 第 i 个扣血效果：扣 2^i 点，不带回复特效
macro_rules! drain_row {
    ($i:literal) => {
        RowSpec { table: &SP_EFFECT, template: ONE_SHOT_TEMPLATE, id: (HP_DRAIN_FIRST + $i) as u32, name: concat!("[Tiered Buff] HP held at low HP threshold - Drain 2^", $i),
            sets: &[Set::F32(sp::CHANGE_HP_RATE, 0.0), Set::I32(sp::CHANGE_HP_POINT, 1 << $i), Set::U16(sp::INVOCATION_STATE_1, 0), Set::I32(sp::VFX_ID, -1),
                Set::Bit(sp::TARGET_BYTE, sp::SELF_TARGET_BIT, true), NO_SYNC] }
    };
}

pub const ROWS: &[RowSpec] = &[
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: UP, name: "[Tiered Buff] Raised low HP threshold",
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, -1.0), Set::U16(sp::SP_CATEGORY, STACKING), SAVE, icon(ICON_HP), text(UP)] },
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: CAP, name: "[Tiered Buff] HP held at low HP threshold",
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, -1.0), SAVE, icon(ICON_HP), text(CAP)] },
    drain_row!(0),
    drain_row!(1),
    drain_row!(2),
    drain_row!(3),
    drain_row!(4),
    drain_row!(5),
    drain_row!(6),
    drain_row!(7),
    drain_row!(8),
    drain_row!(9),
    drain_row!(10),
    drain_row!(11),
];

const _: () = assert!(ROWS.len() == 2 + HP_DRAIN_COUNT, "每个扣血档位都要有效果行");

pub const BUFF_ROWS: &[RowSpec] = buff_rows![
    (UP, UP, ICON_HP, RARITY_PURPLE, "Raised low HP threshold"),
    (CAP, CAP, ICON_HP, RARITY_PURPLE, "HP held at low HP threshold"),
];

pub const PURPLE: &[Candidate] = &[Candidate::Buff(UP), Candidate::Buff(CAP)];
