//! 分配面板的编辑模型：在 SchemeSpec 上做逐级加减，每次改动都用 rules::build 校验并刷新预览，
//! 不合规的改动直接撤回并返回原因。与 ImGui 无关，可单元测试。
//! 自动等级（没手动改过的等级）按成长权重填充；第一次改动某级时，先把它当前显示的值固定为手动分配，数字不会跳。

use crate::stats::growth::{ARCANE, GROWTH_STATS, HeroGrowth, MAX_LEVEL, Stats};
use crate::stats::rules::{self, Allocation, Gains, Limits, SchemeSpec, WeaponPick};
use crate::stats::{STAT_COUNT, stat_name};

/// 成长权重的上限（面板里 0~9）
pub const MAX_WEIGHT: u32 = 9;

/// 某一级在面板上的显示数据
#[derive(Debug, Clone, PartialEq)]
pub struct LevelView {
    pub level: u32,
    /// 本级可分配的点数（含感应返还）
    pub budget: u32,
    /// 其中来自 1 级感应减少的返还点
    pub refund: u32,
    /// 本级已分配的点数（剩余 = budget - assigned，未分配的点保存后按成长权重补齐）
    pub assigned: u32,
    /// 是否手动分配过（否则按成长权重自动填充）
    pub manual: bool,
    /// 上一级的最终属性（1 级为原版 1 级）
    pub before: Stats,
    /// 本级分配后的属性
    pub after: Stats,
    pub vanilla: Stats,
}

pub struct Draft {
    growth: HeroGrowth,
    limits: Limits,
    spec: SchemeSpec,
    /// 撤销修改时回到的方案
    baseline: SchemeSpec,
    /// baseline 是否就是该目标已保存的方案（器皿还没有专属方案、从默认方案起步时为 false）
    persisted: bool,
    budgets: Vec<u32>,
    refund: [u32; MAX_LEVEL as usize],
    preview: Allocation,
    notes: Vec<String>,
}

impl Draft {
    /// spec 不合规（例如上限配置改小了）时退回原版成长，并在提示里说明
    pub fn new(growth: HeroGrowth, limits: Limits, spec: SchemeSpec) -> Self {
        let (spec, warning) = match rules::build(&spec, &growth, &[], limits) {
            Ok(_) => (spec, None),
            // 属性退回原版，初始武器保留
            Err(e) => (
                SchemeSpec { weapons: spec.weapons, ..Default::default() },
                Some(crate::tr!("原方案不合规（{e}），已改为从原版成长开始编辑", "The saved scheme is invalid ({e}); editing starts from vanilla growth")),
            ),
        };
        let (preview, mut notes) = rules::build(&spec, &growth, &[], limits).expect("vanilla growth is always valid");
        notes.extend(warning);
        let budgets = rules::budgets(&spec, &growth, &[]);
        let refund = rules::arcane_refund(&spec, &growth);
        Self { growth, limits, baseline: spec.clone(), persisted: true, spec, budgets, refund, preview, notes }
    }

    pub fn spec(&self) -> &SchemeSpec {
        &self.spec
    }

    /// 是否有尚未保存到该目标的内容
    pub fn dirty(&self) -> bool {
        !self.persisted || self.spec != self.baseline
    }

    /// 起点不是该目标自己保存的方案（例如从默认方案复制来），即使没改动也算未保存
    pub fn mark_unsaved(&mut self) {
        self.persisted = false;
    }

    /// 最近一次校验产生的提示（自动补点等）
    pub fn notes(&self) -> &[String] {
        &self.notes
    }

    pub fn weights(&self) -> Gains {
        self.spec.weights
    }

    pub fn weapons(&self) -> &WeaponPick {
        &self.spec.weapons
    }

    /// 初始武器不影响属性规则，直接替换（合法性由面板按初始装备规则检查）
    pub fn set_weapons(&mut self, pick: WeaponPick) {
        self.spec.weapons = pick;
    }

    /// 预览：方案展开后的最终属性
    pub fn final_at(&self, level: u32) -> Stats {
        self.preview.at(level)
    }

    pub fn vanilla_at(&self, level: u32) -> Stats {
        self.growth.stats_at(level)
    }

    pub fn is_manual(&self, level: u32) -> bool {
        if level == 1 { self.spec.level1.iter().any(Option::is_some) } else { self.spec.overrides.contains_key(&level) }
    }

    pub fn view(&self, level: u32) -> LevelView {
        let level = level.clamp(1, MAX_LEVEL);
        let budget = self.budgets[level as usize - 1];
        let refund = self.refund[level as usize - 1];
        let vanilla = self.vanilla_at(level);
        let manual = self.is_manual(level);
        if level == 1 {
            let after = self.level1_values();
            let assigned = if manual { after.iter().sum() } else { budget };
            return LevelView { level, budget, refund, assigned, manual, before: vanilla, after, vanilla };
        }
        let before = self.final_at(level - 1);
        let (after, assigned) = match self.spec.overrides.get(&level) {
            Some(gains) => {
                let mut after = before;
                (0..GROWTH_STATS).for_each(|i| after[i] += gains[i]);
                (after, gains.iter().sum())
            }
            None => {
                let after = self.final_at(level);
                (after, (0..GROWTH_STATS).map(|i| after[i] - before[i]).sum())
            }
        };
        LevelView { level, budget, refund, assigned, manual, before, after, vanilla }
    }

