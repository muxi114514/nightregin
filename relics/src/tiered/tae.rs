//! 第五批（动作）：靠动作文件里带状态条件的事件触发的角色锻体。锻体标记带一个新状态值（2180 起，参数 / 动作 / 脚本都没用过），
//! tools\tiered_tae.ps1 照原版遗物事件复制一份只认这个状态值的事件（判定 → 行为 → 攻击 / 子弹）。改编号要同步改脚本。
//! - 铁之眼3 / 4：照原版毒遗物（技艺命中的追加判定 6120、绝招箭替换标记 7034601）换成猩红腐败
//! - 学者5：照原版遗物「绝招连结的目标异常伤害增加」（判定 7401 → 子弹挂 7036701），倍率 130 → 175
//! - 追踪者3 / 送葬者6：扣量表的那一刻之后发一颗只给自己加效果的子弹挂「待返还」标记（非蓄力绝招 70 → 50、强化技艺 100 → 55）。
//!   扣量表同时有一段量表锁定（动作类型 1002，与「绝招中」一起结束），锁定期间加量表无效，强化恍惚期间量表增长倍率也是 0，
//!   所以由 DLL 等这些都结束再还（shared::tiered_refund）
//! - 执行者7：执行者的刀类招式在 a277 / a278（不是刀类通用的 a29），每招各有一份只在拔出妖刀（状态 609）时触发、参数第 13 字节为 3 的
//!   事件，用妖刀（挂点 51xx）出招。给这些事件各复制两份带状态 2393 的攻击，判定号同原版铁之眼遗物用四位的全局判定（6998 / 6999 →
//!   行为 600000998 / 999，不随武器变体）。锻体标记直接带状态 2393（借用原版铁之眼遗物的状态，只有铁之眼动作 a902 认它）：
//!   复制的攻击只在「用妖刀出招」的事件里，不拔刀不会播放，不需要脚本再判断拔刀。攻击照原版铁之眼追加判定（不造成伤害、不硬直），
//!   判定球用妖刀刀身，每份挂三种异常（原版武器的异常命中效果，累积 30；攻击行只有 5 个效果槽，所以拆两份）。
//!   两份各用一个本动画没用过的攻击编号（同铁之眼遗物追加攻击用 1、与本体 0 错开），否则和原版那一下共用命中名单，敌人已被打中就不会再中。
//!   诊断确认：事件会触发、条件状态 2393 生效；之前无效就是攻击编号相同被跳过（更早还有附魔挂不上、加错到 a29 等问题）
//! - 执行者8：解放斩（a907 571020，未解放的斩击是 571025，不受影响）整段加一条认状态的跳转表 8（无敌），同原版女爵遗物（a903 539000 + 状态 2394）；
//!   招式表换成 a293（DLL 按角色切换，见 src\crucible_sword.rs，动作在 tools\tiered_moveset.ps1）：重攻击变奥陶漩涡，纳刀架势派生志留亚漩涡 / 百相之羽。
//!   这里只有冲击子弹的行为，子弹本身用原版的（原版行为按战技变体算，妖刀算不到，所以换成全局判定）
//! - 执行者自带能力（不是锻体，所有执行者都有）：妖刀解放状态 707051 原本没有属性，加上攻击力 +15%、减伤 +20%，
//!   所有异常抗性同深夜遗物负面词条「降低对所有异常状态的抵抗力」（6850200）。说明文本在 tools\tiered_texts.ps1

use nr_stat_alloc_shared::tiered_refund::{UNDERTAKER_GAUGE, UNDERTAKER_PENDING, WYLDER_GAUGE, WYLDER_PENDING};

use crate::layout::{ATK_PC, BEHAVIOR_PC, BULLET, SP_EFFECT, atk, behavior, bullet, sp_effect as sp};
use crate::spec::{Patch, RowSpec, Set};

use super::pools::RoleBuff;
use super::rows::{
    EXECUTOR, ICON_SKILL, ICON_ULTIMATE, IRONEYE, MARKER_TEMPLATE, RARITY_PURPLE, SAVE, SCHOLAR, UNDERTAKER, WYLDER, buff_rows, icon, text,
};

