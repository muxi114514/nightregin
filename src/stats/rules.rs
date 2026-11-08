//! 逐级分配规则：把方案（1 级分配 + 成长权重 + 逐级覆盖）展开成 1~15 级每级的目标属性，并校验。
//! 规则：
//! - 感应只在 1 级分配，之后固定。感应比原版少的点不能在 1 级用，而是从 15 级开始倒序逐级返还
//!   （15、14、…、1 级各 1 点为一轮）；感应比原版多的点只能由 1 级其它属性让出。
//!   即调整感应永远不会让前期变强：卖感应换后期点，买感应花 1 级点。
//! - 1 级 8 项总和 = 原版 1 级总和 - 感应减少的点 + 1 级分到的返还；
//!   2~15 级每级新增 = 原版该级新增 + 返还 + 额外点，且只加不减；任何一项不得低于 min、不得超过 cap。

use std::collections::BTreeMap;

use super::distribute::distribute;
use super::growth::{ARCANE, GROWTH_STATS, HeroGrowth, MAX_LEVEL, Stats};
use super::{STAT_COUNT, stat_name};

pub type Gains = [u32; GROWTH_STATS];

#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub min: u32,
    pub cap: u32,
}

/// 方案原始输入。level1 中为 None 的项取原版 1 级数值。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SchemeSpec {
    pub level1: [Option<u32>; STAT_COUNT],
    pub weights: Gains,
    /// 2~15 级手动指定的新增点
    pub overrides: BTreeMap<u32, Gains>,
    /// 初始武器（与属性规则无关，随方案一起按角色 + 器皿保存）
    pub weapons: WeaponPick,
}

/// 初始武器选择，写法见 starting_weapons::choice（"执行者" / "守护者副手" / "无" / 武器 ID）；None 为该手保持原版
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WeaponPick {
    pub main: Option<String>,
    pub off: Option<String>,
}

impl WeaponPick {
    pub fn is_vanilla(&self) -> bool {
        self.main.is_none() && self.off.is_none()
    }
}

/// 展开后的方案：下标 = 等级 - 1
#[derive(Debug, Clone, PartialEq)]
pub struct Allocation {
    levels: Vec<Stats>,
}

impl Allocation {
    pub fn at(&self, level: u32) -> Stats {
        self.levels[(level.clamp(1, MAX_LEVEL) - 1) as usize]
    }
}

/// extra[L-1] 为 L 级额外可分配的点（王之证解锁，第 4 步接入，现为 0）。
/// 成功时返回展开结果与提示（自动补点等）；失败返回原因，调用方应回退原版成长。
pub fn build(spec: &SchemeSpec, vanilla: &HeroGrowth, extra: &[u32], limits: Limits) -> Result<(Allocation, Vec<String>), String> {
    let mut notes = Vec::new();
    let (min, cap) = (limits.min, limits.cap);
    let v1 = vanilla.stats_at(1);
    let in_range = |x: u32| (limits.min..=limits.cap).contains(&x);

    // ---------- 1 级 ----------
    let mut l1: Stats = std::array::from_fn(|i| spec.level1[i].unwrap_or(v1[i]));
    let arcane = l1[ARCANE];
    if !in_range(arcane) {
        return Err(crate::tr!("1 级感应 {arcane} 超出范围 {min}~{cap}", "Lv1 Arcane {arcane} is outside {min}~{cap}"));
    }
    let budget = budgets(spec, vanilla, extra);
    let budget1 = budget[0];
    let sum1: u32 = l1.iter().sum();
    if sum1 > budget1 {
        return Err(crate::tr!("1 级共分配 {sum1} 点，超过可用的 {budget1} 点", "Lv1 uses {sum1} points but only {budget1} are available"));
    }
    if sum1 < budget1 {
        let weights = effective_weights(&spec.weights, vanilla, 2);
        let room: Gains = std::array::from_fn(|i| limits.cap.saturating_sub(l1[i]));
        let (fill, left) = distribute(budget1 - sum1, &weights, &room, &mut [0.0; GROWTH_STATS]);
        add_gains(&mut l1, &fill);
        let filled = budget1 - sum1 - left;
        notes.push(crate::tr!("1 级有 {filled} 点未分配，已按成长权重补上", "{filled} unallocated Lv1 points were filled by growth weights"));
        if left > 0 {
            return Err(crate::tr!("1 级还有 {left} 点无处可加（各项已到上限）", "{left} Lv1 points cannot be placed (every attribute is capped)"));
        }
    }
    if let Some(i) = (0..STAT_COUNT).find(|&i| !in_range(l1[i])) {
        let (name, value) = (stat_name(i), l1[i]);
        return Err(crate::tr!("1 级{name}为 {value}，超出范围 {min}~{cap}", "Lv1 {name} {value} is outside {min}~{cap}"));
    }

    // ---------- 2~15 级 ----------
    let mut levels = vec![l1];
    let mut carry = [0.0; GROWTH_STATS];
    for level in 2..=MAX_LEVEL {
        let budget = budget[level as usize - 1];
        let prev = *levels.last().unwrap();
        let room: Gains = std::array::from_fn(|i| limits.cap.saturating_sub(prev[i]));

        let mut gains = match spec.overrides.get(&level) {
            Some(manual) => {
                let s: u32 = manual.iter().sum();
                if s > budget {
                    return Err(crate::tr!("{level} 级手动分配 {s} 点，超过该级可用的 {budget} 点", "Lv{level} allocates {s} points but only {budget} are available"));
                }
                if let Some(i) = (0..GROWTH_STATS).find(|&i| manual[i] > room[i]) {
                    let name = stat_name(i);
                    return Err(crate::tr!("{level} 级{name}会超过上限 {cap}", "Lv{level} {name} would exceed the cap {cap}"));
                }
                *manual
            }
            None => [0; GROWTH_STATS],
        };
        let assigned: u32 = gains.iter().sum();
        if assigned < budget {
            let room_left: Gains = std::array::from_fn(|i| room[i] - gains[i]);
            let weights = effective_weights(&spec.weights, vanilla, level);
            let (fill, mut left) = distribute(budget - assigned, &weights, &room_left, &mut carry);
            add_gains7(&mut gains, &fill);
            if left > 0 {
                // 权重项都到上限后，剩余点按原版该级成长、再按平均分给其它项
                let room_now: Gains = std::array::from_fn(|i| room[i] - gains[i]);
                let (more, rest) = distribute(left, &fallback_weights(vanilla, level), &room_now, &mut [0.0; GROWTH_STATS]);
                add_gains7(&mut gains, &more);
                left = rest;
            }
            if spec.overrides.contains_key(&level) {
                let filled = budget - assigned - left;
                notes.push(crate::tr!("{level} 级手动分配不足，已按成长权重补上 {filled} 点", "Lv{level} was under-allocated; {filled} points were filled by growth weights"));
            }
            if left > 0 {
                notes.push(crate::tr!("{level} 级有 {left} 点因所有属性都到上限而作废", "Lv{level}: {left} points were lost because every attribute is capped"));
            }
        }
        let mut next = prev;
        add_gains(&mut next, &gains);
        next[ARCANE] = arcane;
        levels.push(next);
    }
    Ok((Allocation { levels }, notes))
}

