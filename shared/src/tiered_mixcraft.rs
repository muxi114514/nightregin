//! 隐士1：技艺直接放混合魔法（同 tiered_runes：regulation 里只放标记，DLL 动手）。
//! 三个属性槽取右手三把武器的属性：只看有属性的，合起来取前三种再重复填满；都没有属性时为三魔力。
//! 武器看本身的基础属性攻击力，不算词条与附魔；法杖一律算魔力，圣印记按下面的表（手指、爪痕算无属性）。

use std::collections::HashMap;

use crate::param::layout::equip_weapon;
use crate::param::{ParamFile, field};

pub const DIRECT_COCKTAIL: i32 = 9_971_860;

/// 属性槽的值（MixcraftMagicParam 的 element1~3），0 为空槽
pub const MAGIC: u8 = 1;
pub const FIRE: u8 = 2;
pub const LIGHTNING: u8 = 3;
pub const HOLY: u8 = 4;
const ELEMENTS: [u8; 4] = [MAGIC, FIRE, LIGHTNING, HOLY];

const STAFF: u16 = 57;
const SEAL: u16 = 61;

/// 吸收该属性痕时引擎施加的效果（回 20 专注等，隐士3 与「收集属性痕」遗物都认它们）
pub fn absorb_effect(element: u8) -> Option<i32> {
    match element {
        FIRE => Some(706_000),
        MAGIC => Some(706_001),
        LIGHTNING => Some(706_002),
        HOLY => Some(706_003),
        _ => None,
    }
}

fn bit(element: u8) -> u8 {
    1 << (element - 1)
}

/// 圣印记的属性：ID 前四位是印记种类，同一种的各档都算
fn seal_element(id: u32) -> Option<u8> {
    match id / 10_000 {
        3401 | 3402 | 3408 | 3409 => Some(FIRE), // 狩猎神祇、巨人、龙飨、癫火
        3403 => Some(LIGHTNING),                 // 碎石
        3406 | 3407 => Some(HOLY),               // 黄金律法、黄金树
        _ => None,
    }
}

/// 一行武器的属性集合（按位）；attacks 的顺序同 ELEMENTS
fn element_mask(id: u32, wep_type: u16, attacks: [u16; 4]) -> u8 {
    match wep_type {
        STAFF => bit(MAGIC),
        SEAL => seal_element(id).map_or(0, bit),
        _ => attacks.iter().zip(ELEMENTS).filter(|(a, _)| **a > 0).fold(0, |m, (_, e)| m | bit(e)),
    }
}

/// 武器本体 ID → 属性集合（没有属性的不收）
#[derive(Debug, Default)]
pub struct WeaponElements {
    masks: HashMap<u32, u8>,
}

impl WeaponElements {
    pub fn build(table: &ParamFile) -> Self {
        Self::from_rows(table.rows().filter_map(|r| {
            let attack = |offset| field::u16(r.data, offset);
            Some((
                r.id,
                field::u16(r.data, equip_weapon::WEP_TYPE)?,
                [attack(equip_weapon::ATTACK_MAGIC)?, attack(equip_weapon::ATTACK_FIRE)?, attack(equip_weapon::ATTACK_THUNDER)?, attack(equip_weapon::ATTACK_DARK)?],
            ))
        }))
    }

    /// rows：(武器本体 ID, wepType, 魔力 / 火 / 雷 / 圣 基础攻击力)
    pub fn from_rows(rows: impl IntoIterator<Item = (u32, u16, [u16; 4])>) -> Self {
        let masks = rows.into_iter().map(|(id, t, attacks)| (id, element_mask(id, t, attacks))).filter(|(_, m)| *m != 0).collect();
        Self { masks }
    }

    pub fn len(&self) -> usize {
        self.masks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.masks.is_empty()
    }

    /// 带强化等级的物品 ID 的属性集合
    pub fn of(&self, item_id: u32) -> u8 {
        self.masks.get(&(item_id - item_id % 100)).copied().unwrap_or(0)
    }
}

