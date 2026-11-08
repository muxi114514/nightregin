//! 第八批：守护者6 / 7（时机由 DLL 判断，见 shared::tiered_guardian 与 src\tiered_effects\guardian.rs）。
//! - 守护者6：圣律加成 = 原版战技「圣律」1841 的复制（+10% 伤害、对死诞者 ×2），改为 1.5 秒、普通类别（不顶掉武器附魔）；
//!   发给队友照第四批 aura.rs 的写法：发出效果（状态 275 + 行为）→ 行为 → 子弹（只打队友，半径 15）
//! - 守护者7：照原版旋风的四套子弹（普通 / 蓄力 × 普通 / 遗物延长）各复制一份冰风暴：几何、寿命与位置都沿用旋风，
//!   换成冰风暴特效、每 0.5 秒打一次的冰属性攻击与原版魔法「冰风暴」的冻伤累积；延长版另外定时补发冰风暴特效。
//!   动作事件由 tools\tiered_tae.ps1 照旋风事件复制：普通版认状态 2393、延长版认 2258（都借自原版铁之眼遗物，只有铁之眼动作 a902 认），
//!   判定 6930~6933 → 行为 600000930~933（a917 等动作已占用 699x，换号前要扫全部 TAE；改编号要同步改脚本）

use nr_stat_alloc_shared::tiered_guardian::{GUARDIAN_ICE, GUARDIAN_SACRED, ICE_LONG, ICE_LONG_BODIES, ICE_SHORT, SACRED_BUFF, SACRED_SPAWN};
use nr_stat_alloc_shared::tiered_params::GUARDIAN_ICE_ATK;

use crate::layout::{ATK_PC, BEHAVIOR_PC, BULLET, SP_EFFECT, atk, behavior, bullet, sp_effect as sp};
use crate::spec::{RowSpec, Set};

use super::pools::RoleBuff;
use super::rows::{GUARDIAN, ICON_SKILL, ICON_ULTIMATE, MARKER_TEMPLATE, RARITY_PURPLE, SAVE, buff_rows, icon, text};

const SACRED: u32 = GUARDIAN_SACRED as u32;
const SPAWN: u32 = SACRED_SPAWN as u32;
const BUFF: u32 = SACRED_BUFF as u32;
const ICE: u32 = GUARDIAN_ICE as u32;
const SHORT: u32 = ICE_SHORT as u32;
const LONG: u32 = ICE_LONG as u32;

/// 发给队友的行为与子弹（同号，接在第四批 900997101~107 之后）
const SACRED_AURA: u32 = 900_997_108;
/// 冰风暴子弹：先导（普通蓄力 / 普通 / 延长蓄力 / 延长）、本体（同顺序）、补发的特效
const ICE_ROOTS: [u32; 4] = [900_997_120, 900_997_121, 900_997_122, 900_997_123];
const ICE_BODIES: [u32; 4] = [900_997_124, 900_997_125, ICE_LONG_BODIES[0], ICE_LONG_BODIES[1]];
const ICE_VISUAL: u32 = 900_997_128;
/// 判定 6930~6933 换算的行为号，顺序同上
const ICE_BEHAVIORS: [u32; 4] = [600_000_930, 600_000_931, 600_000_932, 600_000_933];

/// 普通版借用的状态（原版铁之眼遗物 7500200）、延长版借用的状态（铁之眼遗物「蓄力绝招附加中毒」7034601）
const ICE_SHORT_STATE: u16 = 2393;
const ICE_LONG_STATE: u16 = 2258;