// 铁之眼3：技艺附加猩红腐败，对腐败中的敌人造成大伤害
const IRONEYE_SKILL_ROT: u32 = 9_971_600;
const IRONEYE_SKILL_ROT_INFLICT: u32 = 9_971_601;
const IRONEYE_SKILL_ROT_BONUS: u32 = 9_971_602;
// 铁之眼4：蓄力绝招箭附加猩红腐败
const IRONEYE_ULT_ROT: u32 = 9_971_605;
const IRONEYE_ULT_ROT_MARK: u32 = 9_971_606;
const IRONEYE_ULT_ROT_INFLICT: u32 = 9_971_607;
// 学者5：被绝招连结的敌人受到的异常伤害 +75%
const SCHOLAR_LINK_AILMENT: u32 = 9_971_610;
const SCHOLAR_LINK_AILMENT_DEBUFF: u32 = 9_971_611;
// 追踪者3：非蓄力绝招的量表消耗 70% → 50%
const WYLDER_ULT_REFUND: u32 = 9_971_615;
const WYLDER_ULT_REFUND_GAUGE: u32 = WYLDER_GAUGE as u32;
const WYLDER_ULT_REFUND_PENDING: u32 = WYLDER_PENDING as u32;
// 送葬者6：强化技艺的量表消耗 100% → 55%
const UNDERTAKER_TRANCE_REFUND: u32 = 9_971_620;
const UNDERTAKER_TRANCE_REFUND_GAUGE: u32 = UNDERTAKER_GAUGE as u32;
const UNDERTAKER_TRANCE_REFUND_PENDING: u32 = UNDERTAKER_PENDING as u32;
// 执行者7：妖刀攻击附加全异常；命中效果 +0~5 = 中毒 / 猩红腐败 / 出血 / 冻伤 / 睡眠 / 发狂
const EXECUTOR_ALL_AILMENTS: u32 = 9_971_650;
const EXECUTOR_AILMENT: u32 = 9_971_657;
// 执行者8：熔炉妖刀（解放斩期间无敌 + 招式表换成熔炉系列）
const EXECUTOR_RELEASE_INVINCIBLE: u32 = 9_971_865;

// 执行者自带能力：妖刀解放状态
const UNLEASHED_SWORD: u32 = 707_051;
const UNLEASHED_ATTACK_RATE: f32 = 1.15;
const UNLEASHED_DAMAGE_RATE: f32 = 0.8;
const UNLEASHED_RESIST_POINTS: i32 = -80;

pub const UNLEASHED_SWORD_STATS: Patch = Patch {
    table: &SP_EFFECT,
    id: UNLEASHED_SWORD,
    sets: &[
        Set::F32(sp::ATTACK_RATES[0], UNLEASHED_ATTACK_RATE), Set::F32(sp::ATTACK_RATES[1], UNLEASHED_ATTACK_RATE), Set::F32(sp::ATTACK_RATES[2], UNLEASHED_ATTACK_RATE),
        Set::F32(sp::ATTACK_RATES[3], UNLEASHED_ATTACK_RATE), Set::F32(sp::ATTACK_RATES[4], UNLEASHED_ATTACK_RATE),
        Set::F32(sp::DAMAGE_CUT_RATES[0], UNLEASHED_DAMAGE_RATE), Set::F32(sp::DAMAGE_CUT_RATES[1], UNLEASHED_DAMAGE_RATE), Set::F32(sp::DAMAGE_CUT_RATES[2], UNLEASHED_DAMAGE_RATE),
        Set::F32(sp::DAMAGE_CUT_RATES[3], UNLEASHED_DAMAGE_RATE), Set::F32(sp::DAMAGE_CUT_RATES[4], UNLEASHED_DAMAGE_RATE), Set::F32(sp::DAMAGE_CUT_RATES[5], UNLEASHED_DAMAGE_RATE),
        Set::F32(sp::DAMAGE_CUT_RATES[6], UNLEASHED_DAMAGE_RATE), Set::F32(sp::DAMAGE_CUT_RATES[7], UNLEASHED_DAMAGE_RATE),
        Set::I32(sp::RESIST_POINTS[0], UNLEASHED_RESIST_POINTS), Set::I32(sp::RESIST_POINTS[1], UNLEASHED_RESIST_POINTS), Set::I32(sp::RESIST_POINTS[2], UNLEASHED_RESIST_POINTS),
        Set::I32(sp::RESIST_POINTS[3], UNLEASHED_RESIST_POINTS), Set::I32(sp::RESIST_POINTS[4], UNLEASHED_RESIST_POINTS), Set::I32(sp::RESIST_POINTS[5], UNLEASHED_RESIST_POINTS),
        Set::I32(sp::RESIST_POINTS[6], UNLEASHED_RESIST_POINTS),
        Set::Bit(sp::EFFECT_TARGET_BYTE, sp::EFFECT_TARGET_BITS[0], true), Set::Bit(sp::EFFECT_TARGET_BYTE, sp::EFFECT_TARGET_BITS[1], true),
        Set::Bit(sp::EFFECT_TARGET_BYTE, sp::EFFECT_TARGET_BITS[2], true), Set::Bit(sp::EFFECT_TARGET_BYTE, sp::EFFECT_TARGET_BITS[3], true),
        Set::Bit(sp::EFFECT_TARGET_BYTE, sp::EFFECT_TARGET_BITS[4], true), Set::Bit(sp::EFFECT_TARGET_BYTE, sp::EFFECT_TARGET_BITS[5], true),
    ],
};