    /// 某级某项加减 delta 点；不合规时撤回并返回原因
    pub fn adjust(&mut self, level: u32, stat: usize, delta: i32) -> Result<(), String> {
        if !(1..=MAX_LEVEL).contains(&level) || stat >= STAT_COUNT || delta == 0 {
            return Ok(());
        }
        let view = self.view(level);
        let name = stat_name(stat);
        let (min, cap) = (self.limits.min, self.limits.cap);
        let next = view.after[stat] as i64 + delta as i64;
        if next > self.limits.cap as i64 {
            return Err(crate::tr!("{name}不能超过上限 {cap}", "{name} cannot exceed {cap}"));
        }
        let mut spec = self.spec.clone();
        if level == 1 {
            if next < self.limits.min as i64 {
                return Err(crate::tr!("{name}不能低于 {min}", "{name} cannot go below {min}"));
            }
            let mut l1 = view.after;
            l1[stat] = next as u32;
            spec.level1 = l1.map(Some);
            // 改感应会同时改变 1 级可用点数，所以按改动后的预算判断
            if delta > 0 && l1.iter().sum::<u32>() > rules::budgets(&spec, &self.growth, &[])[0] {
                return Err(crate::tr!("1 级点数已分完，先减少其他属性", "No Lv1 points left; lower another attribute first"));
            }
        } else {
            if stat == ARCANE {
                return Err(crate::tr!("感应只能在 1 级分配", "Arcane can only be allocated at Lv1"));
            }
            if delta > 0 && view.assigned >= view.budget {
                return Err(crate::tr!("{level} 级点数已分完，先减少其他属性，或点「清空本级」", "No Lv{level} points left; lower another attribute or use Clear level"));
            }
            if next < view.before[stat] as i64 {
                let prev = view.before[stat];
                return Err(crate::tr!("{name}不能低于上一级（{prev}）", "{name} cannot go below the previous level ({prev})"));
            }
            let mut gains: Gains = std::array::from_fn(|i| view.after[i] - view.before[i]);
            gains[stat] = (next - view.before[stat] as i64) as u32;
            spec.overrides.insert(level, gains);
        }
        self.apply(spec)
    }

    /// 本级恢复按成长权重自动分配（1 级恢复原版数值）
    pub fn reset_level(&mut self, level: u32) -> Result<(), String> {
        let mut spec = self.spec.clone();
        if level == 1 {
            spec.level1 = [None; STAT_COUNT];
        } else {
            spec.overrides.remove(&level);
        }
        self.apply(spec)
    }

    /// 清空本级：所有新增点退回待分配（1 级每项退到下限）
    pub fn clear_level(&mut self, level: u32) -> Result<(), String> {
        let mut spec = self.spec.clone();
        if level == 1 {
            spec.level1 = [Some(self.limits.min); STAT_COUNT];
        } else {
            spec.overrides.insert(level, [0; GROWTH_STATS]);
        }
        self.apply(spec)
    }

    pub fn set_weight(&mut self, stat: usize, weight: u32) -> Result<(), String> {
        if stat >= GROWTH_STATS {
            return Ok(());
        }
        let mut spec = self.spec.clone();
        spec.weights[stat] = weight.min(MAX_WEIGHT);
        self.apply(spec)
    }

    /// 整个方案恢复原版成长
    pub fn reset_all(&mut self) {
        let _ = self.apply(SchemeSpec::default());
    }

    /// 放弃未保存的修改，回到起点
    pub fn revert(&mut self) {
        let baseline = self.baseline.clone();
        let _ = self.apply(baseline);
    }

    fn apply(&mut self, spec: SchemeSpec) -> Result<(), String> {
        let (preview, notes) = rules::build(&spec, &self.growth, &[], self.limits)?;
        self.budgets = rules::budgets(&spec, &self.growth, &[]);
        self.refund = rules::arcane_refund(&spec, &self.growth);
        self.spec = spec;
        self.preview = preview;
        self.notes = notes;
        Ok(())
    }

