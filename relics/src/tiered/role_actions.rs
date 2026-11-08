//! 第七批（动作脚本改判定）：由 tools\tiered_roles.lua 与 tiered_hks.ps1 改动的原版判定生效的角色锻体。改 ID 要同步改脚本。
//! - 守护者9：闪避时不再选守护者的战步（RollingVariationIndex），改用普通翻滚
//! - 无赖1：开始架势（W_SmashAttackStanceStart）时补上「强化已就绪」704011，架势结束必出强化版
//! - 无赖5：重攻击（W_AttackRightHeavy / W_AttackBothHeavy 开头的事件）时挂 3 秒不硬直 + 减伤 30%
//! - 执行者2：完美弹反判定 707000 结束时再挂 0.15 秒的副本，脚本里三处判定改为「原版或副本」
//! - 执行者3：完美弹反成立时直接给妖刀累积器 +50（原版拨开 15 / 20，满 50 解放）
//! - 执行者6：遗物「绝招期间咆哮回血」的回血 7011701（15%）出现时追加一半（7.5%，只给自己）
//! - 学者6：自身技艺的弹反屏障 708050（1.5 秒）结束时再挂 2 秒副本，受击爆发的判定同样认副本

use crate::layout::{SP_EFFECT, sp_effect as sp};
use crate::spec::{RowSpec, Set};

use super::pools::RoleBuff;
use super::rows::{
    EXECUTOR, GUARDIAN, ICON_ATTACK, ICON_DODGE, ICON_SKILL, ICON_ULTIMATE, MARKER_TEMPLATE, NO_SYNC, RAIDER, RARITY_PURPLE, SAVE, SCHOLAR, buff_rows,
    cut, icon, text,
};

const GUARDIAN_ROLL: u32 = 9_971_740;
const RAIDER_ENHANCED_SKILL: u32 = 9_971_750;
const RAIDER_HEAVY_ARMOR: u32 = 9_971_755;
const RAIDER_HEAVY_ARMOR_ON: u32 = 9_971_756;
const EXECUTOR_PARRY_WINDOW: u32 = 9_971_760;
const EXECUTOR_PARRY_WINDOW_ON: u32 = 9_971_761;
const EXECUTOR_QUICK_RELEASE: u32 = 9_971_765;
const EXECUTOR_QUICK_RELEASE_ACCUM: u32 = 9_971_766;
const EXECUTOR_ROAR_HEAL: u32 = 9_971_770;
const EXECUTOR_ROAR_HEAL_EXTRA: u32 = 9_971_771;
const SCHOLAR_LONG_BARRIER: u32 = 9_971_775;
const SCHOLAR_LONG_BARRIER_ON: u32 = 9_971_776;

// 原版
/// 「Prevent all Stagger」：类别 1001，受击全部替换为不硬直
const NO_STAGGER_TEMPLATE: u32 = 5_300;
/// 妖刀完美弹反判定（状态 606，全免伤，防御耗精力 ×0.1）
const JUST_GUARD: u32 = 707_000;
/// 妖刀累积「拨开 +15」
const ACCUM_TEMPLATE: u32 = 707_055;
/// 遗物咆哮回血（每次恢复 15%）
const ROAR_HEAL: u32 = 7_011_701;
/// 学者对自身技艺的弹反屏障（状态 2029，全免伤，拨开力 35）
const SCHOLAR_BARRIER: u32 = 708_050;

const HEAVY_ARMOR_SECONDS: f32 = 3.0;
const HEAVY_ARMOR_CUT: f32 = 0.7;
const PARRY_WINDOW_EXTRA_SECONDS: f32 = 0.15;
/// 妖刀累积器满值（707050 的 accumuOverVal）
const RELEASE_ACCUM: i32 = 50;
const ROAR_HEAL_EXTRA_RATE: f32 = -7.5;
const BARRIER_EXTRA_SECONDS: f32 = 2.0;

const SELF: Set = Set::Bit(sp::TARGET_BYTE, sp::SELF_TARGET_BIT, true);

const fn marker(id: u32, icon_of: super::rows::Icon) -> [Set; 4] {
    [Set::F32(sp::EFFECT_ENDURANCE, -1.0), SAVE, icon(icon_of), text(id)]
}

const GUARDIAN_ROLL_MARKER: [Set; 4] = marker(GUARDIAN_ROLL, ICON_DODGE);
const RAIDER_SKILL_MARKER: [Set; 4] = marker(RAIDER_ENHANCED_SKILL, ICON_SKILL);
const RAIDER_ARMOR_MARKER: [Set; 4] = marker(RAIDER_HEAVY_ARMOR, ICON_ATTACK);
const PARRY_WINDOW_MARKER: [Set; 4] = marker(EXECUTOR_PARRY_WINDOW, ICON_SKILL);
const QUICK_RELEASE_MARKER: [Set; 4] = marker(EXECUTOR_QUICK_RELEASE, ICON_SKILL);
const ROAR_HEAL_MARKER: [Set; 4] = marker(EXECUTOR_ROAR_HEAL, ICON_ULTIMATE);
const BARRIER_MARKER: [Set; 4] = marker(SCHOLAR_LONG_BARRIER, ICON_SKILL);

