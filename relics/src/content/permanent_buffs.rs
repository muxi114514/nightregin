//! 首领球锻体（PermanentBuffParam）改效果：锻体 → SpEffect，名字与说明取 SpEffect 的 TEXT_ID_1（PermanentBuffName / Info）。
//! 这些 SpEffect 都与同号的武器词条共用（AttachEffect 的常驻效果），所以一律新建效果行（9970000 段），再让锻体行原地改指向，武器词条不受影响。
//! 只改数值的沿用原文本 ID，改了效果的用新文本（tools\relic_assets.ps1）。
//! 「持有卢恩提升攻击力」只在效果上带状态值，增伤在 DLL（src\rune_attack.rs）。

use nr_stat_alloc_shared::rune_attack;

use crate::layout::{PERMANENT_BUFF, SP_EFFECT, permanent_buff as pb, sp_effect as sp};
use crate::pool::PoolMove;
use crate::spec::{RowSpec, Set};

// 原版锻体（同号的 SpEffect 是各自原来的效果，作模板）
const POISE_1: u32 = 8_230_000;
const POISE_2: u32 = 8_230_001;
const ART_GAUGE: u32 = 8_360_000;
const ANTI_DRAGON: u32 = 8_330_700;
const THRUST_COUNTER: u32 = 8_430_000;
const CRIT_HP: u32 = 8_640_000;
const CRIT_FP: u32 = 8_640_100;
const POISON_ROT_HEAL: u32 = 8_700_002;
const STAR_RAIN: u32 = 8_460_400;
const SUDDEN_DEATH: u32 = 8_600_300;

// 模板
/// 「Prevent all Stagger」：类别 1001，受击全部替换为不硬直（战技「忍耐」的霸体部分与它相同）
const NO_STAGGER_TEMPLATE: u32 = 5_300;
/// 大山羊护符的免打断效果（content::bull_goat_talisman 新增，最高优先级）
const BULL_GOAT_NO_STAGGER: i32 = 312_102;
/// 「山顶的恩惠」：常驻、对魔法祷告也生效的弱点特攻（原版 C 1.05 / D 1.1）
const WEAK_DAMAGE_TEMPLATE: u32 = 6_999_320;
/// 武器词条「提升战技攻击力」一级（取原版的 ×1.15 作模板，过滤分类改成近战攻击）
const SKILL_ATTACK_TEMPLATE: u32 = 8_350_000;
/// 攻击分类 130「近战攻击」：武器的近战攻击（原版 35 个效果用它过滤），法术、战技里的魔法不算
const MELEE_ATTACK_CATEGORY: u8 = 130;
/// 祷告「恩惠赐福」：每秒回 8 血、90 秒，再次施加刷新时间
const BLESSINGS_BOON: i32 = 1_643_000;
/// 原版锻体效果的图标（攻击类 / 对龙 / 利普拉增益）
const ATTACK_ICON: i32 = 22_206;
const DRAGON_ICON: i32 = 22_207;
const LIBRA_MENU_ICON: i32 = 21_723;

// 新效果（同号作文本 ID）
const UNFLINCHING: u32 = 9_970_000;
const UNFLINCHING_STAGGER: u32 = 9_970_001;
const UNFLINCHING_GUARD: u32 = 9_970_010;
const ART_GAUGE_10: u32 = 9_970_020;
const DRAGON_DAMAGE: u32 = 9_970_030;
const MELEE_ATTACK: u32 = 9_970_040;
const CRIT_HP_50: [u32; 3] = [9_970_050, 9_970_051, 9_970_052];
const CRIT_FP_100: [u32; 3] = [9_970_060, 9_970_061, 9_970_062];
const VICINITY_BOON: u32 = 9_970_070;
const RUNE_ATTACK: u32 = 9_970_080;
const DEATHBORN_BANE: u32 = 9_970_090;

/// 免硬直效果的施加间隔与持续时间（同大山羊护符）
const CYCLE_SECONDS: f32 = 0.05;
const NO_STAGGER_SECONDS: f32 = 0.2;
const GUARD_DAMAGE_CUT: f32 = 0.9;

const fn text(id: u32) -> Set {
    Set::I32(sp::TEXT_ID_1, id as i32)
}

const fn cut(i: usize) -> Set {
    Set::F32(sp::DAMAGE_CUT_RATES[i], GUARD_DAMAGE_CUT)
}

const fn melee_rate(i: usize) -> Set {
    Set::F32(sp::ATTACK_RATES[i], 1.15)
}

