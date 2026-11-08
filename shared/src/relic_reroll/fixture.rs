//! 单元测试用的小型遗物数据：普通遗物（三栏同表）、深夜遗物（强力栏带负面 + 普通栏）、固定词条遗物。

use std::collections::{BTreeMap, HashMap};

use super::catalog::{Effect, Group, GroupKind, RelicCatalog, RelicRow};

pub const WYLDER: u32 = 1;
pub const GUARDIAN: u32 = 2;

pub const G_ATTR: u32 = 30000;
pub const G_ATK: u32 = 30020;
pub const G_DEF: u32 = 30040;
pub const G_WEAPON: u32 = 20000;
pub const G_HERO_WYLDER: u32 = 10000;
pub const G_HERO_GUARDIAN: u32 = 10010;
pub const G_DEBUFF: u32 = 40000;

pub const VIGOR_1: u32 = 7000000;
pub const VIGOR_2: u32 = 7000001;
pub const ATK_PHYS: u32 = 7001400;
pub const ATK_FIRE: u32 = 7001600;
pub const WEAPON_DAGGER: u32 = 7080000;
pub const HERO_WYLDER: u32 = 6500000;
pub const HERO_GUARDIAN: u32 = 6500100;
pub const STRONG_ATK: u32 = 6001400;
pub const STRONG_DEF: u32 = 6002600;
pub const CURSE_A: u32 = 6820000;
pub const CURSE_B: u32 = 6830000;

pub const T_STD: u32 = 100;
pub const T_STD_2: u32 = 200;
pub const T_STRONG: u32 = 2000000;
pub const T_PLAIN: u32 = 2100000;
pub const T_CURSE: u32 = 3000000;
pub const T_FIXED: u32 = 6001400;

pub const STANDARD: u32 = 102;
pub const DEEP: u32 = 2000001;
pub const FIXED: u32 = 1;

pub fn catalog() -> RelicCatalog {
    let relic = |deep, tables, curse_tables| RelicRow { deep, color: 0, icon_id: 0, tables, curse_tables };
    let relics = HashMap::from([
        (STANDARD, relic(false, [T_STD as i32, T_STD_2 as i32, T_STD as i32], [-1; 3])),
        (DEEP, relic(true, [T_STRONG as i32, T_PLAIN as i32, -1], [T_CURSE as i32, -1, -1])),
        (FIXED, relic(false, [T_FIXED as i32, -1, -1], [-1; 3])),
    ]);
    let std = [(VIGOR_1, 50), (VIGOR_2, 50), (ATK_PHYS, 100), (ATK_FIRE, 100), (WEAPON_DAGGER, 20), (HERO_WYLDER, 10), (HERO_GUARDIAN, 10)];
    let mut entries: Vec<(u32, u32, u32)> = Vec::new();
    for t in [T_STD, T_STD_2] {
        entries.extend(std.iter().map(|&(id, w)| (t, id, w)));
    }
    entries.extend([(T_STRONG, STRONG_ATK, 100), (T_STRONG, STRONG_DEF, 100), (T_PLAIN, VIGOR_1, 50), (T_PLAIN, ATK_PHYS, 50)]);
    entries.extend([(T_CURSE, CURSE_A, 100), (T_CURSE, CURSE_B, 100), (T_FIXED, STRONG_ATK, 100)]);
    let effect = |group, compat: u32| Effect { group, compat: compat as i32, sort_key: compat as i32, text_id: 0, passive: [-1; 3] };
    let effects = HashMap::from([
        (VIGOR_1, effect(G_ATTR, VIGOR_1)),
        (VIGOR_2, effect(G_ATTR, VIGOR_1)),
        (ATK_PHYS, Effect { sort_key: ATK_PHYS as i32, ..effect(G_ATK, 100) }),
        (ATK_FIRE, Effect { sort_key: ATK_FIRE as i32, ..effect(G_ATK, 100) }),
        (WEAPON_DAGGER, effect(G_WEAPON, WEAPON_DAGGER)),
        (HERO_WYLDER, Effect { sort_key: HERO_WYLDER as i32, ..effect(G_HERO_WYLDER, 900) }),
        (HERO_GUARDIAN, Effect { sort_key: HERO_GUARDIAN as i32, ..effect(G_HERO_GUARDIAN, 900) }),
        (STRONG_ATK, effect(G_ATK, STRONG_ATK)),
        (STRONG_DEF, effect(G_DEF, STRONG_DEF)),
        (CURSE_A, effect(G_DEBUFF, CURSE_A)),
        (CURSE_B, effect(G_DEBUFF, CURSE_B)),
    ]);
    // 常驻 SpEffect：词条 ID + 1，生命力 +1 另带一个 ID + 2
    let effects = effects
        .into_iter()
        .map(|(id, e)| (id, Effect { passive: [id as i32 + 1, if id == VIGOR_1 { id as i32 + 2 } else { -1 }, -1], ..e }))
        .collect();
    let group = |id, kind, order| (id, Group { id, kind, text_id: 0, order });
    let groups = BTreeMap::from([
        group(G_ATTR, GroupKind::General, 100),
        group(G_ATK, GroupKind::General, 101),
        group(G_DEF, GroupKind::General, 104),
        group(G_WEAPON, GroupKind::Weapon, 50),
        group(G_HERO_WYLDER, GroupKind::Hero(1), 1),
        group(G_HERO_GUARDIAN, GroupKind::Hero(2), 2),
        group(G_DEBUFF, GroupKind::Debuff, 0),
    ]);
    RelicCatalog::from_parts(relics, entries, effects, groups, false)
}
