//! 三档锻体与角色专属锻体的效果、锻体行与候选清单。效果以原版（或 MMV）同类效果为模板复制，同号作文本 ID；
//! 可叠加的用类别 10（同原版血量 +10% 等数值锻体，拿几次叠几次），不叠加的用类别 20。
//! 追踪者元素锻体直接指向已做好的词条标记，遗物版的常驻效果清空（保留行，免得存档里的遗物出错）。

use crate::layout::{ATTACH_EFFECT, EQUIP_GOODS, SP_EFFECT, attach_effect as ae, equip_goods as goods, sp_effect as sp};
use crate::spec::{Patch, RowSpec, Set};

use super::rows::{
    DUCHESS, EXECUTOR, GUARDIAN, ICON_AGGRO, ICON_ATTACK, ICON_DISCOVERY, ICON_HP, ICON_NEGATION, ICON_RUNES, ICON_SKILL, ICON_ULTIMATE, IRONEYE, MARKER_TEMPLATE,
    NON_STACKING, NO_SYNC,
    ON_HIT_RESTORE_TEMPLATE, ON_HIT_TEMPLATE, PERIODIC_TEMPLATE, RAIDER, RARITY_BLUE, RARITY_GOLD, RARITY_PURPLE, RECLUSE, REVENANT, SAVE,
    STACKING, UNDERTAKER, WYLDER, buff_rows, cut, icon, text,
};
use super::pools::{Candidate, RoleBuff};

// 蓝
const DISCOVERY_20: u32 = 9_971_000;
const RAIN_GUARD: u32 = 9_971_010;
// 紫
const SKILL_USE: u32 = 9_971_100;
const DISCOVERY_40: u32 = 9_971_110;
const GLASS_CANNON: u32 = 9_971_120;
const GLASS_CANNON_HEAL: u32 = 9_971_121;
/// 紫10：自己施加的全部异常累积 ×1.25（类别 10 可叠加，各份相乘）
const AILMENT_BUILDUP: u32 = 9_971_165;
const AILMENT_BUILDUP_RATE: f32 = 1.25;
// 金
const NO_RUNE_LOSS: u32 = 9_971_200;
// 角色专属（与紫色的技艺次数 +1 分开，两者可同时生效）
const ROLE_SKILL_USE: u32 = 9_971_300;
const IRONEYE_LONG_MARK: u32 = 9_971_310;
/// 同遗物换成延长版旋风，DLL 另把延长版寿命翻倍（shared::tiered_params）
const GUARDIAN_LONG_SKILL: u32 = nr_stat_alloc_shared::tiered_guardian::GUARDIAN_LONG_SKILL as u32;
const RECLUSE_LONG_BLOOD: u32 = 9_971_330;
const REVENANT_FAMILY_HP: u32 = 9_971_340;
const EXECUTOR_BEAST_UNDYING: u32 = 9_971_350;
const GUARDIAN_AGGRO: u32 = 9_971_380;
/// 追踪者元素锻体 9971360~367：追加攻击（灵火 / 血焰 / 黑焰 / 命定之死）与绝招地火，直接指向词条标记
const WYLDER_ELEMENT_BUFFS: [u32; 8] = [9_971_360, 9_971_361, 9_971_362, 9_971_363, 9_971_364, 9_971_365, 9_971_366, 9_971_367];
const WYLDER_MARKS: [u32; 8] = [7_020_050, 7_020_060, 7_020_070, 7_020_080, 7_010_550, 7_010_570, 7_010_580, 7_010_590];

// 原版锻体：霸体（强韧度二级）与持有卢恩增伤进金池；骤死、强韧度一级不再掉落
const UNFLINCHING_GUARD: u32 = 8_230_001;
const RUNE_ATTACK: u32 = 8_460_400;
const UNFLINCHING: u32 = 8_230_000;
const SUDDEN_DEATH: u32 = 8_600_300;