/// 每级可分配的点数，下标 = 等级 - 1。1 级为 8 项总和（含感应）；2~15 级为 7 项新增；都含感应返还与额外点
pub fn budgets(spec: &SchemeSpec, vanilla: &HeroGrowth, extra: &[u32]) -> Vec<u32> {
    let v1_total: u32 = vanilla.stats_at(1).iter().sum();
    let refund = arcane_refund(spec, vanilla);
    let extra_at = |level: u32| extra.get(level as usize - 1).copied().unwrap_or(0);
    (1..=MAX_LEVEL)
        .map(|l| {
            let base = if l == 1 { v1_total - arcane_freed(spec, vanilla) } else { vanilla.gain_at(l) };
            base + refund[l as usize - 1] + extra_at(l)
        })
        .collect()
}

/// 感应比原版 1 级少的点数，从 15 级开始倒序逐级返还，每轮 15、14、…、1 级各 1 点。下标 = 等级 - 1
pub fn arcane_refund(spec: &SchemeSpec, vanilla: &HeroGrowth) -> [u32; MAX_LEVEL as usize] {
    let freed = arcane_freed(spec, vanilla);
    std::array::from_fn(|i| {
        let order = MAX_LEVEL - 1 - i as u32; // 15 级为 0，1 级为 14
        freed / MAX_LEVEL + u32::from(order < freed % MAX_LEVEL)
    })
}

fn arcane_freed(spec: &SchemeSpec, vanilla: &HeroGrowth) -> u32 {
    let v1 = vanilla.stats_at(1)[ARCANE];
    v1.saturating_sub(spec.level1[ARCANE].unwrap_or(v1))
}

/// 权重全为 0 时沿用原版该级的成长比例
fn effective_weights(weights: &Gains, vanilla: &HeroGrowth, level: u32) -> Gains {
    if weights.iter().any(|&w| w > 0) { *weights } else { fallback_weights(vanilla, level) }
}

fn fallback_weights(vanilla: &HeroGrowth, level: u32) -> Gains {
    let (a, b) = (vanilla.stats_at(level - 1), vanilla.stats_at(level.max(2)));
    let w: Gains = std::array::from_fn(|i| b[i].saturating_sub(a[i]));
    if w.iter().any(|&x| x > 0) { w } else { [1; GROWTH_STATS] }
}

fn add_gains(stats: &mut Stats, gains: &Gains) {
    for i in 0..GROWTH_STATS {
        stats[i] += gains[i];
    }
}

