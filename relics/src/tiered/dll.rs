//! 第五批：靠 DLL 实现的锻体（src\tiered_effects）。这里只放标记效果与 DLL 施加的子效果，ID 取自 shared::tiered_runes。
//! - 蓝1 卢恩随机倍数：类别 10 可叠加，每拿一次多一份，DLL 按份数增加触发
//! - 紫13 消耗卢恩时按概率返还；金8 致命伤害时扣卢恩免死、回满血并发出冲击波
//! - 圣杯瓶类（shared::tiered_flasks）：金2 被救起时补满、紫5 瓶满时提升攻击力、蓝3 上限 +1（类别 10 可叠加）
//! - 武器类（shared::tiered_weapons）：金4 每把金色武器伤害 +15%（6 档，DLL 按把数挂）、紫15 白蓝武器升紫（类别 10 可叠加）
//! - 道具类（shared::tiered_items）：金12 持有上限 ×2 并让道具效果扩散到同伴、学者2 持有上限 ×2（DLL 改参数表）
//! - 冲击波与蓝6 倒地火焰爆发（shared::tiered_downed）：照原版 3 级振奋香的追加爆炸，效果（状态 275 + 行为）→ 子弹 → 固定伤害的攻击行
//! - 追踪者12 第六感多 1 次（shared::tiered_sixth_sense）：DLL 在原版「已用」出现后把引擎里的第六感次数改回 1 一次，赐福时补满
//! - 蓝10 在装备界面按热键打开锻造台（shared::tiered_quick_forge）
//! - 金6 冷冽海市蜃楼增强版（shared::tiered_cold_mirage）：复制原版卡莉果入侵奖励的「可用 / 已用」作为自己的一对（DLL 让引擎读这一对），
//!   触发时补满血量并挂 30 秒减伤 / 加攻，另外拿过原版这条时多 1 次
//! - 隐士1 技艺直接放混合魔法（shared::tiered_mixcraft）：DLL 按右手武器写属性槽，放完冷却 8 秒，动作脚本禁掉吸收

use nr_stat_alloc_shared::tiered_cold_mirage::{GOLD_MIRAGE, MIRAGE_BUFF, MIRAGE_HEAL, MIRAGE_READY, MIRAGE_USED, VANILLA_READY, VANILLA_USED};
use nr_stat_alloc_shared::tiered_downed::{DOWNED_BURST, DOWNED_BURST_SPAWN};
use nr_stat_alloc_shared::tiered_flasks::{FLASK_FULL_ATTACK, FLASK_FULL_ATTACK_BUFF, FLASK_PLUS, FLASK_REFILL};
use nr_stat_alloc_shared::tiered_items::{GOLD_ITEMS, SCHOLAR_ITEMS};
use nr_stat_alloc_shared::tiered_mixcraft::DIRECT_COCKTAIL;
use nr_stat_alloc_shared::tiered_quick_forge::QUICK_FORGE;
use nr_stat_alloc_shared::tiered_runes::{RUNE_GAMBLE, RUNE_REFUND, RUNE_SHIELD, RUNE_SHIELD_BLAST, RUNE_SHIELD_HEAL, RUNE_SHIELD_UNDYING};
use nr_stat_alloc_shared::tiered_sixth_sense::EXTRA_SIXTH_SENSE;
use nr_stat_alloc_shared::tiered_weapons::{GOLD_WEAPON_DAMAGE, GOLD_WEAPON_ROWS, PURPLE_UPGRADE};

use crate::layout::{ATK_PC, BEHAVIOR_PC, BULLET, SP_EFFECT, atk, behavior, bullet, sp_effect as sp};
use crate::spec::{RowSpec, Set};

use super::pools::{Candidate, RoleBuff};
use super::rows::{
    ICON_ATTACK, ICON_DISCOVERY, ICON_HP, ICON_RUNES, ICON_SIXTH_SENSE, ICON_SKILL, Icon, MARKER_TEMPLATE, NO_SYNC, RARITY_BLUE, RARITY_GOLD, RARITY_PURPLE, RECLUSE,
    SAVE, SCHOLAR, STACKING, WYLDER, attack, buff_rows, cut, icon, text,
};