// 道具候选：石剑钥匙、锻造石【２】（护符与传说武器是表，挂在掉落行一层，见 tiered::split）
const STONESWORD_KEY: i32 = 8_000;
const SMITHING_STONE_2: i32 = 10_101;

// 模板
const DISCOVERY_TEMPLATE: u32 = 311_000;
const RAIN_TEMPLATE: u32 = 8_770_000;
const SKILL_USE_TEMPLATE: u32 = 7_033_200;
const NO_RUNE_LOSS_TEMPLATE: u32 = 360_700;
const IRONEYE_LONG_MARK_TEMPLATE: u32 = 7_280_000;
/// MMV 的隐士遗物「延长血印」（状态 2030 → 绝招改施加 20 秒的长版血印）
const RECLUSE_LONG_BLOOD_TEMPLATE: u32 = 7_034_300;
/// 原版未启用的复仇者遗物：状态 2115 + 给三个家人各挂一个效果（+1 / +2 / +3 是海伦 / 弗雷德里克 / 塞巴斯蒂安）
const REVENANT_FAMILY_TEMPLATE: u32 = 7_220_100;
/// 执行者兽化期间的不死效果（状态 143）
const IMMORTALITY_TEMPLATE: u32 = 707_110;
/// 守护者遗物「能力发动期间容易受敌人攻击」（状态 2265）
const GUARDIAN_AGGRO_TEMPLATE: u32 = 7_033_300;
/// 执行者4 的不死不限时：DLL 在兽化期间挂上、结束时移除（短时补挂会在到期那一帧留空档）
const UNDYING_SECONDS: f32 = -1.0;

