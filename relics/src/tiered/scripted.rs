//! 第二批：靠动作脚本触发的锻体。锻体效果本身只是标记，tools\tiered_buffs.lua 按标记 ID 在放绝招（「绝招中」6999615 出现的那一刻）、
//! 放技艺、每帧检查时施加下面的效果，或拦下技艺 / 绝招。改 ID 要同步改脚本。
//! 金9（喝圣杯瓶）、金13（共鸣，送葬者状态 2227）直接用原版状态。

use crate::layout::{SP_EFFECT, sp_effect as sp};
use crate::spec::{RowSpec, Set};

use super::pools::{Candidate, RoleBuff};
use super::rows::{
    DUCHESS, GUARDIAN, ICON_ATTACK, ICON_DODGE, ICON_HP, ICON_NEGATION, ICON_SKILL, ICON_ULTIMATE, NON_STACKING, NO_SYNC, ON_HIT_RESTORE_TEMPLATE,
    ON_HIT_TEMPLATE, MARKER_TEMPLATE, PERIODIC_TEMPLATE, RARITY_GOLD, RARITY_PURPLE, SAVE, UNDERTAKER, attack, buff_rows, cut, icon, text,
};

// 紫1：绝招后 10 秒内击杀返还 50% 量表（只返还一次）
const ULT_KILL_REFUND: u32 = 9_971_130;
const ULT_KILL_WINDOW: u32 = 9_971_131;
const ULT_KILL_GAUGE: u32 = 9_971_132;
// 紫2：满血时翻滚 / 后跳消耗的精力 -25%（脚本包住 AddStamina 打折）
const FULL_HP_DODGE: u32 = 9_971_160;
// 紫6：圣杯瓶为 0 时减伤 40%
const EMPTY_FLASK_GUARD: u32 = 9_971_140;
const EMPTY_FLASK_GUARD_ON: u32 = 9_971_141;
// 紫14：放绝招时回复 50% 血量
const ULT_HEAL: u32 = 9_971_150;
const ULT_HEAL_RESTORE: u32 = 9_971_151;
// 金5：绝招动画结束后返还全部量表，之后 30 秒冷却
const ULT_REFILL: u32 = 9_971_210;
const ULT_REFILL_GAUGE: u32 = 9_971_212;
const ULT_REFILL_COOLDOWN: u32 = 9_971_213;
// 金9：喝圣杯瓶时霸体且免伤
const FLASK_ARMOR: u32 = 9_971_220;
const FLASK_ARMOR_ON: u32 = 9_971_221;
// 金10：不能放绝招，技艺冷却减半，放技艺后 10 秒攻击力 +15%、命中回 15 血
const SKILL_ONLY: u32 = 9_971_230;
const SKILL_ONLY_BUFF: u32 = 9_971_231;
const SKILL_ONLY_HEAL: u32 = 9_971_232;
// 金11：不能放技艺，伤害 +30%，绝招量表累积 +20%
const NO_SKILL: u32 = 9_971_240;
// 金13：送葬者的「共鸣」（闪避投技或附近队友放绝招时获得一次免费绝招）
const RESONANCE: u32 = 9_971_250;
/// 非送葬者没有免费版绝招动画，放免费绝招仍会被动画（事件 1000）扣量表；结束后按二进制组合还回去：+1 / +2 / +4 … +64
const RESONANCE_REFUND: [u32; 7] = [9_971_251, 9_971_252, 9_971_253, 9_971_254, 9_971_255, 9_971_256, 9_971_257];
/// 非送葬者没有紫色量表，拿到免费绝招（709300，4 秒）时脚本挂这个状态栏图标提示，用掉或到期时清掉
const RESONANCE_SHOWN: u32 = 9_971_258;
/// 送葬者「共鸣」709300 的状态栏图标
const RESONANCE_STATUS_ICON: i32 = 20_495;
// 守护者12：强化防御时精力不会耗尽
const STEEL_GUARD_FREE: u32 = 9_971_370;
// 女爵8：隐身（状态 600）时伤害 +15%
const DUCHESS_HIDDEN_DAMAGE: u32 = 9_971_440;
// 执行者4：兽化（707100）期间血量下降就由 DLL 挂回满（shared::tiered_beast），挂在第一批的标记 9971350 上
const EXECUTOR_BEAST_END_HEAL: u32 = 9_971_352;
// 送葬者3：恍惚（709000 / 709010）期间脚本补挂 0.5 秒霸体
const UNDERTAKER_TRANCE_POISE: u32 = 9_971_496;
const UNDERTAKER_TRANCE_POISE_ON: u32 = 9_971_497;
const STEEL_GUARD_FREE_ON: u32 = 9_971_371;

