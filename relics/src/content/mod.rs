//! 各组自定义词条与改动的行定义

pub mod bull_goat_talisman;
pub mod follow_up_gate;
pub mod kongcheng_follow_up;
pub mod map_icons;
pub mod night_bosses;
pub mod permanent_buffs;
pub mod weapon_passives;
pub mod wylder_bleed_restore;
pub mod wylder_elements;
pub mod wylder_ghostflame;
pub mod wylder_ghostflame_blast;

use crate::layout::{ATTACH_EFFECT, PERMANENT_BUFF, SMALL_BASE, SP_EFFECT};
use crate::spec::{Patch, RowSpec};
use crate::tiered;

/// 允许原地覆盖的原版行（其余已存在的 ID 视为被其他模组占用而报错）：
/// - 攻击表里的空闲行：原版遗留、原版 / MMV / 空城 0.46 都没有子弹 / 行为 / 战技引用。当初误以为乱序表插行会破坏查找才复用，
///   实际游戏会重建有序索引（见 insert.rs）；新内容直接新增行
/// - 直接修改的原版行：行定义以该行自身为模板
/// - 补回 MMV 删掉的原版行：MMV 里新增，原版独立版与空城 0.46 里原地改写为等效内容
pub const IN_PLACE: &[(&str, u32)] = &[
    ("BehaviorParam_PC", wylder_bleed_restore::BEHAVIOR),
    ("AtkParam_Pc", wylder_ghostflame_blast::GHOST_BLAST_ATK),
    ("AtkParam_Pc", wylder_ghostflame::GHOST_GROUND_ATK),
    ("AtkParam_Pc", wylder_elements::GROUND_ATKS[0]),
    ("AtkParam_Pc", wylder_elements::GROUND_ATKS[1]),
    ("AtkParam_Pc", wylder_elements::GROUND_ATKS[2]),
    ("EquipParamAccessory", bull_goat_talisman::TALISMAN),
    ("SpEffectParam", follow_up_gate::FIRE_MARK),
];

/// 该表允许原地覆盖的原版行：IN_PLACE 加上改效果的锻体行、改数值的武器词条（全是原地改写）、改图标的据点行
pub fn in_place(file: &str) -> Vec<u32> {
    let buffs = permanent_buffs::IN_PLACE.iter().map(|&id| (PERMANENT_BUFF.file, id));
    let passives = weapon_passives::rows().map(|s| (s.table.file, s.id));
    let spots = map_icons::IN_PLACE.iter().map(|&id| (SMALL_BASE.file, id));
    IN_PLACE.iter().copied().chain(buffs).chain(passives).chain(spots).filter(|(f, _)| *f == file).map(|(_, id)| id).collect()
}

/// 只有部分整合包才有的行（MMV 新增），表里没有就跳过对它的改动
pub fn is_optional(spec: &RowSpec) -> bool {
    (std::ptr::eq(spec.table, &SP_EFFECT) || std::ptr::eq(spec.table, &ATTACH_EFFECT)) && weapon_passives::OPTIONAL.contains(&spec.id)
}

/// map_icons：MMV 系 pkg 才加的湖沼 / 锻造村地图图标改动（要与 tools\dlc_map_icons.ps1 改的图集同时用）；
/// tiered：三档锻体与角色专属锻体（见 tiered）
pub fn all(map_icons: bool, tiered: bool) -> impl Iterator<Item = &'static RowSpec> {
    let mmv_only: &[RowSpec] = if map_icons { map_icons::ROWS } else { &[] };
    let tiered_rows: Vec<&'static [RowSpec]> = if tiered { tiered::row_groups().collect() } else { Vec::new() };
    [
        wylder_ghostflame::ROWS,
        kongcheng_follow_up::ROWS,
        wylder_ghostflame_blast::ROWS,
        wylder_elements::BLOODFLAME_ROWS,
        wylder_elements::BLACK_FLAME_ROWS,
        wylder_elements::DESTINED_DEATH_ROWS,
        wylder_elements::EXTRA_ROWS,
        follow_up_gate::ROWS,
        bull_goat_talisman::ROWS,
        wylder_bleed_restore::ROWS,
        permanent_buffs::ROWS,
        weapon_passives::SHOWN,
        weapon_passives::RATES,
        mmv_only,
    ]
    .into_iter()
    .chain(tiered_rows)
    .flatten()
}

/// 按开关覆盖既有行定义的个别字段
pub fn patches(tiered: bool) -> &'static [Patch] {
    if tiered { tiered::buffs::PATCHES } else { &[] }
}