pub const ROWS: &[RowSpec] = &[
    RowSpec { table: &SP_EFFECT, template: DISCOVERY_TEMPLATE, id: DISCOVERY_20, name: "[Tiered Buff] Improved Item Discovery +20",
        sets: &[Set::F32(sp::ITEM_DROP_RATE, 0.2), Set::U16(sp::SP_CATEGORY, STACKING), SAVE, icon(ICON_DISCOVERY), text(DISCOVERY_20)] },
    RowSpec { table: &SP_EFFECT, template: RAIN_TEMPLATE, id: RAIN_GUARD, name: "[Tiered Buff] Night's Tide Damage Reduced 10%",
        sets: &[Set::F32(sp::RAIN_DAMAGE_CORRECTION, 0.9), Set::U16(sp::SP_CATEGORY, STACKING), SAVE, icon(ICON_NEGATION), text(RAIN_GUARD)] },
    RowSpec { table: &SP_EFFECT, template: SKILL_USE_TEMPLATE, id: SKILL_USE, name: "[Tiered Buff] +1 Character Skill Use",
        sets: &[icon(ICON_SKILL), text(SKILL_USE)] },
    RowSpec { table: &SP_EFFECT, template: DISCOVERY_TEMPLATE, id: DISCOVERY_40, name: "[Tiered Buff] Improved Item Discovery +40",
        sets: &[Set::F32(sp::ITEM_DROP_RATE, 0.4), Set::U16(sp::SP_CATEGORY, STACKING), SAVE, icon(ICON_DISCOVERY), text(DISCOVERY_40)] },
    // 血量上限减半，攻击命中时回 25 血（命中触发链照抄「攻击命中时恢复精力」）
    // 另带常驻减伤 50%、黑夜雨伤害 -80%（两个字段都不依赖状态值，和命中触发的状态 2103 放在同一个效果里）
    RowSpec { table: &SP_EFFECT, template: ON_HIT_TEMPLATE, id: GLASS_CANNON, name: "[Tiered Buff] Halved Max HP, HP Restoration on Hit",
        sets: &[Set::F32(sp::MAX_HP_RATE, 0.5), Set::I32(sp::ATK_OCCURRENCE, GLASS_CANNON_HEAL as i32), cut(0, 0.5), cut(1, 0.5), cut(2, 0.5),
            cut(3, 0.5), cut(4, 0.5), cut(5, 0.5), cut(6, 0.5), cut(7, 0.5), Set::F32(sp::RAIN_DAMAGE_CORRECTION, 0.2),
            Set::U16(sp::SP_CATEGORY, NON_STACKING), SAVE, icon(ICON_HP), text(GLASS_CANNON)] },
    RowSpec { table: &SP_EFFECT, template: ON_HIT_RESTORE_TEMPLATE, id: GLASS_CANNON_HEAL, name: "[Tiered Buff] Halved Max HP, HP Restoration on Hit - Restore",
        sets: &[Set::I32(sp::CHANGE_STAMINA_POINT, 0), Set::I32(sp::CHANGE_HP_POINT, -25)] },
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: AILMENT_BUILDUP, name: "[Tiered Buff] Increased Status Ailment Buildup",
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, -1.0), Set::F32(sp::INFLICT_RATES[0], AILMENT_BUILDUP_RATE), Set::F32(sp::INFLICT_RATES[1], AILMENT_BUILDUP_RATE),
            Set::F32(sp::INFLICT_RATES[2], AILMENT_BUILDUP_RATE), Set::F32(sp::INFLICT_RATES[3], AILMENT_BUILDUP_RATE),
            Set::F32(sp::INFLICT_RATES[4], AILMENT_BUILDUP_RATE), Set::F32(sp::INFLICT_RATES[5], AILMENT_BUILDUP_RATE),
            Set::F32(sp::INFLICT_RATES[6], AILMENT_BUILDUP_RATE), Set::U16(sp::SP_CATEGORY, STACKING), SAVE, icon(ICON_ATTACK), text(AILMENT_BUILDUP)] },
    RowSpec { table: &SP_EFFECT, template: NO_RUNE_LOSS_TEMPLATE, id: NO_RUNE_LOSS, name: "[Tiered Buff] No Rune Loss or Level Down Upon Death",
        sets: &[Set::U16(sp::SP_CATEGORY, NON_STACKING), SAVE, icon(ICON_RUNES), text(NO_RUNE_LOSS)] },
    RowSpec { table: &SP_EFFECT, template: SKILL_USE_TEMPLATE, id: ROLE_SKILL_USE, name: "[Tiered Buff] Hero: +1 Character Skill Use",
        sets: &[icon(ICON_SKILL), text(ROLE_SKILL_USE)] },
    RowSpec { table: &SP_EFFECT, template: IRONEYE_LONG_MARK_TEMPLATE, id: IRONEYE_LONG_MARK, name: "[Tiered Buff] Ironeye: Extends duration of weak point",
        sets: &[icon(ICON_SKILL), text(IRONEYE_LONG_MARK)] },
    RowSpec { table: &SP_EFFECT, template: PERIODIC_TEMPLATE, id: GUARDIAN_LONG_SKILL, name: "[Tiered Buff] Guardian: Increased duration for Character Skill",
        sets: &[icon(ICON_SKILL), text(GUARDIAN_LONG_SKILL)] },
    RowSpec { table: &SP_EFFECT, template: RECLUSE_LONG_BLOOD_TEMPLATE, id: RECLUSE_LONG_BLOOD, name: "[Tiered Buff] Recluse: Longer Soulblood Song mark",
        sets: &[icon(ICON_ULTIMATE), text(RECLUSE_LONG_BLOOD)] },
    // 家人血量 ×2（原版未启用的遗物结构，待实测）
    RowSpec { table: &SP_EFFECT, template: REVENANT_FAMILY_TEMPLATE, id: REVENANT_FAMILY_HP, name: "[Tiered Buff] Revenant: Family Max HP x2",
        sets: &[Set::I32(sp::REVENANT_FAMILY[0], (REVENANT_FAMILY_HP + 1) as i32), Set::I32(sp::REVENANT_FAMILY[1], (REVENANT_FAMILY_HP + 2) as i32),
            Set::I32(sp::REVENANT_FAMILY[2], (REVENANT_FAMILY_HP + 3) as i32), Set::U16(sp::SP_CATEGORY, NON_STACKING), SAVE, icon(ICON_SKILL),
            text(REVENANT_FAMILY_HP)] },
    RowSpec { table: &SP_EFFECT, template: REVENANT_FAMILY_TEMPLATE + 1, id: REVENANT_FAMILY_HP + 1, name: "[Tiered Buff] Revenant: Family Max HP x2 - Helen",
        sets: &[Set::F32(sp::CHANGE_HP_RATE, 0.0), Set::F32(sp::MAX_HP_RATE, 2.0)] },
    RowSpec { table: &SP_EFFECT, template: REVENANT_FAMILY_TEMPLATE + 2, id: REVENANT_FAMILY_HP + 2, name: "[Tiered Buff] Revenant: Family Max HP x2 - Frederick",
        sets: &[Set::F32(sp::CHANGE_HP_RATE, 0.0), Set::F32(sp::MAX_HP_RATE, 2.0)] },
    RowSpec { table: &SP_EFFECT, template: REVENANT_FAMILY_TEMPLATE + 3, id: REVENANT_FAMILY_HP + 3, name: "[Tiered Buff] Revenant: Family Max HP x2 - Sebastian",
        sets: &[Set::F32(sp::CHANGE_HP_RATE, 0.0), Set::F32(sp::MAX_HP_RATE, 2.0)] },
    // 兽化期间锁满血：标记只给 DLL 判断（src\tiered_effects\beast.rs：兽化（707115）期间挂不死 +1、结束移除，血量下降就补满 +2，见 scripted.rs）；
    // 原来以 707100 / 状态 2290 判断兽化，诊断确认兽化期间根本没有它
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: EXECUTOR_BEAST_UNDYING, name: "[Tiered Buff] Executor: HP locked full while transformed",
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, -1.0), SAVE, icon(ICON_ULTIMATE), text(EXECUTOR_BEAST_UNDYING)] },
    RowSpec { table: &SP_EFFECT, template: IMMORTALITY_TEMPLATE, id: EXECUTOR_BEAST_UNDYING + 1, name: "[Tiered Buff] Executor: HP locked full while transformed - Undying",
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, UNDYING_SECONDS), cut(0, 1.0), cut(1, 1.0), cut(2, 1.0), cut(3, 1.0), cut(4, 1.0), cut(5, 1.0), cut(6, 1.0), cut(7, 1.0),
            NO_SYNC] },
    // 守护者10：照遗物给状态 2265（强化防御期间吸引仇恨）
    RowSpec { table: &SP_EFFECT, template: GUARDIAN_AGGRO_TEMPLATE, id: GUARDIAN_AGGRO, name: "[Tiered Buff] Guardian: Draws enemy aggression",
        sets: &[icon(ICON_AGGRO), text(GUARDIAN_AGGRO)] },
];