// 模板
/// 大空洞恩惠：6999611 量表 +100（0.1 秒，结束换成 6999613）→ 6999613 40 秒（图标 20497）
const GAUGE_REFILL_TEMPLATE: u32 = 6_999_611;
const GAUGE_COOLDOWN_TEMPLATE: u32 = 6_999_613;
/// 遗物「击杀敌人时累积绝招量表」：状态 199 + 击杀时施加。实测普通击杀只触发 APPLY_ON_GET_SOUL，只填 APPLY_ON_KILL_SP 不返还
const ON_KILL_TEMPLATE: u32 = 7_090_000;
/// 遗物「在图腾石碑附近击杀敌人回血」的回复效果：血量回复 50%（原带状态条件 2042，清掉）
const HEAL_HALF_TEMPLATE: u32 = 7_090_303;
/// 隐士遗物「发动绝招时出血并提升攻击力」的 30 秒攻击力增益（带状态栏图标与文本）
const TIMED_BUFF_TEMPLATE: u32 = 7_032_903;
const SKILL_COOLDOWN_TEMPLATE: u32 = 8_370_000;
const GAUGE_RATE_TEMPLATE: u32 = 7_000_900;
/// 送葬者护甲效果：施加状态 2227（免费绝招 709300 / 709301 的生效条件）
const RESONANCE_TEMPLATE: u32 = 7_999_090;
/// 受击等级全换 1（类别 1001，不硬直）
const NO_STAGGER_TEMPLATE: u32 = 5_300;
/// 守护者强化防御的减伤与防御精力倍率（状态 158）
const STEEL_GUARD_BOOST_TEMPLATE: u32 = 701_535;

/// 喝圣杯瓶期间的状态值（原版深夜词条 6850900 用它做条件）
const DRINKING_STATE: u16 = 2_352;
/// 紫1 击杀窗口与返还共用的类别 200 + 独占优先级：返还施加时顶掉窗口，所以只返还一次
const ONE_SHOT_CATEGORY: u16 = 200;
const ONE_SHOT_PRIORITY: u8 = 71;
/// 攻击力增益的状态栏图标（原版攻击力提升）
const ATTACK_UP_STATUS: i32 = 20_406;

/// 状态栏要显示的计时效果：图标 + 自己的文本
const fn shown(icon_id: i32, id: u32) -> [Set; 2] {
    [Set::I32(sp::ICON_ID, icon_id), text(id)]
}

const COOLDOWN_SHOWN: [Set; 2] = shown(20_497, ULT_REFILL_COOLDOWN);
const KILL_WINDOW_SHOWN: [Set; 2] = shown(ICON_ULTIMATE.status, ULT_KILL_WINDOW);
const SKILL_BUFF_SHOWN: [Set; 2] = shown(ATTACK_UP_STATUS, SKILL_ONLY_BUFF);
const GUARD_SHOWN: [Set; 2] = shown(ICON_NEGATION.status, EMPTY_FLASK_GUARD_ON);
const RESONANCE_SHOWN_SETS: [Set; 2] = shown(RESONANCE_STATUS_ICON, RESONANCE_SHOWN);

/// 一次性加量表（照恩惠 6999611：0.1 秒、周期 0.2 秒只加一次），不换成冷却、不播特效
macro_rules! refund {
    ($id:expr, $gauge:expr) => {
        RowSpec { table: &SP_EFFECT, template: GAUGE_REFILL_TEMPLATE, id: $id, name: "[Tiered Buff] Resonance - Gauge Refund",
            sets: &[Set::F32(sp::ULTIMATE_ART_GAUGE, $gauge), Set::I32(sp::REPLACE_SP_EFFECT, -1), Set::I32(sp::VFX_ID, -1)] }
    };
}

