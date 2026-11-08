//! 三档锻体里靠 DLL 实现的武器类词条（同 tiered_runes：regulation 里只放标记，DLL 动手）。
//! 武器颜色 = 当前稀有度（passive_inherit::Rarities：本体稀有度 + 强化等级，0 白 / 1 蓝 / 2 紫 / 3 金）。
//! - 金4 GOLD_WEAPON_DAMAGE：背包里每有一把金色武器挂一档 GOLD_WEAPON_ROWS（第 k 档倍率叠起来正好 +15% × k）
//! - 紫15 PURPLE_UPGRADE：可叠加，每拿一次把背包里白 / 蓝武器的强化等级提到紫色

use crate::passive_inherit::Rarities;

pub const GOLD_WEAPON_DAMAGE: i32 = 9_971_295;
/// 第 1~6 档（第 k 档 = 至少 k 把金色武器）
pub const GOLD_WEAPON_ROWS: [i32; 6] = [9_971_560, 9_971_561, 9_971_562, 9_971_563, 9_971_564, 9_971_565];
pub const PURPLE_UPGRADE: i32 = 9_971_198;

pub const PURPLE: u8 = 2;
pub const GOLD: u8 = 3;

/// 白 / 蓝武器升到紫色后的物品 ID（只提高强化等级）；已是紫色及以上、或不是带词条的武器时为 None
pub fn purple_id(rarities: &Rarities, id: u32) -> Option<u32> {
    let base = rarities.base(id)?;
    if rarities.current(id)? >= PURPLE || base >= PURPLE {
        return None;
    }
    Some(id - id % 100 + (PURPLE - base) as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn white_and_blue_become_purple() {
        let r = Rarities::from_rows([(2000000, 5, 0), (2010000, 5, 1), (2020000, 5, 2), (110000, 33, 0)]);
        assert_eq!(purple_id(&r, 2000000), Some(2000002));
        assert_eq!(purple_id(&r, 2000001), Some(2000002));
        assert_eq!(purple_id(&r, 2010000), Some(2010001));
        assert_eq!(purple_id(&r, 2000002), None);
        assert_eq!(purple_id(&r, 2020000), None);
        assert_eq!(purple_id(&r, 110000), None);
    }
}