/// 动作事件的状态条件（同步改 tools\tiered_tae.ps1）
const IRONEYE_SKILL_ROT_STATE: u16 = 2180;
const IRONEYE_ULT_ROT_STATE: u16 = 2181;
const SCHOLAR_LINK_STATE: u16 = 2182;
const WYLDER_REFUND_STATE: u16 = 2183;
const UNDERTAKER_REFUND_STATE: u16 = 2184;
const EXECUTOR_INVINCIBLE_STATE: u16 = 2185;
/// 借用原版铁之眼遗物「技艺对中毒敌人大伤害」（7500200）的状态：只有铁之眼的动作 a902 认它，执行者不会播那些动作
const EXECUTOR_AILMENT_STATE: u16 = 2393;

/// 新增的攻击行 / 子弹（900997110 起，接在第四批的 900997101~107 之后）
const ROT_SKILL_ATK: u32 = 900_997_110;
const ROT_ARROW_BULLET: u32 = 900_997_111;
const LINK_BULLET: u32 = 900_997_112;
const WYLDER_REFUND_BULLET: u32 = 900_997_113;
const UNDERTAKER_REFUND_BULLET: u32 = 900_997_114;
/// 900997115 / 116 是第六批的冲击波与火焰爆发
const CURSED_AILMENT_ATK: [u32; 2] = [900_997_117, 900_997_118];
/// 行为号 = 判定号换算（6125 → 600000125、7146 → 700000146）
const ROT_SKILL_BEHAVIOR: u32 = 600_000_125;
const ROT_ARROW_BEHAVIOR: u32 = 700_000_146;
const LINK_BEHAVIOR: u32 = 700_000_402;
const WYLDER_REFUND_BEHAVIOR: u32 = 700_000_496;
const UNDERTAKER_REFUND_BEHAVIOR: u32 = 700_000_497;
/// 判定 6998 / 6999 换算（原版没用过）
const CURSED_AILMENT_BEHAVIOR: [u32; 2] = [600_000_998, 600_000_999];
/// 判定 7498 / 7499 换算：奥陶漩涡蓄力版 / 快版的冲击（tools\tiered_moveset.ps1）
const ORDOVIS_VORTEX_BEHAVIOR: [u32; 2] = [700_000_498, 700_000_499];
/// 判定 7493 / 7494 换算：志留亚漩涡蓄力版 / 快版的冲击
const SILURIA_VORTEX_BEHAVIOR: [u32; 2] = [700_000_493, 700_000_494];