pub const BUFF_ROWS: &[RowSpec] = buff_rows![
    (GUARDIAN_AGGRO, GUARDIAN_AGGRO, ICON_AGGRO, RARITY_PURPLE, "Guardian: Draws enemy aggression"),
    (DISCOVERY_20, DISCOVERY_20, ICON_DISCOVERY, RARITY_BLUE, "Improved Item Discovery +20"),
    (RAIN_GUARD, RAIN_GUARD, ICON_NEGATION, RARITY_BLUE, "Night's Tide Damage Reduced 10%"),
    (SKILL_USE, SKILL_USE, ICON_SKILL, RARITY_PURPLE, "+1 Character Skill Use"),
    (DISCOVERY_40, DISCOVERY_40, ICON_DISCOVERY, RARITY_PURPLE, "Improved Item Discovery +40"),
    (GLASS_CANNON, GLASS_CANNON, ICON_HP, RARITY_PURPLE, "Halved Max HP, HP Restoration on Hit"),
    (AILMENT_BUILDUP, AILMENT_BUILDUP, ICON_ATTACK, RARITY_PURPLE, "Increased Status Ailment Buildup"),
    (NO_RUNE_LOSS, NO_RUNE_LOSS, ICON_RUNES, RARITY_GOLD, "No Rune Loss or Level Down Upon Death"),
    (ROLE_SKILL_USE, ROLE_SKILL_USE, ICON_SKILL, RARITY_PURPLE, "Hero: +1 Character Skill Use"),
    (IRONEYE_LONG_MARK, IRONEYE_LONG_MARK, ICON_SKILL, RARITY_PURPLE, "Ironeye: Extends duration of weak point"),
    (GUARDIAN_LONG_SKILL, GUARDIAN_LONG_SKILL, ICON_SKILL, RARITY_PURPLE, "Guardian: Increased duration for Character Skill"),
    (RECLUSE_LONG_BLOOD, RECLUSE_LONG_BLOOD, ICON_ULTIMATE, RARITY_PURPLE, "Recluse: Longer Soulblood Song mark"),
    (REVENANT_FAMILY_HP, REVENANT_FAMILY_HP, ICON_SKILL, RARITY_PURPLE, "Revenant: Family Max HP x2"),
    (EXECUTOR_BEAST_UNDYING, EXECUTOR_BEAST_UNDYING, ICON_ULTIMATE, RARITY_PURPLE, "Executor: HP locked full while transformed"),
    (WYLDER_ELEMENT_BUFFS[0], WYLDER_MARKS[0], ICON_SKILL, RARITY_PURPLE, "Wylder: Ghostflame follow-ups"),
    (WYLDER_ELEMENT_BUFFS[1], WYLDER_MARKS[1], ICON_SKILL, RARITY_PURPLE, "Wylder: Bloodflame follow-ups"),
    (WYLDER_ELEMENT_BUFFS[2], WYLDER_MARKS[2], ICON_SKILL, RARITY_PURPLE, "Wylder: Black Flame follow-ups"),
    (WYLDER_ELEMENT_BUFFS[3], WYLDER_MARKS[3], ICON_SKILL, RARITY_PURPLE, "Wylder: Destined Death follow-ups"),
    (WYLDER_ELEMENT_BUFFS[4], WYLDER_MARKS[4], ICON_ULTIMATE, RARITY_PURPLE, "Wylder: Art spreads ghostflame"),
    (WYLDER_ELEMENT_BUFFS[5], WYLDER_MARKS[5], ICON_ULTIMATE, RARITY_PURPLE, "Wylder: Art spreads bloodflame"),
    (WYLDER_ELEMENT_BUFFS[6], WYLDER_MARKS[6], ICON_ULTIMATE, RARITY_PURPLE, "Wylder: Art spreads black flame"),
    (WYLDER_ELEMENT_BUFFS[7], WYLDER_MARKS[7], ICON_ULTIMATE, RARITY_PURPLE, "Wylder: Art spreads Destined Death flames"),
];