const GAMBLE: u32 = RUNE_GAMBLE as u32;
const REFUND: u32 = RUNE_REFUND as u32;
const SHIELD: u32 = RUNE_SHIELD as u32;
const SHIELD_UNDYING: u32 = RUNE_SHIELD_UNDYING as u32;
const SHIELD_HEAL: u32 = RUNE_SHIELD_HEAL as u32;
const SHIELD_BLAST: u32 = RUNE_SHIELD_BLAST as u32;
const BURST: u32 = DOWNED_BURST as u32;
const BURST_SPAWN: u32 = DOWNED_BURST_SPAWN as u32;
const REFILL: u32 = FLASK_REFILL as u32;
const FULL_ATTACK: u32 = FLASK_FULL_ATTACK as u32;
const FULL_ATTACK_BUFF: u32 = FLASK_FULL_ATTACK_BUFF as u32;
const PLUS: u32 = FLASK_PLUS as u32;
const GOLD_WEAPONS: u32 = GOLD_WEAPON_DAMAGE as u32;
const UPGRADE: u32 = PURPLE_UPGRADE as u32;
const ITEMS: u32 = GOLD_ITEMS as u32;
const SCHOLAR_LIMIT: u32 = SCHOLAR_ITEMS as u32;
const SIXTH_SENSE_PLUS: u32 = EXTRA_SIXTH_SENSE as u32;
const FORGE: u32 = QUICK_FORGE as u32;
const COCKTAIL: u32 = DIRECT_COCKTAIL as u32;
const MIRAGE: u32 = GOLD_MIRAGE as u32;
const MIRAGE_HEAL_ROW: u32 = MIRAGE_HEAL as u32;
const MIRAGE_BUFF_ROW: u32 = MIRAGE_BUFF as u32;
const MIRAGE_READY_ROW: u32 = MIRAGE_READY as u32;
const MIRAGE_USED_ROW: u32 = MIRAGE_USED as u32;
/// 金6 那一对效果在类别 200 里的独占优先级（原版那一对是 220）：互相顶替，但不碰原版的「可用」
const MIRAGE_PRIORITY: u8 = 223;
/// 原版「冷冽海市蜃楼」的图标（锻体 21724，效果 20724）
const ICON_MIRAGE: Icon = Icon { goods: 20_724, buff: 21_724, status: 20_724 };
/// 金6 触发后增益：30 秒、减伤 30%、攻击 +15%
const MIRAGE_BUFF_SECONDS: f32 = 30.0;
const MIRAGE_CUT: f32 = 0.7;
const MIRAGE_ATTACK: f32 = 1.15;

/// 执行者兽化期间的不死（状态 143，血量不会低于 1）
const IMMORTALITY_TEMPLATE: u32 = 707_110;
/// 回复 50% 血量的效果（原带条件状态 2042，清掉）
const HEAL_TEMPLATE: u32 = 7_090_303;
/// 隐士遗物「发动绝招时提升攻击力」的 30 秒增益（带状态栏图标与文本）
const TIMED_BUFF_TEMPLATE: u32 = 7_032_903;
/// 攻击力增益的状态栏图标
const ATTACK_UP_STATUS: i32 = 20_406;
/// 3 级振奋香的追加爆炸：发出效果（0.1 秒，状态 275，挂点 220）、行为、子弹（半径 1.6 → 4、火焰爆炸特效）与攻击行（固定火 490）
const BLAST_SPAWN_TEMPLATE: u32 = 708_696;
const BLAST_BEHAVIOR_TEMPLATE: u32 = 760_000_110;
const BLAST_TEMPLATE: u32 = 13_350_050;
/// 冲击波 / 火焰爆发的行为、子弹、攻击行编号（三张表同号）
const SHIELD_BLAST_BULLET: u32 = 900_997_115;
const BURST_BULLET: u32 = 900_997_116;

const SELF: Set = Set::Bit(sp::TARGET_BYTE, sp::SELF_TARGET_BIT, true);

/// 金4 的第 k 档（DLL 按金色武器把数挂前 k 档）：倍率 (1+0.15k)/(1+0.15(k-1))，前 k 档叠起来正好 +15% × k
macro_rules! gold_weapon_row {
    ($k:literal) => {
        RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: GOLD_WEAPON_ROWS[$k - 1] as u32,
            name: concat!("[Tiered Buff] Damage up per gold armament x", $k),
            sets: &[Set::F32(sp::EFFECT_ENDURANCE, -1.0), attack(0, (1.0 + 0.15 * $k as f32) / (1.0 + 0.15 * ($k - 1) as f32)),
                attack(1, (1.0 + 0.15 * $k as f32) / (1.0 + 0.15 * ($k - 1) as f32)),
                attack(2, (1.0 + 0.15 * $k as f32) / (1.0 + 0.15 * ($k - 1) as f32)),
                attack(3, (1.0 + 0.15 * $k as f32) / (1.0 + 0.15 * ($k - 1) as f32)),
                attack(4, (1.0 + 0.15 * $k as f32) / (1.0 + 0.15 * ($k - 1) as f32)), SELF, NO_SYNC] }
    };
}