// 原版
/// 战技「圣律」的武器增益（60 秒，周期挂 1849 +10% 伤害，类别 162 武器附魔）
const SACRED_ORDER: u32 = 1_841;
/// 遗物「圣杯瓶也能治疗队友」的发出效果、行为与子弹（半径 15、只打队友）
const SPAWN_TEMPLATE: u32 = 7_010_280;
const AURA_TEMPLATE: u32 = 900_102_221;
/// 旋风：普通（蓄力 / 不蓄力）与遗物延长版（7 / 4 秒）的先导子弹与本体，顺序同 ICE_ROOTS
const WHIRLWIND_ROOTS: [u32; 4] = [30_901_000, 30_901_005, 30_901_110, 30_901_115];
const WHIRLWIND_BODIES: [u32; 4] = [30_901_001, 30_901_006, 30_901_111, 30_901_116];
/// 旋风遗物延长版的攻击行（攻击力修正 15%），作冰风暴每一下的攻击
const WHIRLWIND_TICK_ATK: u32 = 600_000_083;
/// 原版魔法「冰风暴」：命中挂的冻伤累积（35）、爆开特效与持续的风暴特效（1.5 秒子弹）
const ICE_STORM_FROST: i32 = 1_441_000;
const ICE_STORM_BURST_SFX: i32 = 523_877;
const ICE_STORM_SFX: i32 = 523_878;
const ICE_STORM_VISUAL: u32 = 3_058;
/// 冰属性命中特效
const ICE_SP_ATTRIBUTE: u8 = 23;

const SACRED_SECONDS: f32 = 1.5;
const AURA_RADIUS: f32 = 15.0;
const ICE_HIT_INTERVAL: f32 = 0.5;
/// 延长版补发风暴特效的间隔（特效 1.5 秒，留一点重叠）
const ICE_VISUAL_INTERVAL: f32 = 1.2;

const SELF: Set = Set::Bit(sp::TARGET_BYTE, sp::SELF_TARGET_BIT, true);

const fn marker(id: u32, icon_of: super::rows::Icon) -> [Set; 4] {
    [Set::F32(sp::EFFECT_ENDURANCE, -1.0), SAVE, icon(icon_of), text(id)]
}

/// 冰风暴第 $i 套：先导（换本体、普通版的先导带爆开特效）、本体、行为
macro_rules! ice_storm {
    ($i:literal, $root_sfx:expr, $visual:expr, $name:literal) => {
        [
            RowSpec { table: &BULLET, template: WHIRLWIND_ROOTS[$i], id: ICE_ROOTS[$i], name: concat!("[Tiered Buff] Guardian: Ice storm whirlwind - ", $name),
                sets: &[Set::I32(bullet::HIT_BULLET_ID, ICE_BODIES[$i] as i32), Set::I32(bullet::SFX_ID, $root_sfx)] },
            RowSpec { table: &BULLET, template: WHIRLWIND_BODIES[$i], id: ICE_BODIES[$i], name: concat!("[Tiered Buff] Guardian: Ice storm whirlwind - ", $name),
                sets: &[Set::I32(bullet::ATK_ID, GUARDIAN_ICE_ATK as i32), Set::I32(bullet::SFX_ID, ICE_STORM_SFX), Set::I32(bullet::SP_EFFECT_0, ICE_STORM_FROST),
                    Set::F32(bullet::DMG_HIT_RECORD_LIFE_TIME, ICE_HIT_INTERVAL), Set::I32(bullet::INTERVAL_CREATE_BULLET, $visual),
                    Set::F32(bullet::INTERVAL_CREATE_TIME_MIN, ICE_VISUAL_INTERVAL), Set::F32(bullet::INTERVAL_CREATE_TIME_MAX, ICE_VISUAL_INTERVAL)] },
            RowSpec { table: &BEHAVIOR_PC, template: WHIRLWIND_BEHAVIOR, id: ICE_BEHAVIORS[$i], name: concat!("[Tiered Buff] Guardian: Ice storm whirlwind - ", $name),
                sets: &[Set::I32(behavior::JUDGE_ID, (ICE_BEHAVIORS[$i] % 1000) as i32), Set::I32(behavior::REF_ID, ICE_ROOTS[$i] as i32)] },
        ]
    };
}

/// 原版普通蓄力旋风的行为（判定 6050 → 子弹 30901000）
const WHIRLWIND_BEHAVIOR: u32 = 600_000_050;