    /// 1 级当前值：手动分配过的取写明的值（没写的取原版），否则取展开结果
    fn level1_values(&self) -> Stats {
        if self.is_manual(1) {
            let v1 = self.vanilla_at(1);
            std::array::from_fn(|i| self.spec.level1[i].unwrap_or(v1[i]))
        } else {
            self.final_at(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::vanilla::embedded;

    const LIMITS: Limits = Limits { min: 1, cap: 70 };
    const EXECUTOR: usize = 7;

    fn draft() -> Draft {
        Draft::new(embedded()[EXECUTOR].clone(), LIMITS, SchemeSpec::default())
    }

    #[test]
    fn weapons_mark_dirty_and_survive_invalid_stats() {
        let mut d = draft();
        d.set_weapons(WeaponPick { main: Some("追踪者".into()), off: None });
        assert!(d.dirty());
        d.revert();
        assert!(d.weapons().is_vanilla());
        // 属性不合规时只退回属性，武器保留
        let mut bad = SchemeSpec { weapons: WeaponPick { main: Some("女爵".into()), off: None }, ..Default::default() };
        bad.level1[0] = Some(99);
        let d = Draft::new(embedded()[EXECUTOR].clone(), LIMITS, bad);
        assert_eq!(d.weapons().main.as_deref(), Some("女爵"));
        assert_eq!(d.spec().level1[0], None);
    }

    #[test]
    fn vanilla_levels_are_full_and_auto() {
        let d = draft();
        for level in 1..=MAX_LEVEL {
            let v = d.view(level);
            assert!(!v.manual);
            assert_eq!(v.assigned, v.budget, "level {level}");
            assert_eq!(v.after, d.vanilla_at(level));
        }
        assert!(!d.dirty());
    }

    #[test]
    fn full_level_requires_decrease_first() {
        let mut d = draft();
        let before = d.view(3);
        let from = (0..GROWTH_STATS).find(|&i| before.after[i] > before.before[i]).unwrap();
        let to = (0..GROWTH_STATS).find(|&i| i != from).unwrap();
        assert!(d.adjust(3, to, 1).is_err());
        d.adjust(3, from, -1).unwrap();
        d.adjust(3, to, 1).unwrap();
        let after = d.view(3);
        assert!(after.manual && d.dirty());
        assert_eq!(after.after[to], before.after[to] + 1);
        assert_eq!(after.after[from], before.after[from] - 1);
        assert_eq!(after.assigned, after.budget);
    }

    #[test]
    fn cannot_go_below_previous_level_or_touch_arcane_later() {
        let mut d = draft();
        d.clear_level(2).unwrap();
        assert!(d.adjust(2, 0, -1).is_err());
        assert!(d.adjust(2, ARCANE, 1).is_err());
        let v = d.view(2);
        assert_eq!(v.assigned, 0);
        assert_eq!(v.after, v.before);
    }

    #[test]
    fn lowering_arcane_refunds_late_levels_not_level1() {
        let mut d = draft();
        let (b1, b15, b14) = (d.view(1).budget, d.view(15).budget, d.view(14).budget);
        d.adjust(1, ARCANE, -2).unwrap();
        let v1 = d.view(1);
        // 1 级仍是满的，只是总量少了 2；15、14 级各多 1 点
        assert_eq!((v1.budget, v1.assigned), (b1 - 2, b1 - 2));
        assert!(d.adjust(1, 0, 1).is_err());
        assert_eq!((d.view(15).budget, d.view(15).refund), (b15 + 1, 1));
        assert_eq!(d.view(14).budget, b14 + 1);
        assert_eq!(d.final_at(15)[ARCANE], 26);
        // 加回感应不受"1 级已满"限制
        d.adjust(1, ARCANE, 1).unwrap();
        assert_eq!(d.view(15).refund, 1);
        assert_eq!(d.view(14).refund, 0);
    }

    #[test]
    fn raising_arcane_takes_level1_points() {
        let mut d = draft();
        assert!(d.adjust(1, ARCANE, 1).is_err());
        d.adjust(1, 4, -2).unwrap();
        d.adjust(1, ARCANE, 2).unwrap();
        let v = d.view(1);
        assert_eq!(v.after, [7, 2, 3, 5, 6, 2, 2, 30]);
        assert_eq!(v.assigned, v.budget);
        assert!(d.adjust(2, ARCANE, 1).is_err());
    }

    #[test]
    fn edits_that_break_later_levels_are_rejected() {
        let mut d = draft();
        // 感应减 1 点让 15 级多 1 点，再把 15 级手动分满；此时把感应加回去，15 级就超额了
        d.adjust(1, ARCANE, -1).unwrap();
        d.clear_level(15).unwrap();
        let budget15 = d.view(15).budget as i32;
        d.adjust(15, 1, budget15).unwrap();
        let err = d.adjust(1, ARCANE, 1).unwrap_err();
        assert!(err.contains("15 级"), "{err}");
        assert_eq!(d.view(1).after[ARCANE], 27);
    }

    #[test]
    fn revert_and_reset_restore_state() {
        let spec = SchemeSpec { weights: [5, 0, 0, 0, 0, 0, 0], ..Default::default() };
        let mut d = Draft::new(embedded()[EXECUTOR].clone(), LIMITS, spec);
        d.adjust(1, ARCANE, -3).unwrap();
        d.adjust(4, 0, -1).unwrap_or(());
        d.clear_level(5).unwrap();
        assert!(d.dirty());
        d.revert();
        assert!(!d.dirty());
        assert_eq!(d.weights()[0], 5);
        d.reset_all();
        assert_eq!(d.spec(), &SchemeSpec::default());
        let mut d = draft();
        d.mark_unsaved();
        d.revert();
        assert!(d.dirty());
    }
}
