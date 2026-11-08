//! DLL 改内存参数表实现的锻体（src\tiered_effects\param_overrides.rs），这里只放标记效果，ID 与要改的字段见 shared::tiered_params：
//! 第八批角色锻体、学者1，以及可叠加的蓝2 护符栏 / 蓝8 道具栏 / 蓝14 黑夜雨（标记用类别 10，每拿一份多一个同号效果，DLL 按个数算）。
//! 学者3 另有两行易伤：DLL 把观察第三级挂给敌人的效果从原版 708020 换成 SCHOLAR_WEAKNESS（同原版：10 秒、被打即结束），
//! 结束时换成 3 秒不会被打掉的易伤，之后再接原版的 0.5 秒收尾特效

use nr_stat_alloc_shared::tiered_params::{
    GUARDIAN_CHEAP_HOLD, GUARDIAN_SKILL_AS_ART, ITEM_SLOT, LONG_NIGHT_TIDE, REVENANT_FAST_REVIVE, REVENANT_SURE_NECROMANCY, SCHOLAR_FAST_ANALYSIS,
    SCHOLAR_LASTING_WEAKNESS, SCHOLAR_MAX_LEVEL, SCHOLAR_WEAKNESS, TALISMAN_SLOT, UNDERTAKER_LONG_TRANCE,
};

use crate::layout::{SP_EFFECT, sp_effect as sp};
use crate::spec::{RowSpec, Set};

use super::pools::{Candidate, RoleBuff};
use super::rows::{
    GUARDIAN, ICON_DISCOVERY, ICON_NEGATION, ICON_SKILL, ICON_ULTIMATE, MARKER_TEMPLATE, RARITY_BLUE, RARITY_PURPLE, REVENANT, SAVE, SCHOLAR, STACKING,
    UNDERTAKER, buff_rows, icon, text,
};

const LONG_TRANCE: u32 = UNDERTAKER_LONG_TRANCE as u32;
const MAX_LEVEL: u32 = SCHOLAR_MAX_LEVEL as u32;
const LASTING_WEAKNESS: u32 = SCHOLAR_LASTING_WEAKNESS as u32;
const WEAKNESS: u32 = SCHOLAR_WEAKNESS as u32;
const WEAKNESS_LASTING: u32 = WEAKNESS + 1;
const FAST_ANALYSIS: u32 = SCHOLAR_FAST_ANALYSIS as u32;
const SURE_NECROMANCY: u32 = REVENANT_SURE_NECROMANCY as u32;
const FAST_REVIVE: u32 = REVENANT_FAST_REVIVE as u32;
const SKILL_AS_ART: u32 = GUARDIAN_SKILL_AS_ART as u32;
const CHEAP_HOLD: u32 = GUARDIAN_CHEAP_HOLD as u32;
const TALISMAN: u32 = TALISMAN_SLOT as u32;
const ITEM: u32 = ITEM_SLOT as u32;
const NIGHT_TIDE: u32 = LONG_NIGHT_TIDE as u32;

/// 学者观察第三级给敌人挂的易伤（全部受到的伤害 ×1.6）与它结束后的 0.5 秒收尾特效
const WEAKNESS_TEMPLATE: u32 = 708_020;
const WEAKNESS_TAIL: u32 = 708_021;
const LASTING_SECONDS: f32 = 3.0;

const fn marker(id: u32, icon_of: super::rows::Icon) -> [Set; 4] {
    [Set::F32(sp::EFFECT_ENDURANCE, -1.0), SAVE, icon(icon_of), text(id)]
}

const fn stacking_marker(id: u32, icon_of: super::rows::Icon) -> [Set; 5] {
    [Set::F32(sp::EFFECT_ENDURANCE, -1.0), Set::U16(sp::SP_CATEGORY, STACKING), SAVE, icon(icon_of), text(id)]
}

