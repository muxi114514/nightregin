//! 第三批：一个锻体要挂多个效果的词条（锻体行只能指向一个效果）。锻体效果只是标记，子效果紧跟在后面连续编号，
//! tools\tiered_buffs.lua 的 TIERED_KEEP 表（标记, 第一个子效果, 个数）在标记在身上时把子效果保持挂着。改 ID / 个数要同步改脚本。
//! - 周围异常：原版各异常的「附近子弹」给周围角色挂瞬时状态（出血 379、冻伤 2280、中毒 / 腐败 380、猩红腐败 2064、睡眠 2378、
//!   发狂 2379），一个效果最多认 3 个状态（之间是「或」），所以分两条触发效果，出现时周期施加结果（同原版「周围出血时提升攻击力」）
//! - 武器数量：原版「装备 3 把以上 X」遗物的 wepTypeTrigger / Count，每类 1~6 把各一行，第 k 行倍率 (1+0.1k)/(1+0.1(k-1))，
//!   叠起来正好是 1 + 0.1 × 把数；不同类别之间相乘（小盾、中盾各 1 面是 1.21 倍）

use crate::layout::{SP_EFFECT, sp_effect as sp};
use crate::spec::{RowSpec, Set};

use super::pools::{Candidate, RoleBuff};
use super::rows::{
    DUCHESS, EXECUTOR, ICON_ATTACK, ICON_DISCOVERY, ICON_HP, ICON_NEGATION, ICON_SKILL, IRONEYE, MARKER_TEMPLATE, NO_SYNC, RARITY_GOLD,
    RARITY_PURPLE, SAVE, buff_rows, icon, text, vanilla,
};

// 金7：周围陷入异常时攻击力 +20%（20 秒，不叠加）
const AILMENT_ATTACK: u32 = 9_971_270;
const AILMENT_ATTACK_BUFF: u32 = 9_971_273;
// 紫17~19：每把盾 / 法杖与圣印记 / 大盾 +10% 精力 / 专注 / 血量上限
const SHIELD_STAMINA: u32 = 9_971_170;
const CATALYST_FP: u32 = 9_971_180;
const GREATSHIELD_HP: u32 = 9_971_190;
/// 各自子效果的第一个编号
const SHIELD_STAMINA_FIRST: u32 = 9_971_500;
const CATALYST_FP_FIRST: u32 = 9_971_520;
const GREATSHIELD_HP_FIRST: u32 = 9_971_540;
// 角色：周围陷入异常时，执行者叠一层不屈、女爵隐身 5 秒、铁之眼观察力永久 +1
const EXECUTOR_AILMENT_TENACITY: u32 = 9_971_390;
const DUCHESS_AILMENT_CONCEAL: u32 = 9_971_400;
const IRONEYE_AILMENT_DISCOVERY: u32 = 9_971_410;

// 结果效果：原版由引擎 / 击杀触发施加的复制一份并打开「可施加给自己」，周期触发才挂得上（同 wylder_elements 的命定之死）
/// 执行者能力「不屈」开始（1.5 秒后换成 707071 持续 18.5 秒，类别 10 可叠）
const TENACITY_TEMPLATE: u32 = 707_070;
const TENACITY: u32 = 9_971_393;
/// 遗物「周围冻伤时隐身」的 5 秒隐身（状态 600），原版就是周期施加
const CONCEALMENT: u32 = 7_260_703;
/// 铁之眼遗物「击杀时提升观察力」：特效 0.1 秒后换成观察力 +1 的永久层（类别 204 逐层叠加）
const DISCOVERY_STACK_TEMPLATE: u32 = 7_280_101;
const DISCOVERY_STACK: u32 = 9_971_413;

// 模板
/// 护符「周围出血时提升攻击力」：常驻、持有状态 379 时周期施加 321601
const VICINITY_TEMPLATE: u32 = 321_600;
/// 武器词条「周围出血时提升攻击力」的 20 秒攻击力 +20%
const AILMENT_ATTACK_TEMPLATE: u32 = 8_710_003;
/// 遗物「装备 3 把以上小盾时提升血量」（状态 2100）
const WEAPON_COUNT_TEMPLATE: u32 = 7_082_700;

// 武器表 wepType
const SMALL_SHIELD: u16 = 65;
const MEDIUM_SHIELD: u16 = 67;
const GREATSHIELD: u16 = 69;
const STAFF: u16 = 57;
const SEAL: u16 = 61;

const SELF: Set = Set::Bit(sp::TARGET_BYTE, sp::SELF_TARGET_BIT, true);

const fn marker(id: u32, icon_of: super::rows::Icon) -> [Set; 4] {
    [Set::F32(sp::EFFECT_ENDURANCE, -1.0), SAVE, icon(icon_of), text(id)]
}

