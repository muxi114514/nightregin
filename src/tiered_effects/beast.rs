//! 执行者4：兽化（或执行者8 的熔炉形态）期间锁满血。不死在变身期间一直挂着、结束就移除（短时补挂会在到期那一帧留空档），
//! 血量低于上限就回满；回满后等一小会再判断，免得游戏还没加上血就重复施加。

use std::time::{Duration, Instant};

use nr_stat_alloc_shared::tiered_beast::{BEAST_FORM, BEAST_HEAL, BEAST_HP_LOCK, BEAST_UNDYING, CRUCIBLE_FORM};

use super::Frame;

const HEAL_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Default)]
pub struct BeastLock {
    next_heal: Option<Instant>,
}

impl BeastLock {
    pub fn check(&mut self, frame: &Frame, mut apply: impl FnMut(i32), mut remove: impl FnMut(i32)) {
        let (hp, max) = frame.hp;
        let transformed = frame.ids.contains(&BEAST_FORM) || frame.ids.contains(&CRUCIBLE_FORM);
        let active = frame.ids.contains(&BEAST_HP_LOCK) && transformed && hp > 0;
        let undying = frame.ids.contains(&BEAST_UNDYING);
        if active && !undying {
            apply(BEAST_UNDYING);
        } else if !active && undying {
            remove(BEAST_UNDYING);
        }
        if !active {
            return;
        }
        let now = Instant::now();
        if hp < max && self.next_heal.is_none_or(|t| now >= t) {
            apply(BEAST_HEAL);
            self.next_heal = Some(now + HEAL_INTERVAL);
        }
    }

    pub fn reset(&mut self) {
        self.next_heal = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(ids: &[i32], hp: u32) -> Frame<'_> {
        Frame { ids, hp: (hp, 1000), runes: 0 }
    }

    #[test]
    fn only_in_beast_form_with_buff() {
        let mut b = BeastLock::default();
        let mut applied = Vec::new();
        b.check(&frame(&[BEAST_HP_LOCK], 500), |id| applied.push(id), |_| {});
        b.check(&frame(&[BEAST_FORM], 500), |id| applied.push(id), |_| {});
        assert!(applied.is_empty());
    }

    #[test]
    fn keeps_undying_and_heals_when_hurt() {
        let mut b = BeastLock::default();
        let mut applied = Vec::new();
        b.check(&frame(&[BEAST_HP_LOCK, BEAST_FORM], 1000), |id| applied.push(id), |_| {});
        assert_eq!(applied, vec![BEAST_UNDYING]);
        applied.clear();
        b.check(&frame(&[BEAST_HP_LOCK, BEAST_FORM, BEAST_UNDYING], 1), |id| applied.push(id), |_| {});
        assert_eq!(applied, vec![BEAST_HEAL]);
        applied.clear();
        // 间隔内不重复回血
        b.check(&frame(&[BEAST_HP_LOCK, BEAST_FORM, BEAST_UNDYING], 1), |id| applied.push(id), |_| {});
        assert!(applied.is_empty());
    }

    #[test]
    fn crucible_form_counts_as_transformed() {
        let mut b = BeastLock::default();
        let mut applied = Vec::new();
        b.check(&frame(&[BEAST_HP_LOCK, CRUCIBLE_FORM], 500), |id| applied.push(id), |_| {});
        assert_eq!(applied, vec![BEAST_UNDYING, BEAST_HEAL]);
    }

    #[test]
    fn removes_undying_when_beast_ends() {
        let mut b = BeastLock::default();
        let mut removed = Vec::new();
        b.check(&frame(&[BEAST_HP_LOCK, BEAST_UNDYING], 1000), |_| {}, |id| removed.push(id));
        assert_eq!(removed, vec![BEAST_UNDYING]);
    }
}
