//! 一件遗物上的词条：3 个正面栏，以及同栏的负面词条（只有深夜遗物的强力栏才有）。

use super::catalog::RelicCatalog;

pub const SLOTS: usize = 3;

/// 遗物上的一个词条位置
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Target {
    Positive(usize),
    Curse(usize),
}

impl Target {
    pub fn slot(self) -> usize {
        match self {
            Self::Positive(i) | Self::Curse(i) => i,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RelicEffects {
    pub positive: [Option<u32>; SLOTS],
    pub curse: [Option<u32>; SLOTS],
}

impl RelicEffects {
    pub fn get(&self, target: Target) -> Option<u32> {
        match target {
            Target::Positive(i) => self.positive.get(i).copied().flatten(),
            Target::Curse(i) => self.curse.get(i).copied().flatten(),
        }
    }

    pub fn set(&mut self, target: Target, id: u32) {
        let slot = match target {
            Target::Positive(i) => self.positive.get_mut(i),
            Target::Curse(i) => self.curse.get_mut(i),
        };
        if let Some(s) = slot {
            *s = Some(id);
        }
    }

    /// 非空位置，按栏依次为正面、负面
    pub fn targets(&self) -> impl Iterator<Item = Target> + '_ {
        (0..SLOTS).flat_map(|i| [Target::Positive(i), Target::Curse(i)]).filter(|&t| self.get(t).is_some())
    }
}

impl RelicCatalog {
    /// 正面栏按 (排序键, ID) 升序重排，负面词条跟着同栏的正面词条走，空栏在最后（与原版生成的遗物顺序一致）
    pub fn sort_effects(&self, effects: &RelicEffects) -> RelicEffects {
        let key = |id: u32| (self.effect(id).map_or(i32::MAX, |e| e.sort_key), id);
        let mut slots: Vec<(Option<u32>, Option<u32>)> = effects.positive.into_iter().zip(effects.curse).collect();
        slots.sort_by_key(|&(p, _)| p.map_or((1, (0, 0)), |id| (0, key(id))));
        let mut out = RelicEffects::default();
        for (i, (p, c)) in slots.into_iter().enumerate() {
            out.positive[i] = p;
            out.curse[i] = c;
        }
        out
    }

    /// 遗物实例里出征时施加的 SpEffect 列表：先正面后负面，每条词条的常驻 SpEffect 依次排列、跳过 -1
    /// （实机 1502 件遗物全部符合）。有词条不在参数表里时为 None
    pub fn sp_effect_cache(&self, effects: &RelicEffects) -> Option<Vec<i32>> {
        let ids = effects.positive.iter().chain(&effects.curse).flatten();
        let mut out = Vec::new();
        for &id in ids {
            out.extend(self.effect(id)?.passive.into_iter().filter(|&s| s != -1));
        }
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::relic_reroll::fixture::*;

    #[test]
    fn sorts_positive_slots_and_keeps_curses_paired() {
        let c = catalog();
        let effects = RelicEffects { positive: [Some(ATK_PHYS), None, Some(HERO_WYLDER)], curse: [Some(CURSE_A), None, None] };
        let sorted = c.sort_effects(&effects);
        assert_eq!(sorted.positive, [Some(HERO_WYLDER), Some(ATK_PHYS), None]);
        assert_eq!(sorted.curse, [None, Some(CURSE_A), None]);
    }

    #[test]
    fn cache_lists_positive_then_curse_sp_effects() {
        let c = catalog();
        let effects = RelicEffects { positive: [Some(VIGOR_1), None, Some(ATK_PHYS)], curse: [None, None, Some(CURSE_A)] };
        let v = VIGOR_1 as i32;
        assert_eq!(c.sp_effect_cache(&effects), Some(vec![v + 1, v + 2, ATK_PHYS as i32 + 1, CURSE_A as i32 + 1]));
        assert_eq!(c.sp_effect_cache(&RelicEffects { positive: [Some(1), None, None], ..Default::default() }), None);
    }

    #[test]
    fn get_set_and_targets() {
        let mut e = RelicEffects::default();
        e.set(Target::Positive(1), 5);
        e.set(Target::Curse(1), 6);
        e.set(Target::Curse(SLOTS), 7);
        assert_eq!((e.get(Target::Positive(1)), e.get(Target::Curse(1)), e.get(Target::Curse(SLOTS))), (Some(5), Some(6), None));
        assert_eq!(e.targets().collect::<Vec<_>>(), [Target::Positive(1), Target::Curse(1)]);
    }
}
