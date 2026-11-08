//! 大山羊护符（护符 1210）：原版「提升强韧度」= 词条 312100 → 常驻效果 312100（受到的韧性伤害 ×0.75）；改为除投技 / 抓取以外的攻击都打不断动作。
//! 护符表的效果字段填的是词条（AttachEffectParam）ID，不是 SpEffect ID，所以要新建词条再让护符指向它。
//! 只把韧性伤害改成 ×0 挡不住受击动作：受击动作由类别 1001 的受击等级替换决定，原版免硬直效果
//! （战技「忍耐」1650、5300「Prevent all Stagger」）都是把 12 档受击全部替换为 1（不硬直）。照搬铁壶香药的施加方式：
//! 常驻效果每 0.05 秒施加一次 0.2 秒的受击替换效果；类别 1001 只取优先级最高的一条，用最高优先级压过其它替换。
//! 投技 / 抓取不走受击等级，不受影响。词条 312100 也能加在遗物上（遗物编辑器里有），所以复制成新词条只给护符用，原行不动。
//! 护符说明与新词条文本由 tools\relic_assets.ps1 处理。

use crate::layout::{ATTACH_EFFECT, EQUIP_ACCESSORY, SP_EFFECT, attach_effect as ae, equip_accessory as acc, sp_effect as sp};
use crate::spec::{RowSpec, Set};

/// 原地修改的护符行（见 content::IN_PLACE）
pub const TALISMAN: u32 = 1_210;
/// 原版词条与常驻效果（同号）
const POISE: u32 = 312_100;
/// 新词条、新常驻效果与新文本（同号，同原版写法）
const UNFLINCHING: u32 = 312_101;
const NO_STAGGER_EFFECT: u32 = 312_102;
/// 原版「Prevent all Stagger」：类别 1001，12 档受击全部替换为不硬直
const NO_STAGGER_TEMPLATE: u32 = 5_300;
/// 施加间隔与持续时间同铁壶香药（503520 → 503523）
const CYCLE_SECONDS: f32 = 0.05;
const NO_STAGGER_SECONDS: f32 = 0.2;
const TOP_PRIORITY: u8 = u8::MAX;

pub const ROWS: &[RowSpec] = &[
    RowSpec { table: &ATTACH_EFFECT, template: POISE, id: UNFLINCHING, name: "[Talisman] Bull-Goat's Talisman - Unflinching",
        sets: &[Set::I32(ae::PASSIVE_SP_EFFECT, UNFLINCHING as i32), Set::I32(ae::TEXT_ID, UNFLINCHING as i32)] },
    RowSpec { table: &SP_EFFECT, template: POISE, id: UNFLINCHING, name: "[Talisman] Bull-Goat's Talisman - Unflinching",
        sets: &[Set::F32(sp::TOUGHNESS_DAMAGE_CUT_RATE, 0.0), Set::F32(sp::MOTION_INTERVAL, CYCLE_SECONDS),
            Set::I32(sp::CYCLE_OCCURRENCE, NO_STAGGER_EFFECT as i32)] },
    // 高频施加的效果不同步给队友（受击由角色主人本机结算）
    RowSpec { table: &SP_EFFECT, template: NO_STAGGER_TEMPLATE, id: NO_STAGGER_EFFECT, name: "[Talisman] Bull-Goat's Talisman - Prevent all Stagger",
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, NO_STAGGER_SECONDS), Set::U8(sp::CATEGORY_PRIORITY, TOP_PRIORITY),
            Set::Bit(sp::NET_SYNC_BYTE, sp::NET_SYNC_DISABLE_BIT, true)] },
    RowSpec { table: &EQUIP_ACCESSORY, template: TALISMAN, id: TALISMAN, name: "[Uncommon] Bull-Goat's Talisman",
        sets: &[Set::I32(acc::ATTACH_EFFECT_1, UNFLINCHING as i32)] },
];