pub const ROWS: &[RowSpec] = &[
    // 紫1
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: ULT_KILL_REFUND, name: "[Tiered Buff] Ult kills refund Art gauge",
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, -1.0), SAVE, icon(ICON_ULTIMATE), text(ULT_KILL_REFUND)] },
    RowSpec { table: &SP_EFFECT, template: ON_KILL_TEMPLATE, id: ULT_KILL_WINDOW, name: "[Tiered Buff] Ult kills refund Art gauge - Window",
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, 10.0), Set::I32(sp::APPLY_ON_GET_SOUL, ULT_KILL_GAUGE as i32),
            Set::I32(sp::APPLY_ON_KILL_SP, -1), Set::U16(sp::SP_CATEGORY, ONE_SHOT_CATEGORY),
            Set::U8(sp::CATEGORY_PRIORITY, ONE_SHOT_PRIORITY), KILL_WINDOW_SHOWN[0], KILL_WINDOW_SHOWN[1]] },
    // 照恩惠 6999611 的写法（0.1 秒、周期 0.2 秒）只加一次；没设周期的 0.1 秒效果实测把量表加满了
    RowSpec { table: &SP_EFFECT, template: GAUGE_REFILL_TEMPLATE, id: ULT_KILL_GAUGE, name: "[Tiered Buff] Ult kills refund Art gauge - Refund",
        sets: &[Set::F32(sp::ULTIMATE_ART_GAUGE, 50.0), Set::I32(sp::REPLACE_SP_EFFECT, -1), Set::U16(sp::SP_CATEGORY, ONE_SHOT_CATEGORY),
            Set::U8(sp::CATEGORY_PRIORITY, ONE_SHOT_PRIORITY)] },
    // 紫6：不限时，脚本在圣杯瓶为 0 时挂上、有瓶时移除（短时补挂会让状态栏图标一直闪）
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: EMPTY_FLASK_GUARD, name: "[Tiered Buff] Damage negation with empty flasks",
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, -1.0), SAVE, icon(ICON_NEGATION), text(EMPTY_FLASK_GUARD)] },
    RowSpec { table: &SP_EFFECT, template: TIMED_BUFF_TEMPLATE, id: EMPTY_FLASK_GUARD_ON, name: "[Tiered Buff] Damage negation with empty flasks - Active",
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, -1.0), attack(0, 1.0), attack(1, 1.0), attack(2, 1.0), attack(3, 1.0), attack(4, 1.0),
            cut(0, 0.6), cut(1, 0.6), cut(2, 0.6), cut(3, 0.6), cut(4, 0.6), cut(5, 0.6), cut(6, 0.6), cut(7, 0.6),
            Set::I32(sp::VFX_ID, -1), NO_SYNC, GUARD_SHOWN[0], GUARD_SHOWN[1]] },
    // 紫2
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: FULL_HP_DODGE, name: "[Tiered Buff] Cheaper dodging at full HP",
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, -1.0), SAVE, icon(ICON_DODGE), text(FULL_HP_DODGE)] },
    // 紫14
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: ULT_HEAL, name: "[Tiered Buff] Ult restores HP",
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, -1.0), SAVE, icon(ICON_ULTIMATE), text(ULT_HEAL)] },
    RowSpec { table: &SP_EFFECT, template: HEAL_HALF_TEMPLATE, id: ULT_HEAL_RESTORE, name: "[Tiered Buff] Ult restores HP - Restore",
        sets: &[Set::U16(sp::INVOCATION_STATE_1, 0)] },
    // 金5：绝招在动画中段发动时才扣量表，所以等动画结束（「绝招中」6999615 消失）由脚本直接补满，补满后进入 30 秒冷却；
    // 补满与冷却期间放绝招不再触发
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: ULT_REFILL, name: "[Tiered Buff] Art gauge refills after Ult",
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, -1.0), SAVE, icon(ICON_ULTIMATE), text(ULT_REFILL)] },
    RowSpec { table: &SP_EFFECT, template: GAUGE_REFILL_TEMPLATE, id: ULT_REFILL_GAUGE, name: "[Tiered Buff] Art gauge refills after Ult - Refill",
        sets: &[Set::I32(sp::REPLACE_SP_EFFECT, ULT_REFILL_COOLDOWN as i32)] },
    RowSpec { table: &SP_EFFECT, template: GAUGE_COOLDOWN_TEMPLATE, id: ULT_REFILL_COOLDOWN, name: "[Tiered Buff] Art gauge refills after Ult - Cooldown",
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, 30.0), Set::F32(sp::ULTIMATE_ART_GAUGE_RATE, 1.0), Set::I32(sp::VFX_ID, -1),
            COOLDOWN_SHOWN[0], COOLDOWN_SHOWN[1]] },
    // 金9：喝圣杯瓶期间每 0.1 秒挂 0.2 秒的不硬直 + 全属性免伤
    RowSpec { table: &SP_EFFECT, template: PERIODIC_TEMPLATE, id: FLASK_ARMOR, name: "[Tiered Buff] Poise and immunity while drinking",
        sets: &[Set::F32(sp::MOTION_INTERVAL, 0.1), Set::I32(sp::CYCLE_OCCURRENCE, FLASK_ARMOR_ON as i32),
            Set::U16(sp::INVOCATION_STATE_1, DRINKING_STATE), icon(ICON_HP),
            text(FLASK_ARMOR)] },
    RowSpec { table: &SP_EFFECT, template: NO_STAGGER_TEMPLATE, id: FLASK_ARMOR_ON, name: "[Tiered Buff] Poise and immunity while drinking - Active",
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, 0.2), cut(0, 0.0), cut(1, 0.0), cut(2, 0.0), cut(3, 0.0), cut(4, 0.0), cut(5, 0.0), cut(6, 0.0),
            cut(7, 0.0), Set::Bit(sp::TARGET_BYTE, sp::SELF_TARGET_BIT, true), NO_SYNC] },
    // 金10
    RowSpec { table: &SP_EFFECT, template: SKILL_COOLDOWN_TEMPLATE, id: SKILL_ONLY, name: "[Tiered Buff] No Ult, faster Skill",
        sets: &[Set::F32(sp::CHARACTER_SKILL_COOLDOWN, 0.5), Set::U16(sp::SP_CATEGORY, NON_STACKING), icon(ICON_SKILL), text(SKILL_ONLY)] },
    RowSpec { table: &SP_EFFECT, template: ON_HIT_TEMPLATE, id: SKILL_ONLY_BUFF, name: "[Tiered Buff] No Ult, faster Skill - After Skill",
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, 10.0), Set::I32(sp::ATK_OCCURRENCE, SKILL_ONLY_HEAL as i32), attack(0, 1.15), attack(1, 1.15),
            attack(2, 1.15), attack(3, 1.15), attack(4, 1.15), SKILL_BUFF_SHOWN[0], SKILL_BUFF_SHOWN[1]] },
    RowSpec { table: &SP_EFFECT, template: ON_HIT_RESTORE_TEMPLATE, id: SKILL_ONLY_HEAL, name: "[Tiered Buff] No Ult, faster Skill - Restore",
        sets: &[Set::I32(sp::CHANGE_STAMINA_POINT, 0), Set::I32(sp::CHANGE_HP_POINT, -15)] },
    // 金11
    RowSpec { table: &SP_EFFECT, template: GAUGE_RATE_TEMPLATE, id: NO_SKILL, name: "[Tiered Buff] No Skill, more damage and Art gauge",
        sets: &[Set::F32(sp::ULTIMATE_ART_GAUGE_RATE, 1.2), Set::F32(sp::ULTIMATE_ART_GAUGE_CHARGE_RATE, 1.2), attack(0, 1.3), attack(1, 1.3),
            attack(2, 1.3), attack(3, 1.3), attack(4, 1.3), Set::U16(sp::SP_CATEGORY, NON_STACKING), SAVE, icon(ICON_ATTACK), text(NO_SKILL)] },
    // 金13
    RowSpec { table: &SP_EFFECT, template: RESONANCE_TEMPLATE, id: RESONANCE, name: "[Tiered Buff] Resonance",
        sets: &[Set::U16(sp::SP_CATEGORY, NON_STACKING), SAVE, icon(ICON_ULTIMATE), text(RESONANCE)] },
    refund!(RESONANCE_REFUND[0], 1.0),
    refund!(RESONANCE_REFUND[1], 2.0),
    refund!(RESONANCE_REFUND[2], 4.0),
    refund!(RESONANCE_REFUND[3], 8.0),
    refund!(RESONANCE_REFUND[4], 16.0),
    refund!(RESONANCE_REFUND[5], 32.0),
    refund!(RESONANCE_REFUND[6], 64.0),
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: RESONANCE_SHOWN, name: "[Tiered Buff] Resonance - Free Ult Ready",
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, 4.0), NO_SYNC, RESONANCE_SHOWN_SETS[0], RESONANCE_SHOWN_SETS[1]] },
    // 守护者12：强化防御中（701530）脚本补挂：防御受击不耗精力，并每 0.1 秒回 1 点抵掉持续消耗（原版每 0.35 秒扣 1）
    // 女爵8：常驻，只在隐身时生效
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: DUCHESS_HIDDEN_DAMAGE, name: "[Tiered Buff] Duchess: More damage while concealed",
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, -1.0), attack(0, 1.15), attack(1, 1.15), attack(2, 1.15), attack(3, 1.15), attack(4, 1.15),
            Set::U16(sp::INVOCATION_STATE_1, 600), SAVE, icon(ICON_ATTACK), text(DUCHESS_HIDDEN_DAMAGE)] },
    // 每次受伤都会补，去掉回血特效免得刷屏
    RowSpec { table: &SP_EFFECT, template: HEAL_HALF_TEMPLATE, id: EXECUTOR_BEAST_END_HEAL, name: "[Tiered Buff] Executor: HP locked full while transformed - Heal",
        sets: &[Set::F32(sp::CHANGE_HP_RATE, -100.0), Set::U16(sp::INVOCATION_STATE_1, 0), Set::I32(sp::VFX_ID, -1)] },
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: UNDERTAKER_TRANCE_POISE, name: "[Tiered Buff] Undertaker: Poise during Trance",
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, -1.0), SAVE, icon(ICON_SKILL), text(UNDERTAKER_TRANCE_POISE)] },
    RowSpec { table: &SP_EFFECT, template: NO_STAGGER_TEMPLATE, id: UNDERTAKER_TRANCE_POISE_ON, name: "[Tiered Buff] Undertaker: Poise during Trance - Active",
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, 0.5), Set::Bit(sp::TARGET_BYTE, sp::SELF_TARGET_BIT, true), NO_SYNC] },
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: STEEL_GUARD_FREE, name: "[Tiered Buff] Guardian: Steel Guard costs no stamina",
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, -1.0), SAVE, icon(ICON_NEGATION), text(STEEL_GUARD_FREE)] },
    RowSpec { table: &SP_EFFECT, template: STEEL_GUARD_BOOST_TEMPLATE, id: STEEL_GUARD_FREE_ON, name: "[Tiered Buff] Guardian: Steel Guard costs no stamina - Active",
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, 0.5), Set::F32(sp::MOTION_INTERVAL, 0.1), Set::I32(sp::CHANGE_STAMINA_POINT, -1),
            Set::F32(sp::GUARD_STAMINA_MULT, 0.0), cut(0, 1.0), cut(1, 1.0), cut(2, 1.0), cut(3, 1.0), cut(4, 1.0), cut(5, 1.0), cut(6, 1.0),
            cut(7, 1.0), NO_SYNC] },
];

