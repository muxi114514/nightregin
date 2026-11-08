//! 三档锻体里靠 DLL 实现的卢恩类词条：regulation 里只放标记效果（mmv_relics --tiered-buffs 写入），DLL 看玩家身上有没有标记再动手。
//! 两边共用这里的效果 ID 与换算规则。
//! - 蓝1 RUNE_GAMBLE：可叠加，每拿一次身上多一份同号效果；份数增加时把持有卢恩乘一个随机倍数（对数均匀，中位数 1 倍）
//! - 紫13 RUNE_REFUND：持有卢恩减少（活着时的花费）按概率退回
//! - 金8 RUNE_SHIELD：卢恩够时 DLL 持续挂短时不死 RUNE_SHIELD_UNDYING，血量被压到 1 时扣卢恩并施加回满血 RUNE_SHIELD_HEAL
//!   与冲击波 RUNE_SHIELD_BLAST（发出子弹，对周围敌人造成大削韧）

pub const RUNE_GAMBLE: i32 = 9_971_020;
pub const RUNE_REFUND: i32 = 9_971_195;
pub const RUNE_SHIELD: i32 = 9_971_280;
pub const RUNE_SHIELD_UNDYING: i32 = 9_971_281;
pub const RUNE_SHIELD_HEAL: i32 = 9_971_282;
pub const RUNE_SHIELD_BLAST: i32 = 9_971_283;

/// 随机倍数：u 为 [0, 1) 的均匀随机数，在 [min, max] 上按对数均匀取值（min = 0.1、max = 10 时一半概率变少、一半变多）
pub fn gamble_multiplier(u: f64, min: f64, max: f64) -> f64 {
    let (lo, hi) = (min.max(f64::MIN_POSITIVE).ln(), max.max(min).ln());
    (lo + (hi - lo) * u.clamp(0.0, 1.0)).exp()
}

/// 乘倍数后的卢恩，封顶 cap
pub fn gambled(runes: u32, multiplier: f64, cap: u32) -> u32 {
    let v = (runes as f64 * multiplier).round();
    if v >= cap as f64 { cap } else { v.max(0.0) as u32 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multiplier_is_log_uniform_between_bounds() {
        assert!((gamble_multiplier(0.0, 0.1, 10.0) - 0.1).abs() < 1e-9);
        assert!((gamble_multiplier(0.5, 0.1, 10.0) - 1.0).abs() < 1e-9);
        assert!((gamble_multiplier(1.0, 0.1, 10.0) - 10.0).abs() < 1e-9);
    }

    #[test]
    fn gambled_runes_round_and_cap() {
        assert_eq!(gambled(12_345, 0.1, 1_000_000), 1_235);
        assert_eq!(gambled(300_000, 10.0, 1_000_000), 1_000_000);
        assert_eq!(gambled(0, 10.0, 1_000_000), 0);
    }
}