pub const ROWS: &[RowSpec] = &[
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: GAMBLE, name: "[Tiered Buff] Runes randomly multiplied",
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, -1.0), Set::U16(sp::SP_CATEGORY, STACKING), SAVE, icon(ICON_RUNES), text(GAMBLE)] },
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: REFUND, name: "[Tiered Buff] Spent runes may be refunded",
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, -1.0), SAVE, icon(ICON_RUNES), text(REFUND)] },
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: SHIELD, name: "[Tiered Buff] Runes avert lethal damage",
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, -1.0), SAVE, icon(ICON_HP), text(SHIELD)] },
    // DLL 在卢恩够时每帧补挂，不同步给队友
    RowSpec { table: &SP_EFFECT, template: IMMORTALITY_TEMPLATE, id: SHIELD_UNDYING, name: "[Tiered Buff] Runes avert lethal damage - Undying",
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, 0.5), cut(0, 1.0), cut(1, 1.0), cut(2, 1.0), cut(3, 1.0), cut(4, 1.0), cut(5, 1.0), cut(6, 1.0),
            cut(7, 1.0), SELF, NO_SYNC] },
    RowSpec { table: &SP_EFFECT, template: HEAL_TEMPLATE, id: SHIELD_HEAL, name: "[Tiered Buff] Runes avert lethal damage - Heal",
        sets: &[Set::F32(sp::CHANGE_HP_RATE, -100.0), Set::U16(sp::INVOCATION_STATE_1, 0), SELF] },
    // 冲击波：物理 400、削韧 150、击退 4 米，半径扩到 8 米
    RowSpec { table: &SP_EFFECT, template: BLAST_SPAWN_TEMPLATE, id: SHIELD_BLAST, name: "[Tiered Buff] Runes avert lethal damage - Shockwave",
        sets: &[Set::I32(sp::BEHAVIOR_ID, SHIELD_BLAST_BULLET as i32), Set::U16(sp::SP_CATEGORY, 20), Set::U8(sp::CATEGORY_PRIORITY, 0)] },
    RowSpec { table: &BEHAVIOR_PC, template: BLAST_BEHAVIOR_TEMPLATE, id: SHIELD_BLAST_BULLET, name: "[Tiered Buff] Runes avert lethal damage - Shockwave",
        sets: &[Set::I32(behavior::REF_ID, SHIELD_BLAST_BULLET as i32)] },
    RowSpec { table: &BULLET, template: BLAST_TEMPLATE, id: SHIELD_BLAST_BULLET, name: "[Tiered Buff] Runes avert lethal damage - Shockwave",
        sets: &[Set::I32(bullet::ATK_ID, SHIELD_BLAST_BULLET as i32), Set::F32(bullet::HIT_RADIUS, 2.0), Set::F32(bullet::HIT_RADIUS_MAX, 8.0)] },
    RowSpec { table: &ATK_PC, template: BLAST_TEMPLATE, id: SHIELD_BLAST_BULLET, name: "[Tiered Buff] Runes avert lethal damage - Shockwave",
        sets: &[Set::U16(atk::ATK_FIRE, 0), Set::U16(atk::ATK_PHYS, 400), Set::F32(atk::ATK_SUPER_ARMOR, 150.0), Set::F32(atk::KNOCKBACK_DIST, 4.0),
            Set::U8(atk::SP_ATTRIBUTE, 0)] },
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: REFILL, name: "[Tiered Buff] Flasks refilled when revived",
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, -1.0), SAVE, icon(ICON_HP), text(REFILL)] },
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: FULL_ATTACK, name: "[Tiered Buff] Attack up while flasks are full",
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, -1.0), SAVE, icon(ICON_ATTACK), text(FULL_ATTACK)] },
    // 不限时：DLL 瓶满时挂上、不满时移除（短时补挂会让状态栏图标一直闪）
    RowSpec { table: &SP_EFFECT, template: TIMED_BUFF_TEMPLATE, id: FULL_ATTACK_BUFF, name: "[Tiered Buff] Attack up while flasks are full - Buff",
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, -1.0), attack(0, 1.15), attack(1, 1.15), attack(2, 1.15), attack(3, 1.15), attack(4, 1.15),
            Set::U16(sp::SP_CATEGORY, 20), Set::I32(sp::VFX_ID, -1), Set::I32(sp::ICON_ID, ATTACK_UP_STATUS), text(FULL_ATTACK_BUFF), SELF, NO_SYNC] },
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: PLUS, name: "[Tiered Buff] +1 Flask charge",
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, -1.0), Set::U16(sp::SP_CATEGORY, STACKING), SAVE, icon(ICON_HP), text(PLUS)] },
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: GOLD_WEAPONS, name: "[Tiered Buff] Damage up per gold armament",
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, -1.0), SAVE, icon(ICON_ATTACK), text(GOLD_WEAPONS)] },
    gold_weapon_row!(1),
    gold_weapon_row!(2),
    gold_weapon_row!(3),
    gold_weapon_row!(4),
    gold_weapon_row!(5),
    gold_weapon_row!(6),
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: UPGRADE, name: "[Tiered Buff] White and blue armaments become purple",
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, -1.0), Set::U16(sp::SP_CATEGORY, STACKING), SAVE, icon(ICON_ATTACK), text(UPGRADE)] },
    // 蓝6：火 900、削韧 60，半径扩到 6 米
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: BURST, name: "[Tiered Buff] Fiery burst when downed",
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, -1.0), SAVE, icon(ICON_HP), text(BURST)] },
    RowSpec { table: &SP_EFFECT, template: BLAST_SPAWN_TEMPLATE, id: BURST_SPAWN, name: "[Tiered Buff] Fiery burst when downed - Spawn",
        sets: &[Set::I32(sp::BEHAVIOR_ID, BURST_BULLET as i32), Set::U16(sp::SP_CATEGORY, 20), Set::U8(sp::CATEGORY_PRIORITY, 0)] },
    RowSpec { table: &BEHAVIOR_PC, template: BLAST_BEHAVIOR_TEMPLATE, id: BURST_BULLET, name: "[Tiered Buff] Fiery burst when downed",
        sets: &[Set::I32(behavior::REF_ID, BURST_BULLET as i32)] },
    RowSpec { table: &BULLET, template: BLAST_TEMPLATE, id: BURST_BULLET, name: "[Tiered Buff] Fiery burst when downed",
        sets: &[Set::I32(bullet::ATK_ID, BURST_BULLET as i32), Set::F32(bullet::HIT_RADIUS, 2.0), Set::F32(bullet::HIT_RADIUS_MAX, 6.0)] },
    RowSpec { table: &ATK_PC, template: BLAST_TEMPLATE, id: BURST_BULLET, name: "[Tiered Buff] Fiery burst when downed",
        sets: &[Set::U16(atk::ATK_FIRE, 900), Set::F32(atk::ATK_SUPER_ARMOR, 60.0)] },
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: ITEMS, name: "[Tiered Buff] Item capacity up, items affect allies",
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, -1.0), SAVE, icon(ICON_DISCOVERY), text(ITEMS)] },
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: SCHOLAR_LIMIT, name: "[Tiered Buff] Scholar: Item capacity doubled",
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, -1.0), SAVE, icon(ICON_DISCOVERY), text(SCHOLAR_LIMIT)] },
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: SIXTH_SENSE_PLUS, name: "[Tiered Buff] Wylder: +1 Sixth Sense use",
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, -1.0), SAVE, icon(ICON_SIXTH_SENSE), text(SIXTH_SENSE_PLUS)] },
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: FORGE, name: "[Tiered Buff] Open the forge from the equipment menu",
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, -1.0), SAVE, icon(ICON_DISCOVERY), text(FORGE)] },
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: COCKTAIL, name: "[Tiered Buff] Recluse: Skill casts Magic Cocktails directly",
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, -1.0), SAVE, icon(ICON_SKILL), text(COCKTAIL)] },
    // 金6：标记不显示状态栏图标（原版「冷冽海市蜃楼」的可用 / 已用自带图标）
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: MIRAGE, name: "[Tiered Buff] Cold Mirage, empowered",
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, -1.0), SAVE, Set::I32(sp::ICON_ID, -1), text(MIRAGE)] },
    RowSpec { table: &SP_EFFECT, template: VANILLA_READY as u32, id: MIRAGE_READY_ROW, name: "[Tiered Buff] Cold Mirage, empowered - Ready",
        sets: &[Set::U8(sp::CATEGORY_PRIORITY, MIRAGE_PRIORITY)] },
    RowSpec { table: &SP_EFFECT, template: VANILLA_USED as u32, id: MIRAGE_USED_ROW, name: "[Tiered Buff] Cold Mirage, empowered - Used",
        sets: &[Set::U8(sp::CATEGORY_PRIORITY, MIRAGE_PRIORITY)] },
    RowSpec { table: &SP_EFFECT, template: HEAL_TEMPLATE, id: MIRAGE_HEAL_ROW, name: "[Tiered Buff] Cold Mirage, empowered - Heal",
        sets: &[Set::F32(sp::CHANGE_HP_RATE, -100.0), Set::U16(sp::INVOCATION_STATE_1, 0), SELF] },
    RowSpec { table: &SP_EFFECT, template: TIMED_BUFF_TEMPLATE, id: MIRAGE_BUFF_ROW, name: "[Tiered Buff] Cold Mirage, empowered - Buff",
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, MIRAGE_BUFF_SECONDS), attack(0, MIRAGE_ATTACK), attack(1, MIRAGE_ATTACK), attack(2, MIRAGE_ATTACK),
            attack(3, MIRAGE_ATTACK), attack(4, MIRAGE_ATTACK), cut(0, MIRAGE_CUT), cut(1, MIRAGE_CUT), cut(2, MIRAGE_CUT), cut(3, MIRAGE_CUT),
            cut(4, MIRAGE_CUT), cut(5, MIRAGE_CUT), cut(6, MIRAGE_CUT), cut(7, MIRAGE_CUT), Set::U16(sp::SP_CATEGORY, 20), Set::I32(sp::VFX_ID, -1),
            Set::I32(sp::ICON_ID, ATTACK_UP_STATUS), text(MIRAGE_BUFF_ROW), SELF, NO_SYNC] },
];