/// 周围异常的两条触发（标记 +1 / +2）：状态 379 / 2280 / 380 与 2064 / 2378 / 2379，任一出现时施加 $payload
macro_rules! ailment_triggers {
    ($marker:expr, $payload:expr, $name:literal) => {
        [
            RowSpec { table: &SP_EFFECT, template: VICINITY_TEMPLATE, id: $marker + 1, name: concat!("[Tiered Buff] ", $name, " - Trigger A"),
                sets: &[Set::U16(sp::INVOCATION_STATE_1, 379), Set::U16(sp::INVOCATION_STATE_2, 2_280), Set::U16(sp::INVOCATION_STATE_3, 380),
                    Set::I32(sp::CYCLE_OCCURRENCE, $payload as i32), NO_SYNC] },
            RowSpec { table: &SP_EFFECT, template: VICINITY_TEMPLATE, id: $marker + 2, name: concat!("[Tiered Buff] ", $name, " - Trigger B"),
                sets: &[Set::U16(sp::INVOCATION_STATE_1, 2_064), Set::U16(sp::INVOCATION_STATE_2, 2_378), Set::U16(sp::INVOCATION_STATE_3, 2_379),
                    Set::I32(sp::CYCLE_OCCURRENCE, $payload as i32), NO_SYNC] },
        ]
    };
}

/// 某类武器 1~6 把各一行（$first 起连续编号），倍率写进 $field
macro_rules! per_weapon {
    ($first:expr, $wep:expr, $field:expr, $name:literal) => {
        [
            per_weapon!(@row $first, $wep, $field, 1, $name),
            per_weapon!(@row $first, $wep, $field, 2, $name),
            per_weapon!(@row $first, $wep, $field, 3, $name),
            per_weapon!(@row $first, $wep, $field, 4, $name),
            per_weapon!(@row $first, $wep, $field, 5, $name),
            per_weapon!(@row $first, $wep, $field, 6, $name),
        ]
    };
    (@row $first:expr, $wep:expr, $field:expr, $k:literal, $name:literal) => {
        RowSpec { table: &SP_EFFECT, template: WEAPON_COUNT_TEMPLATE, id: $first + $k - 1, name: concat!("[Tiered Buff] ", $name, " x", $k),
            sets: &[Set::U16(sp::WEP_TYPE_TRIGGER, $wep), Set::U8(sp::WEP_TYPE_TRIGGER_COUNT, $k), Set::U8(sp::ADD_VIGOR, 0),
                Set::F32($field, (1.0 + 0.1 * $k as f32) / (1.0 + 0.1 * ($k - 1) as f32)), Set::I32(sp::ICON_ID, -1), Set::I32(sp::TEXT_ID_1, -1),
                NO_SYNC] }
    };
}

const MARKERS: &[RowSpec] = &[
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: AILMENT_ATTACK, name: "[Tiered Buff] Ailment in vicinity increases attack",
        sets: &marker(AILMENT_ATTACK, ICON_ATTACK) },
    RowSpec { table: &SP_EFFECT, template: AILMENT_ATTACK_TEMPLATE, id: AILMENT_ATTACK_BUFF, name: "[Tiered Buff] Ailment in vicinity increases attack - Buff",
        sets: &[NO_SYNC] },
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: SHIELD_STAMINA, name: "[Tiered Buff] Max stamina per shield",
        sets: &marker(SHIELD_STAMINA, ICON_NEGATION) },
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: CATALYST_FP, name: "[Tiered Buff] Max FP per staff or seal",
        sets: &marker(CATALYST_FP, vanilla(292)) },
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: GREATSHIELD_HP, name: "[Tiered Buff] Max HP per greatshield",
        sets: &marker(GREATSHIELD_HP, ICON_HP) },
    RowSpec { table: &SP_EFFECT, template: TENACITY_TEMPLATE, id: TENACITY, name: "[Tiered Buff] Executor: Ailment in vicinity grants Tenacity - Stack",
        sets: &[SELF] },
    RowSpec { table: &SP_EFFECT, template: DISCOVERY_STACK_TEMPLATE, id: DISCOVERY_STACK, name: "[Tiered Buff] Ironeye: Ailment in vicinity raises discovery - Stack",
        sets: &[SELF] },
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: EXECUTOR_AILMENT_TENACITY, name: "[Tiered Buff] Executor: Ailment in vicinity grants Tenacity",
        sets: &marker(EXECUTOR_AILMENT_TENACITY, ICON_ATTACK) },
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: DUCHESS_AILMENT_CONCEAL, name: "[Tiered Buff] Duchess: Ailment in vicinity conceals self",
        sets: &marker(DUCHESS_AILMENT_CONCEAL, ICON_SKILL) },
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: IRONEYE_AILMENT_DISCOVERY, name: "[Tiered Buff] Ironeye: Ailment in vicinity raises discovery",
        sets: &marker(IRONEYE_AILMENT_DISCOVERY, ICON_DISCOVERY) },
];

