//! 内置的原版成长表，只在游戏参数读取失败时用于诊断（识别角色、校验内存偏移）。
//! 正式加点一律以运行时读取的参数表为准（与实际加载的 regulation 同源）。

use super::growth::{HeroGrowth, Stats};
use super::heroes::HERO_COUNT;

const KEY_LEVELS: [u32; 4] = [1, 2, 12, 15];

/// [角色][关键帧][属性]，属性顺序：生命 精神 耐力 力量 敏捷 智力 信仰 感应
const KEYFRAMES: [[Stats; 4]; HERO_COUNT] = [
    [[8, 4, 3, 5, 4, 2, 2, 10], [16, 6, 6, 18, 15, 4, 4, 10], [46, 16, 24, 44, 35, 12, 12, 10], [52, 19, 27, 50, 40, 15, 15, 10]],
    [[10, 2, 6, 4, 3, 2, 3, 10], [19, 3, 12, 11, 9, 3, 5, 10], [54, 11, 33, 36, 26, 8, 18, 10], [60, 14, 38, 41, 31, 10, 21, 10]],
    [[6, 2, 4, 4, 8, 2, 2, 13], [12, 3, 7, 7, 20, 3, 3, 13], [34, 11, 25, 17, 51, 6, 10, 13], [37, 14, 28, 19, 57, 7, 13, 13]],
    [[7, 6, 3, 2, 5, 4, 3, 11], [13, 9, 4, 4, 13, 8, 6, 11], [35, 24, 14, 9, 38, 36, 24, 11], [39, 27, 18, 11, 45, 42, 27, 11]],
    [[9, 2, 6, 9, 4, 1, 2, 10], [18, 3, 12, 25, 6, 2, 4, 10], [50, 8, 31, 60, 16, 2, 10, 10], [56, 10, 37, 68, 19, 3, 12, 10]],
    [[6, 7, 3, 2, 2, 4, 5, 12], [11, 11, 4, 6, 6, 7, 15, 12], [32, 28, 18, 18, 18, 27, 45, 12], [35, 31, 21, 21, 21, 30, 51, 12]],
    [[6, 7, 3, 1, 2, 5, 5, 10], [10, 11, 5, 2, 4, 15, 15, 10], [30, 27, 20, 9, 16, 45, 45, 10], [33, 30, 23, 12, 19, 51, 51, 10]],
    [[7, 2, 3, 5, 8, 2, 2, 28], [14, 3, 5, 9, 20, 3, 3, 28], [40, 9, 24, 22, 60, 7, 5, 28], [46, 11, 27, 25, 63, 8, 6, 28]],
    [[7, 4, 3, 1, 2, 4, 2, 50], [14, 6, 5, 2, 4, 7, 4, 50], [36, 17, 22, 12, 15, 25, 12, 50], [41, 20, 25, 14, 18, 28, 15, 50]],
    [[7, 4, 3, 5, 2, 2, 5, 10], [14, 6, 5, 18, 4, 2, 17, 10], [42, 12, 19, 44, 11, 4, 37, 10], [48, 14, 22, 50, 13, 5, 41, 10]],
];

pub fn embedded() -> Vec<HeroGrowth> {
    KEYFRAMES
        .iter()
        .map(|frames| HeroGrowth::new(KEY_LEVELS.to_vec(), frames.to_vec()).expect("embedded table is valid"))
        .collect()
}

/// 按属性距离识别角色：返回 (角色序号, 曼哈顿距离)。距离 0 说明与原版表完全吻合。
pub fn identify(growths: &[HeroGrowth], level: u32, stats: &Stats) -> Option<(usize, u32)> {
    growths
        .iter()
        .enumerate()
        .map(|(h, g)| (h, g.stats_at(level).iter().zip(stats).map(|(a, b)| a.abs_diff(*b)).sum()))
        .min_by_key(|&(_, d)| d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_wiki_raider_table() {
        let g = &embedded()[4];
        let vig = [9, 18, 21, 24, 27, 30, 34, 37, 40, 43, 46, 50, 52, 54, 56];
        let str_ = [9, 25, 28, 32, 35, 39, 42, 46, 49, 53, 56, 60, 62, 65, 68];
        for lv in 1..=15u32 {
            let s = g.stats_at(lv);
            assert_eq!(s[0], vig[lv as usize - 1], "vigor lv{lv}");
            assert_eq!(s[3], str_[lv as usize - 1], "strength lv{lv}");
        }
    }

    #[test]
    fn identifies_exact_hero() {
        let all = embedded();
        for (h, g) in all.iter().enumerate() {
            for lv in 1..=15 {
                assert_eq!(identify(&all, lv, &g.stats_at(lv)), Some((h, 0)));
            }
        }
    }

    #[test]
    fn vanilla_gains_are_never_negative_and_arcane_is_constant() {
        for g in embedded() {
            for lv in 2..=15 {
                let (a, b) = (g.stats_at(lv - 1), g.stats_at(lv));
                assert!(a.iter().zip(b).all(|(x, y)| *x <= y));
                assert_eq!(a[7], b[7]);
            }
        }
    }
}