pub const BUFF_ROWS: &[RowSpec] = buff_rows![
    (GAMBLE, GAMBLE, ICON_RUNES, RARITY_BLUE, "Runes randomly multiplied"),
    (REFUND, REFUND, ICON_RUNES, RARITY_PURPLE, "Spent runes may be refunded"),
    (SHIELD, SHIELD, ICON_HP, RARITY_GOLD, "Runes avert lethal damage"),
    (REFILL, REFILL, ICON_HP, RARITY_GOLD, "Flasks refilled when revived"),
    (FULL_ATTACK, FULL_ATTACK, ICON_ATTACK, RARITY_PURPLE, "Attack up while flasks are full"),
    (PLUS, PLUS, ICON_HP, RARITY_BLUE, "+1 Flask charge"),
    (GOLD_WEAPONS, GOLD_WEAPONS, ICON_ATTACK, RARITY_GOLD, "Damage up per gold armament"),
    (UPGRADE, UPGRADE, ICON_ATTACK, RARITY_PURPLE, "White and blue armaments become purple"),
    (BURST, BURST, ICON_HP, RARITY_BLUE, "Fiery burst when downed"),
    (ITEMS, ITEMS, ICON_DISCOVERY, RARITY_GOLD, "Item capacity up, items affect allies"),
    (SCHOLAR_LIMIT, SCHOLAR_LIMIT, ICON_DISCOVERY, RARITY_PURPLE, "Scholar: Item capacity doubled"),
    (SIXTH_SENSE_PLUS, SIXTH_SENSE_PLUS, ICON_SIXTH_SENSE, RARITY_PURPLE, "Wylder: +1 Sixth Sense use"),
    (MIRAGE, MIRAGE, ICON_MIRAGE, RARITY_GOLD, "Cold Mirage, empowered"),
    (FORGE, FORGE, ICON_DISCOVERY, RARITY_BLUE, "Open the forge from the equipment menu"),
    (COCKTAIL, COCKTAIL, ICON_SKILL, RARITY_PURPLE, "Recluse: Skill casts Magic Cocktails directly"),
];

pub const BLUE: &[Candidate] = &[Candidate::Buff(GAMBLE), Candidate::Buff(PLUS), Candidate::Buff(BURST), Candidate::Buff(FORGE)];
pub const PURPLE: &[Candidate] = &[Candidate::Buff(REFUND), Candidate::Buff(FULL_ATTACK), Candidate::Buff(UPGRADE)];
pub const GOLD: &[Candidate] =
    &[Candidate::Buff(SHIELD), Candidate::Buff(REFILL), Candidate::Buff(GOLD_WEAPONS), Candidate::Buff(ITEMS), Candidate::Buff(MIRAGE)];
pub const ROLE: &[RoleBuff] =
    &[RoleBuff { hero: SCHOLAR, buff: SCHOLAR_LIMIT }, RoleBuff { hero: WYLDER, buff: SIXTH_SENSE_PLUS }, RoleBuff { hero: RECLUSE, buff: COCKTAIL }];