/// 元素附魔效果的时长（右 / 左手各一行）
macro_rules! enchant {
    ($id:expr, $seconds:expr) => {
        Patch { table: &SP_EFFECT, id: $id, sets: &[Set::F32(sp::EFFECT_ENDURANCE, $seconds)] }
    };
}

/// 遗物版词条（与标记同号）不再挂常驻效果
macro_rules! relic_off {
    ($id:expr) => {
        Patch { table: &ATTACH_EFFECT, id: $id, sets: &[Set::I32(ae::PASSIVE_SP_EFFECT, -1)] }
    };
}

/// 附魔时长：灵火、血焰 30 秒，黑焰、命定之死 10 秒；遗物版的元素词条全部清空；移进金池的原版锻体改金色稀有度；执行者的解放状态加属性
pub const PATCHES: &[Patch] = &[
    super::tae::UNLEASHED_SWORD_STATS,
    Patch { table: &EQUIP_GOODS, id: UNFLINCHING_GUARD, sets: &[Set::U8(goods::RARITY, RARITY_GOLD)] },
    Patch { table: &EQUIP_GOODS, id: RUNE_ATTACK, sets: &[Set::U8(goods::RARITY, RARITY_GOLD)] },
    enchant!(7_020_052, 30.0),
    enchant!(7_020_054, 30.0),
    enchant!(7_020_062, 30.0),
    enchant!(7_020_064, 30.0),
    enchant!(7_020_072, 10.0),
    enchant!(7_020_074, 10.0),
    enchant!(7_020_082, 10.0),
    enchant!(7_020_084, 10.0),
    relic_off!(WYLDER_MARKS[0]),
    relic_off!(WYLDER_MARKS[1]),
    relic_off!(WYLDER_MARKS[2]),
    relic_off!(WYLDER_MARKS[3]),
    relic_off!(WYLDER_MARKS[4]),
    relic_off!(WYLDER_MARKS[5]),
    relic_off!(WYLDER_MARKS[6]),
    relic_off!(WYLDER_MARKS[7]),
];