pub const ROWS: &[RowSpec] = &[
    // 强韧度一级：只免硬直（原版强韧度 ×0.75 去掉）
    RowSpec { table: &SP_EFFECT, template: POISE_1, id: UNFLINCHING, name: "[Permanent Buff] Unflinching",
        sets: &[Set::F32(sp::TOUGHNESS_DAMAGE_CUT_RATE, 1.0), Set::F32(sp::MOTION_INTERVAL, CYCLE_SECONDS),
            Set::I32(sp::CYCLE_OCCURRENCE, UNFLINCHING_STAGGER as i32), text(UNFLINCHING)] },
    // 高频施加的效果不同步给队友（受击由角色主人本机结算）
    RowSpec { table: &SP_EFFECT, template: NO_STAGGER_TEMPLATE, id: UNFLINCHING_STAGGER, name: "[Permanent Buff] Unflinching - Prevent all Stagger",
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, NO_STAGGER_SECONDS), Set::Bit(sp::NET_SYNC_BYTE, sp::NET_SYNC_DISABLE_BIT, true)] },
    // 强韧度二级：大山羊护符同款免打断 + 受到的伤害 ×0.9
    RowSpec { table: &SP_EFFECT, template: POISE_2, id: UNFLINCHING_GUARD, name: "[Permanent Buff] Unflinching + Damage Negation",
        sets: &[Set::F32(sp::TOUGHNESS_DAMAGE_CUT_RATE, 0.0), Set::F32(sp::MOTION_INTERVAL, CYCLE_SECONDS),
            Set::I32(sp::CYCLE_OCCURRENCE, BULL_GOAT_NO_STAGGER), cut(0), cut(1), cut(2), cut(3), cut(4), cut(5), cut(6), cut(7),
            text(UNFLINCHING_GUARD)] },
    RowSpec { table: &SP_EFFECT, template: ART_GAUGE, id: ART_GAUGE_10, name: "[Permanent Buff] Faster Art Gauge Buildup 10%",
        sets: &[Set::F32(sp::ULTIMATE_ART_GAUGE_RATE, 1.1)] },
    // 对龙：数值同龙飨武器效果（古龙类 ×1.2、龙类 ×1.3），改为常驻、不限武器
    RowSpec { table: &SP_EFFECT, template: WEAK_DAMAGE_TEMPLATE, id: DRAGON_DAMAGE, name: "[Permanent Buff] Improved Damage against Dragons",
        sets: &[Set::F32(sp::WEAK_DMG_RATE_C, 1.2), Set::F32(sp::WEAK_DMG_RATE_D, 1.3), Set::I32(sp::ICON_ID, DRAGON_ICON), text(DRAGON_DAMAGE)] },
    RowSpec { table: &SP_EFFECT, template: SKILL_ATTACK_TEMPLATE, id: MELEE_ATTACK, name: "[Permanent Buff] Improved Melee Attack Power",
        sets: &[melee_rate(0), melee_rate(1), melee_rate(2), melee_rate(3), melee_rate(4), Set::U8(sp::MAGIC_SUB_CATEGORY[0], MELEE_ATTACK_CATEGORY),
            Set::U8(sp::MAGIC_SUB_CATEGORY[1], 0), Set::I32(sp::ICON_ID, ATTACK_ICON), Set::U8(sp::SAVE_CATEGORY, 10), text(MELEE_ATTACK)] },
    // 致命一击回复：触发 → 0.8 秒后换成回复效果，链路照抄原版
    RowSpec { table: &SP_EFFECT, template: CRIT_HP, id: CRIT_HP_50[0], name: "[Permanent Buff] Critical Hit HP Restoration 50%",
        sets: &[Set::I32(sp::CYCLE_OCCURRENCE, CRIT_HP_50[1] as i32)] },
    RowSpec { table: &SP_EFFECT, template: CRIT_HP + 1, id: CRIT_HP_50[1], name: "[Permanent Buff] Critical Hit HP Restoration 50%",
        sets: &[Set::I32(sp::REPLACE_SP_EFFECT, CRIT_HP_50[2] as i32)] },
    RowSpec { table: &SP_EFFECT, template: CRIT_HP + 2, id: CRIT_HP_50[2], name: "[Permanent Buff] Critical Hit HP Restoration 50%",
        sets: &[Set::F32(sp::CHANGE_HP_RATE, -50.0)] },
    RowSpec { table: &SP_EFFECT, template: CRIT_FP, id: CRIT_FP_100[0], name: "[Permanent Buff] Critical Hit FP Restoration 100%",
        sets: &[Set::I32(sp::CYCLE_OCCURRENCE, CRIT_FP_100[1] as i32)] },
    RowSpec { table: &SP_EFFECT, template: CRIT_FP + 1, id: CRIT_FP_100[1], name: "[Permanent Buff] Critical Hit FP Restoration 100%",
        sets: &[Set::I32(sp::REPLACE_SP_EFFECT, CRIT_FP_100[2] as i32)] },
    RowSpec { table: &SP_EFFECT, template: CRIT_FP + 2, id: CRIT_FP_100[2], name: "[Permanent Buff] Critical Hit FP Restoration 100%",
        sets: &[Set::F32(sp::CHANGE_MP_RATE, -100.0)] },
    // 原版条件（状态 380 / 2064 = 周围有人中毒 / 腐败）不变，触发的回血换成恩惠赐福本身
    RowSpec { table: &SP_EFFECT, template: POISON_ROT_HEAL, id: VICINITY_BOON, name: "[Permanent Buff] Poison & Rot in Vicinity Grant Blessing's Boon",
        sets: &[Set::I32(sp::CYCLE_OCCURRENCE, BLESSINGS_BOON), text(VICINITY_BOON)] },
    RowSpec { table: &SP_EFFECT, template: STAR_RAIN, id: RUNE_ATTACK, name: "[Permanent Buff] Attack Power Rises with Runes Held",
        sets: &[Set::U16(sp::STATE_INFO, rune_attack::STATE), Set::I32(sp::ICON_ID, ATTACK_ICON), text(RUNE_ATTACK)] },
    // 对死诞者 ×2，同圣律（原版「死诞者被打败后不会复活」的来源共有的只有这一项）
    RowSpec { table: &SP_EFFECT, template: WEAK_DAMAGE_TEMPLATE, id: DEATHBORN_BANE, name: "[Permanent Buff] Deathborn Cannot Revive",
        sets: &[Set::F32(sp::WEAK_DMG_RATE_B, 2.0), Set::F32(sp::WEAK_DMG_RATE_C, 1.0), Set::F32(sp::WEAK_DMG_RATE_D, 1.0), text(DEATHBORN_BANE)] },
    // 锻体行原地改指向（见 content::IN_PLACE）
    RowSpec { table: &PERMANENT_BUFF, template: POISE_1, id: POISE_1, name: "Unflinching",
        sets: &[sp_of(UNFLINCHING), Set::U16(pb::DISPLAY_NUMERIC, pb::DISPLAY_NONE), Set::U16(pb::DISPLAY_PERCENT, 0), value(0)] },
    RowSpec { table: &PERMANENT_BUFF, template: POISE_2, id: POISE_2, name: "Unflinching + Damage Negation (10%)",
        sets: &[sp_of(UNFLINCHING_GUARD), value(10)] },
    RowSpec { table: &PERMANENT_BUFF, template: ART_GAUGE, id: ART_GAUGE, name: "Faster Art Gauge Buildup (+10%)",
        sets: &[sp_of(ART_GAUGE_10), value(10)] },
    RowSpec { table: &PERMANENT_BUFF, template: ANTI_DRAGON, id: ANTI_DRAGON, name: "Improved Damage against Dragons",
        sets: &[sp_of(DRAGON_DAMAGE)] },
    RowSpec { table: &PERMANENT_BUFF, template: THRUST_COUNTER, id: THRUST_COUNTER, name: "Improved Melee Attack Power (+15%)",
        sets: &[sp_of(MELEE_ATTACK), value(15)] },
    RowSpec { table: &PERMANENT_BUFF, template: CRIT_HP, id: CRIT_HP, name: "Critical Hit HP Restoration (50%)",
        sets: &[sp_of(CRIT_HP_50[0]), value(50)] },
    RowSpec { table: &PERMANENT_BUFF, template: CRIT_FP, id: CRIT_FP, name: "Critical Hit FP Restoration (100%)",
        sets: &[sp_of(CRIT_FP_100[0]), value(100)] },
    RowSpec { table: &PERMANENT_BUFF, template: POISON_ROT_HEAL, id: POISON_ROT_HEAL, name: "Poison & Rot in Vicinity Grant Blessing's Boon",
        sets: &[sp_of(VICINITY_BOON), Set::U16(pb::DISPLAY_NUMERIC, pb::DISPLAY_NONE), value(0)] },
    RowSpec { table: &PERMANENT_BUFF, template: STAR_RAIN, id: STAR_RAIN, name: "Attack Power Rises with Runes Held",
        sets: &[sp_of(RUNE_ATTACK), Set::I32(pb::ICON_ID, LIBRA_MENU_ICON)] },
    RowSpec { table: &PERMANENT_BUFF, template: SUDDEN_DEATH, id: SUDDEN_DEATH, name: "Deathborn Cannot Revive",
        sets: &[sp_of(DEATHBORN_BANE)] },
];

/// 原地改写的锻体行
pub const IN_PLACE: [u32; 10] =
    [POISE_1, POISE_2, ART_GAUGE, ANTI_DRAGON, THRUST_COUNTER, CRIT_HP, CRIT_FP, POISON_ROT_HEAL, STAR_RAIN, SUDDEN_DEATH];

/// 骤死（现为「死诞者无法复活」）从野外首领第三选项（4200200）与第一夜首领 / 高阶封印监牢（4200220）的特殊池移到小首领普通球（4200100）；
/// 强韧度一级（「动作不会被打断」）不再掉落（普通池与没人引用的全量池 4200000 都删），只留二级
pub const POOL_MOVES: &[PoolMove] = &[
    PoolMove { buff: SUDDEN_DEATH, from: &[4_200_200, 4_200_220], to: Some(4_200_100) },
    PoolMove { buff: POISE_1, from: &[4_200_100, 4_200_000], to: None },
];

const fn sp_of(id: u32) -> Set {
    Set::I32(pb::SP_EFFECT, id as i32)
}

const fn value(v: i32) -> Set {
    Set::I32(pb::NUMERIC_VALUE, v)
}