fn add_gains7(a: &mut Gains, b: &Gains) {
    for i in 0..GROWTH_STATS {
        a[i] += b[i];
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::vanilla::embedded;

    const LIMITS: Limits = Limits { min: 1, cap: 70 };
    const EXECUTOR: usize = 7;
    const SCHOLAR: usize = 8;

    fn check_invariants(spec: &SchemeSpec, alloc: &Allocation, vanilla: &HeroGrowth) {
        let budget = budgets(spec, vanilla, &[]);
        let l1 = alloc.at(1);
        assert_eq!(l1.iter().sum::<u32>(), budget[0]);
        for level in 2..=MAX_LEVEL {
            let (a, b) = (alloc.at(level - 1), alloc.at(level));
            assert!(a.iter().zip(b).all(|(x, y)| *x <= y), "decreased at {level}");
            assert_eq!(a[ARCANE], b[ARCANE]);
            let gained: u32 = (0..GROWTH_STATS).map(|i| b[i] - a[i]).sum();
            assert_eq!(gained, budget[level as usize - 1], "budget at {level}");
            assert!(b.iter().all(|&x| (1..=70).contains(&x)));
        }
        // 15 级总量始终与原版一致
        assert_eq!(alloc.at(15).iter().sum::<u32>(), vanilla.stats_at(15).iter().sum::<u32>());
    }

    #[test]
    fn empty_scheme_reproduces_vanilla_totals() {
        let all = embedded();
        for g in &all {
            let (alloc, _) = build(&SchemeSpec::default(), g, &[], LIMITS).unwrap();
            check_invariants(&SchemeSpec::default(), &alloc, g);
            assert_eq!(alloc.at(1), g.stats_at(1));
        }
    }

    #[test]
    fn weights_shape_growth_and_respect_cap() {
        let g = &embedded()[EXECUTOR];
        let spec = SchemeSpec { weights: [3, 0, 1, 0, 3, 0, 0], ..Default::default() };
        let (alloc, _) = build(&spec, g, &[], LIMITS).unwrap();
        check_invariants(&spec, &alloc, g);
        let l15 = alloc.at(15);
        assert!(l15[0] > 40 && l15[4] > 40 && l15[1] == 2, "{l15:?}");
    }

    #[test]
    fn refund_runs_backwards_from_level_15() {
        let g = &embedded()[SCHOLAR];
        let refund = |freed: u32| {
            let mut spec = SchemeSpec::default();
            spec.level1[ARCANE] = Some(50 - freed);
            arcane_refund(&spec, g)
        };
        let mut expected = [0; 15];
        expected[14] = 1;
        assert_eq!(refund(1), expected);
        expected[13] = 1;
        assert_eq!(refund(2), expected);
        assert_eq!(refund(15), [1; 15]);
        let mut expected = [1; 15];
        expected[14] = 2;
        assert_eq!(refund(16), expected);
    }

    #[test]
    fn lowering_arcane_moves_points_to_late_levels() {
        let g = &embedded()[SCHOLAR];
        let mut spec = SchemeSpec::default();
        spec.level1[ARCANE] = Some(10);
        let (alloc, _) = build(&spec, g, &[], LIMITS).unwrap();
        check_invariants(&spec, &alloc, g);
        // 少 40 点：2 轮半，1 级拿回 2 点，其余 7 项与原版 1 级相同
        assert_eq!(alloc.at(1).iter().sum::<u32>(), g.stats_at(1).iter().sum::<u32>() - 38);
        assert_eq!(alloc.at(15)[ARCANE], 10);
    }

    #[test]
    fn raising_arcane_needs_level1_points() {
        let g = &embedded()[EXECUTOR];
        let mut spec = SchemeSpec::default();
        spec.level1[ARCANE] = Some(30);
        assert!(build(&spec, g, &[], LIMITS).is_err());
        spec.level1[3] = Some(3);
        let (alloc, _) = build(&spec, g, &[], LIMITS).unwrap();
        check_invariants(&spec, &alloc, g);
        assert_eq!(alloc.at(15)[ARCANE], 30);
    }

    #[test]
    fn rejects_over_budget_and_out_of_range() {
        let g = &embedded()[EXECUTOR];
        let mut spec = SchemeSpec::default();
        spec.level1[0] = Some(60);
        assert!(build(&spec, g, &[], LIMITS).is_err());
        let mut spec = SchemeSpec::default();
        spec.level1[ARCANE] = Some(0);
        assert!(build(&spec, g, &[], LIMITS).is_err());
        let mut spec = SchemeSpec::default();
        spec.overrides.insert(2, [100, 0, 0, 0, 0, 0, 0]);
        assert!(build(&spec, g, &[], LIMITS).is_err());
    }

    #[test]
    fn under_allocated_level_is_filled() {
        let g = &embedded()[EXECUTOR];
        let mut spec = SchemeSpec { weights: [1, 0, 0, 0, 0, 0, 0], ..Default::default() };
        spec.overrides.insert(5, [1, 0, 0, 0, 0, 0, 0]);
        let (alloc, notes) = build(&spec, g, &[], LIMITS).unwrap();
        check_invariants(&spec, &alloc, g);
        assert!(notes.iter().any(|n| n.contains("5 级")));
    }

    #[test]
    fn extra_points_raise_budget() {
        let g = &embedded()[EXECUTOR];
        let extra = [0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1];
        let (alloc, _) = build(&SchemeSpec::default(), g, &extra, LIMITS).unwrap();
        assert_eq!(alloc.at(15).iter().sum::<u32>(), g.stats_at(15).iter().sum::<u32>() + 3);
    }
}