// 原版
/// 铁之眼遗物「技艺对中毒的敌人造成大伤害」：技艺追加判定的攻击与行为（命中挂 6902 / 7500201 中毒 / 7500202 对中毒追加伤害）
const POISON_SKILL_ATK: u32 = 600_000_120;
const POISON_BONUS_TEMPLATE: u32 = 7_500_202;
/// 铁之眼遗物「蓄力绝招附加中毒」：常驻 7034600 每 0.1 秒挂替换标记 7034601（类别 202）顶掉绝招开头挂的普通标记
const POISON_ARROW_RELIC: u32 = 7_034_600;
const POISON_ARROW_MARK: u32 = 7_034_601;
const POISON_ARROW_BEHAVIOR: u32 = 700_000_140;
/// 普通绝招箭（不带异常，命中地面换 30902013）
const ARROW_BULLET: u32 = 30_902_010;
/// 原版猩红腐败（40 秒，每 1.3 秒掉血，状态 5）
const ROT_TEMPLATE: u32 = 6_600;
/// 学者遗物：绝招连结时的子弹与减益
const LINK_RELIC_BEHAVIOR: u32 = 700_000_401;
const LINK_RELIC_BULLET: u32 = 30_908_011;
const LINK_DEBUFF_TEMPLATE: u32 = 7_036_701;
/// 大空洞恩惠：一次性加量表
const GAUGE_REFILL_TEMPLATE: u32 = 6_999_611;
/// 空效果子弹（配「No Target」攻击，只给发射者加效果）
const FX_BULLET: u32 = 2_900;
/// 奥陶琵斯漩涡的冲击子弹（半径 3）：蓄力版连发三次、快版一次
const ORDOVIS_VORTEX_BULLET: [u32; 2] = [2_780, 2_783];
/// 志留亚漩涡的冲击子弹：蓄力版向前远射、快版就地爆发
const SILURIA_VORTEX_BULLET: [u32; 2] = [2_381, 2_382];
/// 原版武器的异常命中效果「Lvl 1-2 +45」：中毒 / 猩红腐败 / 出血 / 冻伤 / 睡眠 / 发狂（同 sp::AILMENT_ATTACK_POWERS 的顺序）
const AILMENT_TEMPLATES: [u32; 6] = [106_010, 107_010, 105_010, 107_510, 105_510, 108_510];
/// 每次命中的累积（同 1 级油脂）
const AILMENT_BUILDUP: i32 = 30;
/// 妖刀轻攻击 960200 的判定球：刀身胶囊（挂点 5100~5110，半径 0.35）+ 刀尖球（5120，0.2）
const BLADE_DMY_POLYS: [u16; 3] = [5_100, 5_110, 5_120];
const BLADE_RADII: [f32; 2] = [0.35, 0.2];
const BLADE_TIP_HIT_TYPE: u8 = 2;
/// 不要命中特效
const NO_SP_ATTRIBUTE: u8 = 255;

const SKILL_ROT_BUILDUP: i32 = 70;
const ARROW_ROT_BUILDUP: i32 = 300;
const LINK_AILMENT_RATE: u8 = 175;
const WYLDER_REFUND: f32 = 20.0;
const UNDERTAKER_REFUND: f32 = 45.0;
/// 「待返还」标记的时长：DLL 还完就移除，这只是 DLL 不在时的兜底；要长过送葬者强化恍惚（15 秒，送葬者4 再 ×1.5）加技艺动作
const PENDING_SECONDS: f32 = 60.0;
/// 腐败中（原版「对中毒的敌人追加伤害」的条件状态 2 换成 5）
const ROT_STATE: u16 = 5;

const fn marker(id: u32, state: u16, icon_of: super::rows::Icon) -> [Set; 5] {
    [Set::F32(sp::EFFECT_ENDURANCE, -1.0), Set::U16(sp::STATE_INFO, state), SAVE, icon(icon_of), text(id)]
}

/// 只给发射者加效果的子弹
const fn self_bullet(effect: u32) -> [Set; 3] {
    [Set::I32(bullet::ATK_ID, atk::NO_TARGET), Set::F32(bullet::HIT_RADIUS, bullet::SELF_EFFECT_RADIUS), Set::I32(bullet::SP_EFFECT_FOR_SHOOTER, effect as i32)]
}

const fn refund(gauge: f32) -> [Set; 3] {
    [Set::F32(sp::ULTIMATE_ART_GAUGE, gauge), Set::I32(sp::REPLACE_SP_EFFECT, -1), Set::I32(sp::VFX_ID, -1)]
}

/// 执行者7 第 k 种异常的命中效果：原版武器版，累积改 30
macro_rules! ailment {
    ($k:literal, $name:literal) => {
        RowSpec { table: &SP_EFFECT, template: AILMENT_TEMPLATES[$k], id: EXECUTOR_AILMENT + $k, name: $name,
            sets: &[Set::I32(sp::AILMENT_ATTACK_POWERS[$k], AILMENT_BUILDUP)] }
    };
}