pub const BUFF_ROWS: &[RowSpec] = buff_rows![
    (ULT_KILL_REFUND, ULT_KILL_REFUND, ICON_ULTIMATE, RARITY_PURPLE, "Ult kills refund Art gauge"),
    (EMPTY_FLASK_GUARD, EMPTY_FLASK_GUARD, ICON_NEGATION, RARITY_PURPLE, "Damage negation with empty flasks"),
    (FULL_HP_DODGE, FULL_HP_DODGE, ICON_DODGE, RARITY_PURPLE, "Cheaper dodging at full HP"),
    (ULT_HEAL, ULT_HEAL, ICON_ULTIMATE, RARITY_PURPLE, "Ult restores HP"),
    (ULT_REFILL, ULT_REFILL, ICON_ULTIMATE, RARITY_GOLD, "Art gauge refills after Ult"),
    (FLASK_ARMOR, FLASK_ARMOR, ICON_HP, RARITY_GOLD, "Poise and immunity while drinking"),
    (SKILL_ONLY, SKILL_ONLY, ICON_SKILL, RARITY_GOLD, "No Ult, faster Skill"),
    (NO_SKILL, NO_SKILL, ICON_ATTACK, RARITY_GOLD, "No Skill, more damage and Art gauge"),
    (RESONANCE, RESONANCE, ICON_ULTIMATE, RARITY_GOLD, "Resonance"),
    (STEEL_GUARD_FREE, STEEL_GUARD_FREE, ICON_NEGATION, RARITY_PURPLE, "Guardian: Steel Guard costs no stamina"),
    (DUCHESS_HIDDEN_DAMAGE, DUCHESS_HIDDEN_DAMAGE, ICON_ATTACK, RARITY_PURPLE, "Duchess: More damage while concealed"),
    (UNDERTAKER_TRANCE_POISE, UNDERTAKER_TRANCE_POISE, ICON_SKILL, RARITY_PURPLE, "Undertaker: Poise during Trance"),
];

pub const PURPLE: &[Candidate] =
    &[Candidate::Buff(ULT_KILL_REFUND), Candidate::Buff(EMPTY_FLASK_GUARD), Candidate::Buff(ULT_HEAL), Candidate::Buff(FULL_HP_DODGE)];

pub const GOLD: &[Candidate] = &[
    Candidate::Buff(ULT_REFILL),
    Candidate::Buff(FLASK_ARMOR),
    Candidate::Buff(SKILL_ONLY),
    Candidate::Buff(NO_SKILL),
    Candidate::Buff(RESONANCE),
];

pub const ROLE: &[RoleBuff] = &[
    RoleBuff { hero: GUARDIAN, buff: STEEL_GUARD_FREE },
    RoleBuff { hero: DUCHESS, buff: DUCHESS_HIDDEN_DAMAGE },
    RoleBuff { hero: UNDERTAKER, buff: UNDERTAKER_TRANCE_POISE },
];
