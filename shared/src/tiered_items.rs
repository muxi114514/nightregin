//! 三档锻体里靠 DLL 实现的道具类词条（同 tiered_runes：regulation 里只放标记，DLL 动手）。
//! 持有上限来自 EquipParamGoods 的 maxNum（学者用 bagcraftMaxNum，-1 时同 maxNum），游戏拾取时现查参数表，
//! 所以 DLL 直接改内存里的这两个字段，倍数变化或回到圆桌时按原值重算。
//! - 金12 GOLD_ITEMS：持有上限 ×2，并常驻遗物「使用道具时效果扩散到周围同伴」的三个状态效果 SPREAD_EFFECTS
//! - 学者2 SCHOLAR_ITEMS：持有上限 ×2，与金12 一起为 ×4

pub const GOLD_ITEMS: i32 = 9_971_298;
pub const SCHOLAR_ITEMS: i32 = 9_971_498;
/// 遗物 7050100 的常驻效果（状态 2013 / 2014 / 2104，不限时、无图标）
pub const SPREAD_EFFECTS: [i32; 3] = [7_050_100, 7_050_200, 7_050_300];
/// 学者上限为 -1、但也要翻倍的道具：石剑钥匙、锻造石【1】~【3】
pub const EXTRA_GOODS: [u32; 4] = [8_000, 10_100, 10_101, 10_102];

/// 要改上限的道具：有学者上限的消耗品，加上 EXTRA_GOODS
pub fn is_scaled(id: u32, bagcraft_max: i8) -> bool {
    bagcraft_max >= 0 || EXTRA_GOODS.contains(&id)
}

/// 持有上限倍数：每个标记 ×2
pub fn multiplier(ids: &[i32]) -> u8 {
    [GOLD_ITEMS, SCHOLAR_ITEMS].iter().filter(|id| ids.contains(id)).fold(1, |m, _| m * 2)
}

/// 原上限乘倍数；s8 字段封顶 127，-1 等非正数不动
pub fn scaled_max(original: i16, m: u8) -> i16 {
    if original <= 0 { original } else { original.saturating_mul(m as i16) }
}

pub fn scaled_bagcraft(original: i8, m: u8) -> i8 {
    if original <= 0 { original } else { (original as i16 * m as i16).min(i8::MAX as i16) as i8 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markers_double_each() {
        assert_eq!(multiplier(&[]), 1);
        assert_eq!(multiplier(&[GOLD_ITEMS, 5]), 2);
        assert_eq!(multiplier(&[SCHOLAR_ITEMS, GOLD_ITEMS]), 4);
    }

    #[test]
    fn scaling_keeps_unset_limits() {
        assert_eq!(scaled_max(2, 4), 8);
        assert_eq!(scaled_bagcraft(4, 2), 8);
        assert_eq!(scaled_bagcraft(-1, 4), -1);
        assert_eq!(scaled_bagcraft(99, 4), 127);
        assert!(is_scaled(3030, 4) && is_scaled(8000, -1) && !is_scaled(45000, -1));
    }
}