/// 执行者7 第 i 份攻击：命中挂第 3i ~ 3i+2 种异常，判定球用妖刀刀身
macro_rules! cursed_ailment_atk {
    ($i:literal, $name:literal) => {
        RowSpec { table: &ATK_PC, template: POISON_SKILL_ATK, id: CURSED_AILMENT_ATK[$i], name: $name,
            sets: &[Set::I32(atk::SP_EFFECT_0, (EXECUTOR_AILMENT + 3 * $i) as i32), Set::I32(atk::SP_EFFECT_1, (EXECUTOR_AILMENT + 3 * $i + 1) as i32),
                Set::I32(atk::SP_EFFECT_2, (EXECUTOR_AILMENT + 3 * $i + 2) as i32), Set::F32(atk::HIT0_RADIUS, BLADE_RADII[0]),
                Set::F32(atk::HIT1_RADIUS, BLADE_RADII[1]), Set::U16(atk::HIT0_DMY_POLY1, BLADE_DMY_POLYS[0]), Set::U16(atk::HIT0_DMY_POLY2, BLADE_DMY_POLYS[1]),
                Set::U16(atk::HIT1_DMY_POLY1, BLADE_DMY_POLYS[2]), Set::U8(atk::HIT1_HIT_TYPE, BLADE_TIP_HIT_TYPE), Set::U8(atk::SP_ATTRIBUTE, NO_SP_ATTRIBUTE)] }
    };
}

/// 执行者7 第 i 份攻击的行为
macro_rules! cursed_ailment_behavior {
    ($i:literal, $name:literal) => {
        RowSpec { table: &BEHAVIOR_PC, template: POISON_SKILL_ATK, id: CURSED_AILMENT_BEHAVIOR[$i], name: $name,
            sets: &[Set::I32(behavior::JUDGE_ID, (CURSED_AILMENT_BEHAVIOR[$i] % 1000) as i32), Set::I32(behavior::REF_ID, CURSED_AILMENT_ATK[$i] as i32)] }
    };
}

const SKILL_ROT_MARKER: [Set; 5] = marker(IRONEYE_SKILL_ROT, IRONEYE_SKILL_ROT_STATE, ICON_SKILL);
const LINK_MARKER: [Set; 5] = marker(SCHOLAR_LINK_AILMENT, SCHOLAR_LINK_STATE, ICON_ULTIMATE);
const WYLDER_MARKER: [Set; 5] = marker(WYLDER_ULT_REFUND, WYLDER_REFUND_STATE, ICON_ULTIMATE);
const UNDERTAKER_MARKER: [Set; 5] = marker(UNDERTAKER_TRANCE_REFUND, UNDERTAKER_REFUND_STATE, ICON_SKILL);
const AILMENTS_MARKER: [Set; 5] = marker(EXECUTOR_ALL_AILMENTS, EXECUTOR_AILMENT_STATE, ICON_SKILL);
const INVINCIBLE_MARKER: [Set; 5] = marker(EXECUTOR_RELEASE_INVINCIBLE, EXECUTOR_INVINCIBLE_STATE, ICON_SKILL);
const WYLDER_BULLET: [Set; 3] = self_bullet(WYLDER_ULT_REFUND_PENDING);
const UNDERTAKER_BULLET: [Set; 3] = self_bullet(UNDERTAKER_TRANCE_REFUND_PENDING);
const PENDING: [Set; 2] = [Set::F32(sp::EFFECT_ENDURANCE, PENDING_SECONDS), Set::I32(sp::ICON_ID, -1)];
const WYLDER_REFUND_GAUGE_SETS: [Set; 3] = refund(WYLDER_REFUND);
const UNDERTAKER_REFUND_GAUGE_SETS: [Set; 3] = refund(UNDERTAKER_REFUND);