const AILMENT_ATTACK_TRIGGERS: [RowSpec; 2] = ailment_triggers!(AILMENT_ATTACK, AILMENT_ATTACK_BUFF, "Ailment in vicinity increases attack");
const EXECUTOR_TRIGGERS: [RowSpec; 2] = ailment_triggers!(EXECUTOR_AILMENT_TENACITY, TENACITY, "Executor: Ailment in vicinity grants Tenacity");
const DUCHESS_TRIGGERS: [RowSpec; 2] = ailment_triggers!(DUCHESS_AILMENT_CONCEAL, CONCEALMENT, "Duchess: Ailment in vicinity conceals self");
const IRONEYE_TRIGGERS: [RowSpec; 2] = ailment_triggers!(IRONEYE_AILMENT_DISCOVERY, DISCOVERY_STACK, "Ironeye: Ailment in vicinity raises discovery");
const SMALL_SHIELD_ROWS: [RowSpec; 6] = per_weapon!(SHIELD_STAMINA_FIRST, SMALL_SHIELD, sp::MAX_STAMINA_RATE, "Max stamina per small shield");
const MEDIUM_SHIELD_ROWS: [RowSpec; 6] = per_weapon!(SHIELD_STAMINA_FIRST + 6, MEDIUM_SHIELD, sp::MAX_STAMINA_RATE, "Max stamina per medium shield");
const GREATSHIELD_STAMINA_ROWS: [RowSpec; 6] = per_weapon!(SHIELD_STAMINA_FIRST + 12, GREATSHIELD, sp::MAX_STAMINA_RATE, "Max stamina per greatshield");
const STAFF_ROWS: [RowSpec; 6] = per_weapon!(CATALYST_FP_FIRST, STAFF, sp::MAX_MP_RATE, "Max FP per staff");
const SEAL_ROWS: [RowSpec; 6] = per_weapon!(CATALYST_FP_FIRST + 6, SEAL, sp::MAX_MP_RATE, "Max FP per seal");
const GREATSHIELD_HP_ROWS: [RowSpec; 6] = per_weapon!(GREATSHIELD_HP_FIRST, GREATSHIELD, sp::MAX_HP_RATE, "Max HP per greatshield");

pub const ROW_GROUPS: [&[RowSpec]; 11] = [
    MARKERS,
    &AILMENT_ATTACK_TRIGGERS,
    &EXECUTOR_TRIGGERS,
    &DUCHESS_TRIGGERS,
    &IRONEYE_TRIGGERS,
    &SMALL_SHIELD_ROWS,
    &MEDIUM_SHIELD_ROWS,
    &GREATSHIELD_STAMINA_ROWS,
    &STAFF_ROWS,
    &SEAL_ROWS,
    &GREATSHIELD_HP_ROWS,
];

pub const BUFF_ROWS: &[RowSpec] = buff_rows![
    (AILMENT_ATTACK, AILMENT_ATTACK, ICON_ATTACK, RARITY_GOLD, "Ailment in vicinity increases attack"),
    (SHIELD_STAMINA, SHIELD_STAMINA, ICON_NEGATION, RARITY_PURPLE, "Max stamina per shield"),
    (CATALYST_FP, CATALYST_FP, vanilla(292), RARITY_PURPLE, "Max FP per staff or seal"),
    (GREATSHIELD_HP, GREATSHIELD_HP, ICON_HP, RARITY_PURPLE, "Max HP per greatshield"),
    (EXECUTOR_AILMENT_TENACITY, EXECUTOR_AILMENT_TENACITY, ICON_ATTACK, RARITY_PURPLE, "Executor: Ailment in vicinity grants Tenacity"),
    (DUCHESS_AILMENT_CONCEAL, DUCHESS_AILMENT_CONCEAL, ICON_SKILL, RARITY_PURPLE, "Duchess: Ailment in vicinity conceals self"),
    (IRONEYE_AILMENT_DISCOVERY, IRONEYE_AILMENT_DISCOVERY, ICON_DISCOVERY, RARITY_PURPLE, "Ironeye: Ailment in vicinity raises discovery"),
];

pub const PURPLE: &[Candidate] = &[Candidate::Buff(SHIELD_STAMINA), Candidate::Buff(CATALYST_FP), Candidate::Buff(GREATSHIELD_HP)];

pub const GOLD: &[Candidate] = &[Candidate::Buff(AILMENT_ATTACK)];

pub const ROLE: &[RoleBuff] = &[
    RoleBuff { hero: EXECUTOR, buff: EXECUTOR_AILMENT_TENACITY },
    RoleBuff { hero: DUCHESS, buff: DUCHESS_AILMENT_CONCEAL },
    RoleBuff { hero: IRONEYE, buff: IRONEYE_AILMENT_DISCOVERY },
];