/// 三把武器的属性集合 → 三个属性槽
pub fn slots(masks: [u8; 3]) -> [u8; 3] {
    let mut found: Vec<u8> = Vec::with_capacity(4);
    for mask in masks {
        for element in ELEMENTS {
            if mask & bit(element) != 0 && !found.contains(&element) {
                found.push(element);
            }
        }
    }
    match found[..] {
        [] => [MAGIC; 3],
        [a] => [a; 3],
        [a, b] => [a, a, b],
        [a, b, c, ..] => [a, b, c],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NO_ATTACK: [u16; 4] = [0; 4];

    #[test]
    fn staves_and_seals_ignore_their_attack() {
        assert_eq!(element_mask(33_000_000, STAFF, NO_ATTACK), bit(MAGIC));
        assert_eq!(element_mask(34_030_000, SEAL, NO_ATTACK), bit(LIGHTNING));
        assert_eq!(element_mask(34_010_000, SEAL, NO_ATTACK), bit(FIRE));
        assert_eq!(element_mask(34_020_000, SEAL, NO_ATTACK), bit(FIRE));
        assert_eq!(element_mask(34_080_000, SEAL, NO_ATTACK), bit(FIRE));
        assert_eq!(element_mask(34_090_000, SEAL, NO_ATTACK), bit(FIRE));
        assert_eq!(element_mask(34_060_000, SEAL, NO_ATTACK), bit(HOLY));
        assert_eq!(element_mask(34_070_000, SEAL, [0, 0, 0, 15]), bit(HOLY));
    }

    #[test]
    fn finger_and_clawmark_seals_have_no_element() {
        assert_eq!(element_mask(34_000_000, SEAL, NO_ATTACK), 0);
        assert_eq!(element_mask(34_040_000, SEAL, NO_ATTACK), 0);
    }

    #[test]
    fn other_weapons_use_every_base_attack() {
        assert_eq!(element_mask(1_000_000, 3, NO_ATTACK), 0);
        assert_eq!(element_mask(2_140_000, 3, [42, 42, 0, 0]), bit(MAGIC) | bit(FIRE));
        assert_eq!(element_mask(24_070_000, 87, [0, 18, 0, 42]), bit(FIRE) | bit(HOLY));
    }

    #[test]
    fn lookup_ignores_the_upgrade_level() {
        let table = WeaponElements::from_rows([(2_140_000, 3, [42, 42, 0, 0]), (1_000_000, 3, NO_ATTACK)]);
        assert_eq!(table.len(), 1);
        assert_eq!(table.of(2_140_003), bit(MAGIC) | bit(FIRE));
        assert_eq!(table.of(1_000_002), 0);
    }

    #[test]
    fn slots_only_look_at_weapons_with_elements() {
        assert_eq!(slots([0, 0, 0]), [MAGIC; 3]);
        assert_eq!(slots([bit(FIRE), 0, 0]), [FIRE; 3]);
        assert_eq!(slots([0, bit(FIRE), bit(FIRE)]), [FIRE; 3]);
        assert_eq!(slots([bit(FIRE), 0, bit(HOLY)]), [FIRE, FIRE, HOLY]);
        assert_eq!(slots([bit(LIGHTNING), bit(FIRE), bit(HOLY)]), [LIGHTNING, FIRE, HOLY]);
    }

    #[test]
    fn dual_element_weapons_count_both_and_extra_elements_drop_in_slot_order() {
        assert_eq!(slots([bit(MAGIC) | bit(FIRE), 0, 0]), [MAGIC, MAGIC, FIRE]);
        assert_eq!(slots([bit(LIGHTNING), bit(MAGIC) | bit(FIRE), bit(HOLY)]), [LIGHTNING, MAGIC, FIRE]);
    }

    #[test]
    fn every_element_has_its_absorb_effect() {
        assert_eq!(ELEMENTS.map(absorb_effect), [Some(706_001), Some(706_000), Some(706_002), Some(706_003)]);
        assert_eq!(absorb_effect(0), None);
    }
}