pub const ROWS: &[RowSpec] = &[
    // 铁之眼3
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: IRONEYE_SKILL_ROT, name: "[Tiered Buff] Ironeye: Skill inflicts scarlet rot", sets: &SKILL_ROT_MARKER },
    RowSpec { table: &SP_EFFECT, template: ROT_TEMPLATE, id: IRONEYE_SKILL_ROT_INFLICT, name: "[Tiered Buff] Ironeye: Skill inflicts scarlet rot - Rot",
        sets: &[Set::I32(sp::DISEASE_ATTACK_POWER, SKILL_ROT_BUILDUP)] },
    RowSpec { table: &SP_EFFECT, template: POISON_BONUS_TEMPLATE, id: IRONEYE_SKILL_ROT_BONUS, name: "[Tiered Buff] Ironeye: Skill inflicts scarlet rot - Extra Damage",
        sets: &[Set::U16(sp::INVOCATION_STATE_1, ROT_STATE)] },
    RowSpec { table: &ATK_PC, template: POISON_SKILL_ATK, id: ROT_SKILL_ATK, name: "[Tiered Buff] Ironeye: Skill inflicts scarlet rot",
        sets: &[Set::I32(atk::SP_EFFECT_1, IRONEYE_SKILL_ROT_INFLICT as i32), Set::I32(atk::SP_EFFECT_2, IRONEYE_SKILL_ROT_BONUS as i32)] },
    RowSpec { table: &BEHAVIOR_PC, template: POISON_SKILL_ATK, id: ROT_SKILL_BEHAVIOR, name: "[Tiered Buff] Ironeye: Skill inflicts scarlet rot",
        sets: &[Set::I32(behavior::JUDGE_ID, (ROT_SKILL_BEHAVIOR % 1000) as i32), Set::I32(behavior::REF_ID, ROT_SKILL_ATK as i32)] },
    // 铁之眼4：同原版毒遗物的替换机制，标记每 0.1 秒挂一次替换标记（类别 202 顶掉绝招开头的普通标记，箭事件认它的状态值）
    RowSpec { table: &SP_EFFECT, template: POISON_ARROW_RELIC, id: IRONEYE_ULT_ROT, name: "[Tiered Buff] Ironeye: Charged Ult inflicts scarlet rot",
        sets: &[Set::I32(sp::CYCLE_OCCURRENCE, IRONEYE_ULT_ROT_MARK as i32), icon(ICON_ULTIMATE), text(IRONEYE_ULT_ROT)] },
    RowSpec { table: &SP_EFFECT, template: POISON_ARROW_MARK, id: IRONEYE_ULT_ROT_MARK, name: "[Tiered Buff] Ironeye: Charged Ult inflicts scarlet rot - Change Ult",
        sets: &[Set::U16(sp::STATE_INFO, IRONEYE_ULT_ROT_STATE)] },
    RowSpec { table: &SP_EFFECT, template: ROT_TEMPLATE, id: IRONEYE_ULT_ROT_INFLICT, name: "[Tiered Buff] Ironeye: Charged Ult inflicts scarlet rot - Rot",
        sets: &[Set::I32(sp::DISEASE_ATTACK_POWER, ARROW_ROT_BUILDUP)] },
    RowSpec { table: &BULLET, template: ARROW_BULLET, id: ROT_ARROW_BULLET, name: "[Tiered Buff] Ironeye: Charged Ult inflicts scarlet rot - Arrow",
        sets: &[Set::I32(bullet::SP_EFFECT_0, IRONEYE_ULT_ROT_INFLICT as i32)] },
    RowSpec { table: &BEHAVIOR_PC, template: POISON_ARROW_BEHAVIOR, id: ROT_ARROW_BEHAVIOR, name: "[Tiered Buff] Ironeye: Charged Ult inflicts scarlet rot - Arrow",
        sets: &[Set::I32(behavior::JUDGE_ID, (ROT_ARROW_BEHAVIOR % 1000) as i32), Set::I32(behavior::REF_ID, ROT_ARROW_BULLET as i32)] },
    // 学者5
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: SCHOLAR_LINK_AILMENT, name: "[Tiered Buff] Scholar: Linked foes take more ailment damage", sets: &LINK_MARKER },
    RowSpec { table: &SP_EFFECT, template: LINK_DEBUFF_TEMPLATE, id: SCHOLAR_LINK_AILMENT_DEBUFF, name: "[Tiered Buff] Scholar: Linked foes take more ailment damage - Debuff",
        sets: &[Set::U8(sp::AILMENT_DAMAGE_RATES[0], LINK_AILMENT_RATE), Set::U8(sp::AILMENT_DAMAGE_RATES[1], LINK_AILMENT_RATE),
            Set::U8(sp::AILMENT_DAMAGE_RATES[2], LINK_AILMENT_RATE), Set::U8(sp::AILMENT_DAMAGE_RATES[3], LINK_AILMENT_RATE),
            Set::U8(sp::AILMENT_DAMAGE_RATES[4], LINK_AILMENT_RATE), Set::U8(sp::AILMENT_DAMAGE_RATES[5], LINK_AILMENT_RATE)] },
    RowSpec { table: &BULLET, template: LINK_RELIC_BULLET, id: LINK_BULLET, name: "[Tiered Buff] Scholar: Linked foes take more ailment damage",
        sets: &[Set::I32(bullet::SP_EFFECT_0, SCHOLAR_LINK_AILMENT_DEBUFF as i32)] },
    RowSpec { table: &BEHAVIOR_PC, template: LINK_RELIC_BEHAVIOR, id: LINK_BEHAVIOR, name: "[Tiered Buff] Scholar: Linked foes take more ailment damage",
        sets: &[Set::I32(behavior::JUDGE_ID, (LINK_BEHAVIOR % 1000) as i32), Set::I32(behavior::REF_ID, LINK_BULLET as i32)] },
    // 追踪者3
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: WYLDER_ULT_REFUND, name: "[Tiered Buff] Wylder: Uncharged Ult refunds more gauge", sets: &WYLDER_MARKER },
    RowSpec { table: &SP_EFFECT, template: GAUGE_REFILL_TEMPLATE, id: WYLDER_ULT_REFUND_GAUGE, name: "[Tiered Buff] Wylder: Uncharged Ult refunds more gauge - Refund",
        sets: &WYLDER_REFUND_GAUGE_SETS },
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: WYLDER_ULT_REFUND_PENDING, name: "[Tiered Buff] Wylder: Uncharged Ult refunds more gauge - Pending",
        sets: &PENDING },
    RowSpec { table: &BULLET, template: FX_BULLET, id: WYLDER_REFUND_BULLET, name: "[Tiered Buff] Wylder: Uncharged Ult refunds more gauge", sets: &WYLDER_BULLET },
    RowSpec { table: &BEHAVIOR_PC, template: LINK_RELIC_BEHAVIOR, id: WYLDER_REFUND_BEHAVIOR, name: "[Tiered Buff] Wylder: Uncharged Ult refunds more gauge",
        sets: &[Set::I32(behavior::JUDGE_ID, (WYLDER_REFUND_BEHAVIOR % 1000) as i32), Set::I32(behavior::REF_ID, WYLDER_REFUND_BULLET as i32)] },
    // 送葬者6
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: UNDERTAKER_TRANCE_REFUND, name: "[Tiered Buff] Undertaker: Cheaper Enhanced Trance",
        sets: &UNDERTAKER_MARKER },
    RowSpec { table: &SP_EFFECT, template: GAUGE_REFILL_TEMPLATE, id: UNDERTAKER_TRANCE_REFUND_GAUGE, name: "[Tiered Buff] Undertaker: Cheaper Enhanced Trance - Refund",
        sets: &UNDERTAKER_REFUND_GAUGE_SETS },
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: UNDERTAKER_TRANCE_REFUND_PENDING, name: "[Tiered Buff] Undertaker: Cheaper Enhanced Trance - Pending",
        sets: &PENDING },
    RowSpec { table: &BULLET, template: FX_BULLET, id: UNDERTAKER_REFUND_BULLET, name: "[Tiered Buff] Undertaker: Cheaper Enhanced Trance", sets: &UNDERTAKER_BULLET },
    RowSpec { table: &BEHAVIOR_PC, template: LINK_RELIC_BEHAVIOR, id: UNDERTAKER_REFUND_BEHAVIOR, name: "[Tiered Buff] Undertaker: Cheaper Enhanced Trance",
        sets: &[Set::I32(behavior::JUDGE_ID, (UNDERTAKER_REFUND_BEHAVIOR % 1000) as i32), Set::I32(behavior::REF_ID, UNDERTAKER_REFUND_BULLET as i32)] },
    // 执行者7
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: EXECUTOR_ALL_AILMENTS, name: "[Tiered Buff] Executor: Cursed sword inflicts all ailments", sets: &AILMENTS_MARKER },
    ailment!(0, "[Tiered Buff] Executor: Cursed sword inflicts all ailments - Poison"),
    ailment!(1, "[Tiered Buff] Executor: Cursed sword inflicts all ailments - Scarlet Rot"),
    ailment!(2, "[Tiered Buff] Executor: Cursed sword inflicts all ailments - Blood Loss"),
    ailment!(3, "[Tiered Buff] Executor: Cursed sword inflicts all ailments - Frostbite"),
    ailment!(4, "[Tiered Buff] Executor: Cursed sword inflicts all ailments - Sleep"),
    ailment!(5, "[Tiered Buff] Executor: Cursed sword inflicts all ailments - Madness"),
    cursed_ailment_atk!(0, "[Tiered Buff] Executor: Cursed sword inflicts all ailments - Poison / Rot / Blood"),
    cursed_ailment_atk!(1, "[Tiered Buff] Executor: Cursed sword inflicts all ailments - Frost / Sleep / Madness"),
    cursed_ailment_behavior!(0, "[Tiered Buff] Executor: Cursed sword inflicts all ailments - Poison / Rot / Blood"),
    cursed_ailment_behavior!(1, "[Tiered Buff] Executor: Cursed sword inflicts all ailments - Frost / Sleep / Madness"),
    // 执行者8
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: EXECUTOR_RELEASE_INVINCIBLE, name: "[Tiered Buff] Executor: Crucible cursed sword", sets: &INVINCIBLE_MARKER },
    RowSpec { table: &BEHAVIOR_PC, template: LINK_RELIC_BEHAVIOR, id: ORDOVIS_VORTEX_BEHAVIOR[0], name: "[Tiered Buff] Executor: Crucible moveset - Charged Ordovis's Vortex",
        sets: &[Set::I32(behavior::JUDGE_ID, (ORDOVIS_VORTEX_BEHAVIOR[0] % 1000) as i32), Set::I32(behavior::REF_ID, ORDOVIS_VORTEX_BULLET[0] as i32)] },
    RowSpec { table: &BEHAVIOR_PC, template: LINK_RELIC_BEHAVIOR, id: ORDOVIS_VORTEX_BEHAVIOR[1], name: "[Tiered Buff] Executor: Crucible moveset - Ordovis's Vortex",
        sets: &[Set::I32(behavior::JUDGE_ID, (ORDOVIS_VORTEX_BEHAVIOR[1] % 1000) as i32), Set::I32(behavior::REF_ID, ORDOVIS_VORTEX_BULLET[1] as i32)] },
    RowSpec { table: &BEHAVIOR_PC, template: LINK_RELIC_BEHAVIOR, id: SILURIA_VORTEX_BEHAVIOR[0], name: "[Tiered Buff] Executor: Crucible moveset - Charged Siluria's Woe",
        sets: &[Set::I32(behavior::JUDGE_ID, (SILURIA_VORTEX_BEHAVIOR[0] % 1000) as i32), Set::I32(behavior::REF_ID, SILURIA_VORTEX_BULLET[0] as i32)] },
    RowSpec { table: &BEHAVIOR_PC, template: LINK_RELIC_BEHAVIOR, id: SILURIA_VORTEX_BEHAVIOR[1], name: "[Tiered Buff] Executor: Crucible moveset - Siluria's Woe",
        sets: &[Set::I32(behavior::JUDGE_ID, (SILURIA_VORTEX_BEHAVIOR[1] % 1000) as i32), Set::I32(behavior::REF_ID, SILURIA_VORTEX_BULLET[1] as i32)] },
];

