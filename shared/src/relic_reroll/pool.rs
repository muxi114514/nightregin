//! 词条候选与抽取。候选 = 遗物这一栏原版抽奖表里权重 > 0 的词条，去掉其他角色的专属组、原词条本身，
//! 以及与遗物上保留下来的词条同类（compatibilityId 相同）的词条。
//! 通用组与武器组不按角色允许位过滤：初始武器可以互换，出击时改战技 / 魔法 / 附加类遗物会跟着武器生效。

use std::collections::HashSet;

use super::catalog::{GroupKind, RelicCatalog};
use super::effects::{RelicEffects, Target};
use super::price::{Cost, Prices};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Candidate {
    pub effect: u32,
    pub weight: u32,
    pub group: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RerollError {
    NotRerollable,
    EmptySlot(Target),
    NoCandidates(Target),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RerollResult {
    /// 重排后的遗物词条
    pub effects: RelicEffects,
    /// (原词条, 新词条)，与传入的位置顺序一致
    pub changes: Vec<(u32, u32)>,
}

impl RelicCatalog {
    /// 这个位置用的抽奖表：负面词条用遗物的负面表；正面词条在「同样带 / 不带负面」的栏里找，
    /// 优先含原词条的那张（普通遗物三栏的表候选相同，只是权重略有差别）
    pub fn table_for(&self, relic: u32, effects: &RelicEffects, target: Target) -> Option<u32> {
        let row = self.relic(relic)?;
        let list: Vec<i32> = match target {
            Target::Curse(_) => row.curse_tables.to_vec(),
            Target::Positive(i) => {
                let cursed = effects.curse.get(i).copied().flatten().is_some();
                (0..3).filter(|&k| self.table(row.curse_tables[k]).is_some() == cursed).map(|k| row.tables[k]).collect()
            }
        };
        let list: Vec<u32> = list.into_iter().filter(|&t| self.table(t).is_some()).map(|t| t as u32).collect();
        let current = effects.get(target);
        let has_current = |t: &&u32| current.is_some_and(|e| self.tables[*t].iter().any(|&(id, _)| id == e));
        list.iter().find(has_current).or(list.first()).copied()
    }

    /// 一个位置的全部候选（不看勾选）。replaced：同批重铸、之后会被换掉的位置——它们的原词条不再出现，也不参与同类判断
    pub fn candidates(&self, relic: u32, effects: &RelicEffects, target: Target, hero: u32, replaced: &[Target]) -> Vec<Candidate> {
        let Some(table) = self.table_for(relic, effects, target).and_then(|t| self.tables.get(&t)) else { return Vec::new() };
        let olds: Vec<u32> = std::iter::once(target).chain(replaced.iter().copied()).filter_map(|t| effects.get(t)).collect();
        let kept: Vec<u32> = effects.targets().filter(|t| *t != target && !replaced.contains(t)).filter_map(|t| effects.get(t)).collect();
        table
            .iter()
            .filter_map(|&(id, weight)| {
                let e = self.effect(id)?;
                let other_hero = matches!(self.group(e.group).map(|g| g.kind), Some(GroupKind::Hero(h)) if h as u32 != hero);
                let blocked = other_hero || olds.contains(&id) || kept.iter().any(|&k| self.conflicts(k, id));
                (!blocked).then_some(Candidate { effect: id, weight, group: e.group })
            })
            .collect()
    }

    /// 计价用的候选权重：(全部, 勾选后)；replaced 同 candidates
    pub fn weights(&self, relic: u32, effects: &RelicEffects, target: Target, hero: u32, replaced: &[Target], excluded: &HashSet<u32>) -> (u64, u64) {
        let all = self.candidates(relic, effects, target, hero, replaced);
        (total(&all), total(all.iter().filter(|c| !excluded.contains(&c.group))))
    }

    /// 一批位置的总价（各位置按自己的候选占比计价后相加）；有位置没有候选时为 None。界面显示与实际扣费都用它
    #[allow(clippy::too_many_arguments)]
    pub fn quote(
        &self,
        prices: &Prices,
        relic: u32,
        effects: &RelicEffects,
        targets: &[Target],
        hero: u32,
        excluded: &HashSet<u32>,
        exponent: f32,
    ) -> Option<Cost> {
        let deep = self.relic(relic)?.deep;
        let mut sum = Cost::default();
        for &t in targets {
            let others: Vec<Target> = targets.iter().copied().filter(|&o| o != t).collect();
            let (all, selected) = self.weights(relic, effects, t, hero, &others, excluded);
            sum = sum + prices.quote(deep, all, selected, exponent)?;
        }
        (!targets.is_empty()).then_some(sum)
    }

    /// 依次重铸各位置：先抽的结果参与后抽位置的同类判断，同批的原词条都不会再抽到（避免只是互换位置）；
    /// excluded 为取消勾选的分组，每抽一次调用一次 next_random
    pub fn reroll(
        &self,
        relic: u32,
        effects: &RelicEffects,
        targets: &[Target],
        hero: u32,
        excluded: &HashSet<u32>,
        mut next_random: impl FnMut() -> u64,
    ) -> Result<RerollResult, RerollError> {
        if !self.rerollable(relic) {
            return Err(RerollError::NotRerollable);
        }
        let olds: Vec<u32> = targets.iter().filter_map(|&t| effects.get(t)).collect();
        let mut state = *effects;
        let mut changes = Vec::with_capacity(targets.len());
        for (i, &target) in targets.iter().enumerate() {
            let old = state.get(target).ok_or(RerollError::EmptySlot(target))?;
            let pool: Vec<Candidate> = self
                .candidates(relic, &state, target, hero, &targets[i + 1..])
                .into_iter()
                .filter(|c| !excluded.contains(&c.group) && !olds.contains(&c.effect))
                .collect();
            let new = roll(&pool, next_random()).ok_or(RerollError::NoCandidates(target))?;
            state.set(target, new);
            changes.push((old, new));
        }
        Ok(RerollResult { effects: self.sort_effects(&state), changes })
    }

    /// 同一件遗物上不能共存：同一词条，或同属一个 compatibilityId 组
    pub fn conflicts(&self, a: u32, b: u32) -> bool {
        a == b || matches!((self.effect(a), self.effect(b)), (Some(x), Some(y)) if x.compat >= 0 && x.compat == y.compat)
    }
}

pub fn total<'a>(candidates: impl IntoIterator<Item = &'a Candidate>) -> u64 {
    candidates.into_iter().map(|c| c.weight as u64).sum()
}

/// 按权重抽一个；random 为任意随机数
pub fn roll(candidates: &[Candidate], random: u64) -> Option<u32> {
    let sum = total(candidates);
    if sum == 0 {
        return None;
    }
    let mut pick = random % sum;
    candidates.iter().find(|c| {
        let hit = pick < c.weight as u64;
        pick = pick.saturating_sub(c.weight as u64);
        hit
    })
    .map(|c| c.effect)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::relic_reroll::fixture::*;

    fn ids(c: &[Candidate]) -> Vec<u32> {
        let mut v: Vec<u32> = c.iter().map(|c| c.effect).collect();
        v.sort();
        v
    }

    #[test]
    fn standard_slot_excludes_self_conflicts_and_other_heroes() {
        let c = catalog();
        let e = RelicEffects { positive: [Some(VIGOR_1), Some(ATK_PHYS), None], curse: [None; 3] };
        // 追踪者重铸生命力 +1：去掉自己与守护者专属；物理攻击力留在遗物上，同组的物理 / 火攻击力都不能出
        let got = ids(&c.candidates(STANDARD, &e, Target::Positive(0), WYLDER, &[]));
        assert_eq!(got, [HERO_WYLDER, VIGOR_2, WEAPON_DAGGER]);
        // 物理攻击力同批会被换掉：火攻击力回到候选里，物理攻击力本身不再出现
        let got = ids(&c.candidates(STANDARD, &e, Target::Positive(0), WYLDER, &[Target::Positive(1)]));
        assert_eq!(got, [HERO_WYLDER, VIGOR_2, ATK_FIRE, WEAPON_DAGGER]);
        // 守护者看到的是自己的专属组
        let got = ids(&c.candidates(STANDARD, &e, Target::Positive(0), GUARDIAN, &[]));
        assert_eq!(got, [HERO_GUARDIAN, VIGOR_2, WEAPON_DAGGER]);
    }

    #[test]
    fn deep_slots_use_strong_or_plain_tables() {
        let c = catalog();
        let e = RelicEffects { positive: [Some(STRONG_ATK), Some(VIGOR_1), None], curse: [Some(CURSE_A), None, None] };
        assert_eq!(c.table_for(DEEP, &e, Target::Positive(0)), Some(T_STRONG));
        assert_eq!(c.table_for(DEEP, &e, Target::Positive(1)), Some(T_PLAIN));
        assert_eq!(c.table_for(DEEP, &e, Target::Curse(0)), Some(T_CURSE));
        assert_eq!(ids(&c.candidates(DEEP, &e, Target::Positive(0), WYLDER, &[])), [STRONG_DEF]);
        assert_eq!(ids(&c.candidates(DEEP, &e, Target::Curse(0), WYLDER, &[])), [CURSE_B]);
    }

    #[test]
    fn rerolls_in_order_and_sorts_result() {
        let c = catalog();
        let e = RelicEffects { positive: [Some(ATK_PHYS), Some(VIGOR_1), None], curse: [None; 3] };
        let excluded = HashSet::from([G_HERO_WYLDER, G_WEAPON]);
        // 两条一起重铸、只留通用组，随机数总取 0（抽候选里第一个）：两条原词条都不再出现；
        // 第一条在 [生命力+2, 火攻击力] 里抽到生命力 +2，第二条避开同类的生命力，只剩火攻击力；结果按排序键重排
        let r = c.reroll(STANDARD, &e, &[Target::Positive(0), Target::Positive(1)], WYLDER, &excluded, || 0).unwrap();
        assert_eq!(r.changes, [(ATK_PHYS, VIGOR_2), (VIGOR_1, ATK_FIRE)]);
        assert_eq!(r.effects.positive, [Some(VIGOR_2), Some(ATK_FIRE), None]);
        // 全部组都取消：没有候选
        let none: HashSet<u32> = c.groups.keys().copied().collect();
        assert_eq!(c.reroll(STANDARD, &e, &[Target::Positive(0)], WYLDER, &none, || 0), Err(RerollError::NoCandidates(Target::Positive(0))));
        assert_eq!(c.reroll(FIXED, &e, &[Target::Positive(0)], WYLDER, &excluded, || 0), Err(RerollError::NotRerollable));
        assert_eq!(c.reroll(STANDARD, &e, &[Target::Positive(2)], WYLDER, &excluded, || 0), Err(RerollError::EmptySlot(Target::Positive(2))));
    }

    #[test]
    fn quote_sums_targets_and_uses_deep_price() {
        let c = catalog();
        let p = crate::relic_reroll::DEFAULT_PRICES;
        let e = RelicEffects { positive: [Some(ATK_PHYS), Some(VIGOR_1), None], curse: [None; 3] };
        let none = HashSet::new();
        // 两条全选各一份基础价；不选位置不报价
        assert_eq!(c.quote(&p, STANDARD, &e, &[Target::Positive(0), Target::Positive(1)], WYLDER, &none, 0.5), Some(Cost { murk: 1200, sigils: 10 }));
        assert_eq!(c.quote(&p, STANDARD, &e, &[], WYLDER, &none, 0.5), None);
        // 深夜遗物按深夜价；负面栏只剩一个候选，勾掉减益组就没有候选
        let d = RelicEffects { positive: [Some(STRONG_ATK), None, None], curse: [Some(CURSE_A), None, None] };
        assert_eq!(c.quote(&p, DEEP, &d, &[Target::Curse(0)], WYLDER, &none, 0.5), Some(p.deep));
        assert_eq!(c.quote(&p, DEEP, &d, &[Target::Curse(0)], WYLDER, &HashSet::from([G_DEBUFF]), 0.5), None);
    }

    #[test]
    fn roll_follows_weights() {
        let pool = [Candidate { effect: 1, weight: 10, group: 0 }, Candidate { effect: 2, weight: 30, group: 0 }];
        assert_eq!([0, 9, 10, 39, 40].map(|r| roll(&pool, r)), [Some(1), Some(1), Some(2), Some(2), Some(1)]);
        assert_eq!(roll(&[], 5), None);
    }
}