/// 不再出现在任何池里的原版锻体
pub const EXCLUDED: &[u32] = &[UNFLINCHING, SUDDEN_DEATH];

pub const BLUE_EXTRA: &[Candidate] = &[
    Candidate::Buff(DISCOVERY_20),
    Candidate::Buff(RAIN_GUARD),
    Candidate::Goods(STONESWORD_KEY, 2),
    Candidate::Goods(SMITHING_STONE_2, 1),
];

pub const PURPLE_EXTRA: &[Candidate] =
    &[Candidate::Buff(SKILL_USE), Candidate::Buff(DISCOVERY_40), Candidate::Buff(GLASS_CANNON), Candidate::Buff(AILMENT_BUILDUP)];

/// 金池全部候选（原版锻体里的霸体与卢恩增伤从蓝 / 紫池移过来，霸体只在金池）
pub const GOLD: &[Candidate] = &[Candidate::Buff(NO_RUNE_LOSS), Candidate::Buff(UNFLINCHING_GUARD), Candidate::Buff(RUNE_ATTACK)];

pub const ROLE: &[RoleBuff] = &[
    RoleBuff { hero: WYLDER, buff: ROLE_SKILL_USE },
    RoleBuff { hero: WYLDER, buff: WYLDER_ELEMENT_BUFFS[0] },
    RoleBuff { hero: WYLDER, buff: WYLDER_ELEMENT_BUFFS[1] },
    RoleBuff { hero: WYLDER, buff: WYLDER_ELEMENT_BUFFS[2] },
    RoleBuff { hero: WYLDER, buff: WYLDER_ELEMENT_BUFFS[3] },
    RoleBuff { hero: WYLDER, buff: WYLDER_ELEMENT_BUFFS[4] },
    RoleBuff { hero: WYLDER, buff: WYLDER_ELEMENT_BUFFS[5] },
    RoleBuff { hero: WYLDER, buff: WYLDER_ELEMENT_BUFFS[6] },
    RoleBuff { hero: WYLDER, buff: WYLDER_ELEMENT_BUFFS[7] },
    RoleBuff { hero: GUARDIAN, buff: GUARDIAN_LONG_SKILL },
    RoleBuff { hero: GUARDIAN, buff: ROLE_SKILL_USE },
    RoleBuff { hero: GUARDIAN, buff: GUARDIAN_AGGRO },
    RoleBuff { hero: IRONEYE, buff: IRONEYE_LONG_MARK },
    RoleBuff { hero: IRONEYE, buff: ROLE_SKILL_USE },
    RoleBuff { hero: DUCHESS, buff: ROLE_SKILL_USE },
    RoleBuff { hero: RAIDER, buff: ROLE_SKILL_USE },
    RoleBuff { hero: REVENANT, buff: REVENANT_FAMILY_HP },
    RoleBuff { hero: RECLUSE, buff: RECLUSE_LONG_BLOOD },
    RoleBuff { hero: EXECUTOR, buff: EXECUTOR_BEAST_UNDYING },
    RoleBuff { hero: UNDERTAKER, buff: ROLE_SKILL_USE },
];