pub const BUFF_ROWS: &[RowSpec] = buff_rows![
    (IRONEYE_SKILL_ROT, IRONEYE_SKILL_ROT, ICON_SKILL, RARITY_PURPLE, "Ironeye: Skill inflicts scarlet rot"),
    (IRONEYE_ULT_ROT, IRONEYE_ULT_ROT, ICON_ULTIMATE, RARITY_PURPLE, "Ironeye: Charged Ult inflicts scarlet rot"),
    (SCHOLAR_LINK_AILMENT, SCHOLAR_LINK_AILMENT, ICON_ULTIMATE, RARITY_PURPLE, "Scholar: Linked foes take more ailment damage"),
    (WYLDER_ULT_REFUND, WYLDER_ULT_REFUND, ICON_ULTIMATE, RARITY_PURPLE, "Wylder: Uncharged Ult refunds more gauge"),
    (UNDERTAKER_TRANCE_REFUND, UNDERTAKER_TRANCE_REFUND, ICON_SKILL, RARITY_PURPLE, "Undertaker: Cheaper Enhanced Trance"),
    (EXECUTOR_ALL_AILMENTS, EXECUTOR_ALL_AILMENTS, ICON_SKILL, RARITY_PURPLE, "Executor: Cursed sword inflicts all ailments"),
    (EXECUTOR_RELEASE_INVINCIBLE, EXECUTOR_RELEASE_INVINCIBLE, ICON_SKILL, RARITY_PURPLE, "Executor: Crucible cursed sword"),
];

pub const ROLE: &[RoleBuff] = &[
    RoleBuff { hero: IRONEYE, buff: IRONEYE_SKILL_ROT },
    RoleBuff { hero: IRONEYE, buff: IRONEYE_ULT_ROT },
    RoleBuff { hero: SCHOLAR, buff: SCHOLAR_LINK_AILMENT },
    RoleBuff { hero: WYLDER, buff: WYLDER_ULT_REFUND },
    RoleBuff { hero: UNDERTAKER, buff: UNDERTAKER_TRANCE_REFUND },
    RoleBuff { hero: EXECUTOR, buff: EXECUTOR_ALL_AILMENTS },
    RoleBuff { hero: EXECUTOR, buff: EXECUTOR_RELEASE_INVINCIBLE },
];
