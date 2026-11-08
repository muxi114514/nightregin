//! 重铸价格。基础价 = 商店里买一个情景原石的价格：普通遗物按情景原石（暗痕）/ 大颗情景原石（为王之证），
//! 深夜遗物按暗淡版。倍率 = (全部候选权重 / 勾选后候选权重)^指数：筛得越窄每次越贵，但指数 < 1 时
//! 抽到指定词条的期望花费仍比不筛低。暗痕四舍五入到 10、为王之证到 1，不低于基础价。

use crate::param::layout::shop_lineup;
use crate::param::{ParamFile, field};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Cost {
    pub murk: u32,
    pub sigils: u32,
}

impl std::ops::Add for Cost {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self { murk: self.murk.saturating_add(other.murk), sigils: self.sigils.saturating_add(other.sigils) }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Prices {
    pub standard: Cost,
    pub deep: Cost,
}

/// 原版 1.03 的价格，商店表读不到时使用
pub const DEFAULT_PRICES: Prices = Prices { standard: Cost { murk: 600, sigils: 5 }, deep: Cost { murk: 1800, sigils: 10 } };

/// 商店行：小壶商人的情景原石 / 暗淡情景原石（暗痕），收藏家告示牌的大颗情景原石 / 暗淡大颗情景原石（为王之证）
const SHOP_ROWS: [u32; 4] = [10114, 10115, 13102, 13112];

impl Prices {
    pub fn from_shop(shop: &ParamFile) -> Self {
        let price = |row: u32, fallback: u32| {
            shop.row(row).and_then(|r| field::i32(r, shop_lineup::VALUE)).and_then(|v| u32::try_from(v).ok()).filter(|&v| v > 0).unwrap_or(fallback)
        };
        let d = DEFAULT_PRICES;
        Self {
            standard: Cost { murk: price(SHOP_ROWS[0], d.standard.murk), sigils: price(SHOP_ROWS[2], d.standard.sigils) },
            deep: Cost { murk: price(SHOP_ROWS[1], d.deep.murk), sigils: price(SHOP_ROWS[3], d.deep.sigils) },
        }
    }

    pub fn base(&self, deep: bool) -> Cost {
        if deep { self.deep } else { self.standard }
    }

    /// 一个位置的价格；勾选后没有候选时为 None
    pub fn quote(&self, deep: bool, all: u64, selected: u64, exponent: f32) -> Option<Cost> {
        if selected == 0 || all < selected {
            return None;
        }
        let factor = (all as f64 / selected as f64).powf(exponent.clamp(0.0, 4.0) as f64);
        let base = self.base(deep);
        let scale = |value: u32, step: f64| ((value as f64 * factor / step).round() * step).min(u32::MAX as f64) as u32;
        Some(Cost { murk: scale(base.murk, 10.0).max(base.murk), sigils: scale(base.sigils, 1.0).max(base.sigils) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quote_scales_by_selected_share() {
        let p = DEFAULT_PRICES;
        // 全选：基础价；只留 1/4 权重：√4 = 2 倍
        assert_eq!(p.quote(false, 1000, 1000, 0.5), Some(Cost { murk: 600, sigils: 5 }));
        assert_eq!(p.quote(true, 1000, 250, 0.5), Some(Cost { murk: 3600, sigils: 20 }));
        // 只去掉 1%：取整后仍是基础价；指数 1 时按比例
        assert_eq!(p.quote(false, 1000, 990, 0.5), Some(Cost { murk: 600, sigils: 5 }));
        assert_eq!(p.quote(false, 1000, 189, 1.0), Some(Cost { murk: 3170, sigils: 26 }));
        // 指数 0：筛选不加价；没有候选不能报价
        assert_eq!(p.quote(false, 1000, 1, 0.0), Some(p.standard));
        assert_eq!(p.quote(false, 1000, 0, 0.5), None);
    }

    #[test]
    fn costs_add_up() {
        assert_eq!(Cost { murk: 600, sigils: 5 } + Cost { murk: 1380, sigils: 11 }, Cost { murk: 1980, sigils: 16 });
    }
}