pub const ROWS: &[RowSpec] = &[
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: GUARDIAN_ROLL, name: "[Tiered Buff] Guardian: Normal roll instead of step", sets: &GUARDIAN_ROLL_MARKER },
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: RAIDER_ENHANCED_SKILL, name: "[Tiered Buff] Raider: Skill always enhanced", sets: &RAIDER_SKILL_MARKER },
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: RAIDER_HEAVY_ARMOR, name: "[Tiered Buff] Raider: Poise and negation on heavy attacks", sets: &RAIDER_ARMOR_MARKER },
    RowSpec { table: &SP_EFFECT, template: NO_STAGGER_TEMPLATE, id: RAIDER_HEAVY_ARMOR_ON, name: "[Tiered Buff] Raider: Poise and negation on heavy attacks - Active",
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, HEAVY_ARMOR_SECONDS), cut(0, HEAVY_ARMOR_CUT), cut(1, HEAVY_ARMOR_CUT), cut(2, HEAVY_ARMOR_CUT), cut(3, HEAVY_ARMOR_CUT),
            cut(4, HEAVY_ARMOR_CUT), cut(5, HEAVY_ARMOR_CUT), cut(6, HEAVY_ARMOR_CUT), cut(7, HEAVY_ARMOR_CUT), SELF, NO_SYNC] },
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: EXECUTOR_PARRY_WINDOW, name: "[Tiered Buff] Executor: Longer cursed sword deflect window", sets: &PARRY_WINDOW_MARKER },
    RowSpec { table: &SP_EFFECT, template: JUST_GUARD, id: EXECUTOR_PARRY_WINDOW_ON, name: "[Tiered Buff] Executor: Longer cursed sword deflect window - Extension",
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, PARRY_WINDOW_EXTRA_SECONDS), SELF] },
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: EXECUTOR_QUICK_RELEASE, name: "[Tiered Buff] Executor: One deflect unlocks cursed sword", sets: &QUICK_RELEASE_MARKER },
    RowSpec { table: &SP_EFFECT, template: ACCUM_TEMPLATE, id: EXECUTOR_QUICK_RELEASE_ACCUM, name: "[Tiered Buff] Executor: One deflect unlocks cursed sword - AccumuVal 50",
        sets: &[Set::I32(sp::ACCUMU_VAL, RELEASE_ACCUM), SELF] },
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: EXECUTOR_ROAR_HEAL, name: "[Tiered Buff] Executor: Stronger roar healing", sets: &ROAR_HEAL_MARKER },
    RowSpec { table: &SP_EFFECT, template: ROAR_HEAL, id: EXECUTOR_ROAR_HEAL_EXTRA, name: "[Tiered Buff] Executor: Stronger roar healing - Extra",
        sets: &[Set::F32(sp::CHANGE_HP_RATE, ROAR_HEAL_EXTRA_RATE), Set::I32(sp::VFX_ID, -1), SELF] },
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: SCHOLAR_LONG_BARRIER, name: "[Tiered Buff] Scholar: Longer parry barrier", sets: &BARRIER_MARKER },
    RowSpec { table: &SP_EFFECT, template: SCHOLAR_BARRIER, id: SCHOLAR_LONG_BARRIER_ON, name: "[Tiered Buff] Scholar: Longer parry barrier - Extension",
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, BARRIER_EXTRA_SECONDS), SELF] },
];

pub const BUFF_ROWS: &[RowSpec] = buff_rows![
    (GUARDIAN_ROLL, GUARDIAN_ROLL, ICON_DODGE, RARITY_PURPLE, "Guardian: Normal roll instead of step"),
    (RAIDER_ENHANCED_SKILL, RAIDER_ENHANCED_SKILL, ICON_SKILL, RARITY_PURPLE, "Raider: Skill always enhanced"),
    (RAIDER_HEAVY_ARMOR, RAIDER_HEAVY_ARMOR, ICON_ATTACK, RARITY_PURPLE, "Raider: Poise and negation on heavy attacks"),
    (EXECUTOR_PARRY_WINDOW, EXECUTOR_PARRY_WINDOW, ICON_SKILL, RARITY_PURPLE, "Executor: Longer cursed sword deflect window"),
    (EXECUTOR_QUICK_RELEASE, EXECUTOR_QUICK_RELEASE, ICON_SKILL, RARITY_PURPLE, "Executor: One deflect unlocks cursed sword"),
    (EXECUTOR_ROAR_HEAL, EXECUTOR_ROAR_HEAL, ICON_ULTIMATE, RARITY_PURPLE, "Executor: Stronger roar healing"),
    (SCHOLAR_LONG_BARRIER, SCHOLAR_LONG_BARRIER, ICON_SKILL, RARITY_PURPLE, "Scholar: Longer parry barrier"),
];

pub const ROLE: &[RoleBuff] = &[
    RoleBuff { hero: GUARDIAN, buff: GUARDIAN_ROLL },
    RoleBuff { hero: RAIDER, buff: RAIDER_ENHANCED_SKILL },
    RoleBuff { hero: RAIDER, buff: RAIDER_HEAVY_ARMOR },
    RoleBuff { hero: EXECUTOR, buff: EXECUTOR_PARRY_WINDOW },
    RoleBuff { hero: EXECUTOR, buff: EXECUTOR_QUICK_RELEASE },
    RoleBuff { hero: EXECUTOR, buff: EXECUTOR_ROAR_HEAL },
    RoleBuff { hero: SCHOLAR, buff: SCHOLAR_LONG_BARRIER },
];