pub const ROWS: &[RowSpec] = &[
    // 守护者6
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: SACRED, name: "[Tiered Buff] Guardian: Ult shares Sacred Order", sets: &marker(SACRED, ICON_ULTIMATE) },
    RowSpec { table: &SP_EFFECT, template: SACRED_ORDER, id: BUFF, name: "[Tiered Buff] Guardian: Ult shares Sacred Order - Buff",
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, SACRED_SECONDS), Set::U16(sp::SP_CATEGORY, 20), Set::U16(sp::STATE_INFO, 0), SELF] },
    RowSpec { table: &SP_EFFECT, template: SPAWN_TEMPLATE, id: SPAWN, name: "[Tiered Buff] Guardian: Ult shares Sacred Order - Spawn",
        sets: &[Set::I32(sp::BEHAVIOR_ID, SACRED_AURA as i32)] },
    RowSpec { table: &BEHAVIOR_PC, template: AURA_TEMPLATE, id: SACRED_AURA, name: "[Tiered Buff] Guardian: Ult shares Sacred Order",
        sets: &[Set::I32(behavior::REF_ID, SACRED_AURA as i32)] },
    RowSpec { table: &BULLET, template: AURA_TEMPLATE, id: SACRED_AURA, name: "[Tiered Buff] Guardian: Ult shares Sacred Order",
        sets: &[Set::F32(bullet::HIT_RADIUS, AURA_RADIUS), Set::I32(bullet::SP_EFFECT_0, BUFF as i32), Set::I32(bullet::SP_EFFECT_1, 0)] },
    // 守护者7：两个状态效果由 DLL 二选一挂着
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: ICE, name: "[Tiered Buff] Guardian: Ice storm whirlwind", sets: &marker(ICE, ICON_SKILL) },
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: SHORT, name: "[Tiered Buff] Guardian: Ice storm whirlwind - Normal",
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, -1.0), Set::U16(sp::STATE_INFO, ICE_SHORT_STATE)] },
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: LONG, name: "[Tiered Buff] Guardian: Ice storm whirlwind - Extended",
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, -1.0), Set::U16(sp::STATE_INFO, ICE_LONG_STATE)] },
    RowSpec { table: &ATK_PC, template: WHIRLWIND_TICK_ATK, id: GUARDIAN_ICE_ATK, name: "[Tiered Buff] Guardian: Ice storm whirlwind",
        sets: &[Set::U8(atk::SP_ATTRIBUTE, ICE_SP_ATTRIBUTE)] },
    RowSpec { table: &BULLET, template: ICE_STORM_VISUAL, id: ICE_VISUAL, name: "[Tiered Buff] Guardian: Ice storm whirlwind - Visual", sets: &[] },
];

const ICE_CHARGED: [RowSpec; 3] = ice_storm!(0, ICE_STORM_BURST_SFX, -1, "Charged");
const ICE_UNCHARGED: [RowSpec; 3] = ice_storm!(1, ICE_STORM_BURST_SFX, -1, "Uncharged");
const ICE_LONG_CHARGED: [RowSpec; 3] = ice_storm!(2, -1, ICE_VISUAL as i32, "Extended Charged");
const ICE_LONG_UNCHARGED: [RowSpec; 3] = ice_storm!(3, -1, ICE_VISUAL as i32, "Extended Uncharged");

pub const ROW_GROUPS: [&[RowSpec]; 5] = [ROWS, &ICE_CHARGED, &ICE_UNCHARGED, &ICE_LONG_CHARGED, &ICE_LONG_UNCHARGED];

pub const BUFF_ROWS: &[RowSpec] = buff_rows![
    (SACRED, SACRED, ICON_ULTIMATE, RARITY_PURPLE, "Guardian: Ult shares Sacred Order"),
    (ICE, ICE, ICON_SKILL, RARITY_PURPLE, "Guardian: Ice storm whirlwind"),
];

pub const ROLE: &[RoleBuff] = &[RoleBuff { hero: GUARDIAN, buff: SACRED }, RoleBuff { hero: GUARDIAN, buff: ICE }];