pub const ROWS: &[RowSpec] = &[
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: LONG_TRANCE, name: "[Tiered Buff] Undertaker: Longer Trance", sets: &marker(LONG_TRANCE, ICON_SKILL) },
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: MAX_LEVEL, name: "[Tiered Buff] Scholar: Items start at max level",
        sets: &marker(MAX_LEVEL, ICON_DISCOVERY) },
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: LASTING_WEAKNESS, name: "[Tiered Buff] Scholar: Lasting analysis weakness",
        sets: &marker(LASTING_WEAKNESS, ICON_SKILL) },
    RowSpec { table: &SP_EFFECT, template: WEAKNESS_TEMPLATE, id: WEAKNESS, name: "[Tiered Buff] Scholar: Lasting analysis weakness - Weakness",
        sets: &[Set::I32(sp::REPLACE_SP_EFFECT, WEAKNESS_LASTING as i32)] },
    RowSpec { table: &SP_EFFECT, template: WEAKNESS_TEMPLATE, id: WEAKNESS_LASTING, name: "[Tiered Buff] Scholar: Lasting analysis weakness - Lasting",
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, LASTING_SECONDS), Set::U8(sp::DELETE_CRITERIA_DAMAGE, 0), Set::I32(sp::REPLACE_SP_EFFECT, WEAKNESS_TAIL as i32)] },
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: FAST_ANALYSIS, name: "[Tiered Buff] Scholar: Faster analysis", sets: &marker(FAST_ANALYSIS, ICON_SKILL) },
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: SURE_NECROMANCY, name: "[Tiered Buff] Revenant: Necromancy always triggers",
        sets: &marker(SURE_NECROMANCY, ICON_SKILL) },
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: FAST_REVIVE, name: "[Tiered Buff] Revenant: Family revives faster", sets: &marker(FAST_REVIVE, ICON_SKILL) },
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: SKILL_AS_ART, name: "[Tiered Buff] Guardian: Skill counts as skill attack",
        sets: &marker(SKILL_AS_ART, ICON_SKILL) },
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: CHEAP_HOLD, name: "[Tiered Buff] Guardian: Sustained Ult drains less gauge",
        sets: &marker(CHEAP_HOLD, ICON_ULTIMATE) },
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: TALISMAN, name: "[Tiered Buff] +1 Talisman Slot", sets: &stacking_marker(TALISMAN, ICON_DISCOVERY) },
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: ITEM, name: "[Tiered Buff] +1 Item Slot", sets: &stacking_marker(ITEM, ICON_DISCOVERY) },
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: NIGHT_TIDE, name: "[Tiered Buff] Night's Tide takes longer to turn lethal",
        sets: &stacking_marker(NIGHT_TIDE, ICON_NEGATION) },
];

pub const BUFF_ROWS: &[RowSpec] = buff_rows![
    (LONG_TRANCE, LONG_TRANCE, ICON_SKILL, RARITY_PURPLE, "Undertaker: Longer Trance"),
    (MAX_LEVEL, MAX_LEVEL, ICON_DISCOVERY, RARITY_PURPLE, "Scholar: Items start at max level"),
    (LASTING_WEAKNESS, LASTING_WEAKNESS, ICON_SKILL, RARITY_PURPLE, "Scholar: Lasting analysis weakness"),
    (FAST_ANALYSIS, FAST_ANALYSIS, ICON_SKILL, RARITY_PURPLE, "Scholar: Faster analysis"),
    (SURE_NECROMANCY, SURE_NECROMANCY, ICON_SKILL, RARITY_PURPLE, "Revenant: Necromancy always triggers"),
    (FAST_REVIVE, FAST_REVIVE, ICON_SKILL, RARITY_PURPLE, "Revenant: Family revives faster"),
    (SKILL_AS_ART, SKILL_AS_ART, ICON_SKILL, RARITY_PURPLE, "Guardian: Skill counts as skill attack"),
    (CHEAP_HOLD, CHEAP_HOLD, ICON_ULTIMATE, RARITY_PURPLE, "Guardian: Sustained Ult drains less gauge"),
    (TALISMAN, TALISMAN, ICON_DISCOVERY, RARITY_BLUE, "+1 Talisman Slot"),
    (ITEM, ITEM, ICON_DISCOVERY, RARITY_BLUE, "+1 Item Slot"),
    (NIGHT_TIDE, NIGHT_TIDE, ICON_NEGATION, RARITY_BLUE, "Night's Tide takes longer to turn lethal"),
];

pub const BLUE: &[Candidate] = &[Candidate::Buff(TALISMAN), Candidate::Buff(ITEM), Candidate::Buff(NIGHT_TIDE)];

pub const ROLE: &[RoleBuff] = &[
    RoleBuff { hero: UNDERTAKER, buff: LONG_TRANCE },
    RoleBuff { hero: SCHOLAR, buff: MAX_LEVEL },
    RoleBuff { hero: SCHOLAR, buff: LASTING_WEAKNESS },
    RoleBuff { hero: SCHOLAR, buff: FAST_ANALYSIS },
    RoleBuff { hero: REVENANT, buff: SURE_NECROMANCY },
    RoleBuff { hero: REVENANT, buff: FAST_REVIVE },
    RoleBuff { hero: GUARDIAN, buff: SKILL_AS_ART },
    RoleBuff { hero: GUARDIAN, buff: CHEAP_HOLD },
];
