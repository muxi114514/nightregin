//! 金6（shared::tiered_cold_mirage）：拿到时挂上金6 自己的「可用」（引擎读的效果号由 tiered_params 换成这一对），
//! 每次触发（「无敌隐身」出现）补满血量并挂 30 秒增益。
//! 身上有原版「冷冽海市蜃楼」的效果就是另外拿过原版（此后引擎不会再用掉它），多 1 次：「已用」且隐身结束后把「可用」挂回去，
//! 赐福休息（或引擎自己挂回「可用」）时补满。

use std::time::{Duration, Instant};

use log::info;
use nr_stat_alloc_shared::tiered_cold_mirage::{GOLD_MIRAGE, MIRAGE_ACTIVE, MIRAGE_BUFF, MIRAGE_HEAL, MIRAGE_READY, MIRAGE_USED, VANILLA_READY, VANILLA_USED};
use nr_stat_alloc_shared::tiered_flasks::GRACE_RELOAD;

use super::Frame;

/// 隐身结束后再等一会才挂回「可用」
const RESTORE_DELAY: Duration = Duration::from_secs(1);
/// 自己挂上后这段时间内出现的「可用」不算引擎补充
const OWN_WINDOW: Duration = Duration::from_secs(1);

#[derive(Default)]
pub struct ColdMirage {
    owned: bool,
    /// 另外拿过原版这条锻体
    vanilla: bool,
    /// 剩余的额外次数
    extra: u32,
    had_ready: bool,
    had_active: bool,
    had_grace: bool,
    restore_at: Option<Instant>,
    restored_at: Option<Instant>,
}

impl ColdMirage {
    pub fn check(&mut self, frame: &Frame, now: Instant, mut apply: impl FnMut(i32), mut remove: impl FnMut(i32)) {
        let has = |id: i32| frame.ids.contains(&id);
        let (owned, ready, active, used, grace) = (has(GOLD_MIRAGE), has(MIRAGE_READY), has(MIRAGE_ACTIVE), has(MIRAGE_USED), has(GRACE_RELOAD));
        if owned && !self.owned {
            // 原版的「已用」之后没人清，换回原版的「可用」（同类别同优先级，顶掉它）留作拿过原版的标记
            if has(VANILLA_USED) {
                apply(VANILLA_READY);
            }
            apply(MIRAGE_READY);
            self.restored_at = Some(now);
            info!("cold mirage: gained");
        }
        if owned && !self.vanilla && (has(VANILLA_READY) || has(VANILLA_USED)) {
            self.vanilla = true;
            self.extra = 1;
            info!("cold mirage: vanilla buff held, extra use ready");
        }
        let ready_edge = ready && !self.had_ready && !self.restored_at.take().is_some_and(|t| now < t + OWN_WINDOW);
        if owned && self.vanilla && self.extra == 0 && (ready_edge || (grace && !self.had_grace)) {
            self.extra = 1;
            info!("cold mirage: extra use ready");
        }
        if owned && active && !self.had_active {
            apply(MIRAGE_HEAL);
            apply(MIRAGE_BUFF);
            info!("cold mirage: triggered, healed and buffed");
        }
        (self.owned, self.had_ready, self.had_active, self.had_grace) = (owned, ready, active, grace);
        if !(owned && used && !ready && !active && self.extra > 0 && frame.hp.0 > 0) {
            self.restore_at = None;
            return;
        }
        if now < *self.restore_at.get_or_insert(now + RESTORE_DELAY) {
            return;
        }
        remove(MIRAGE_USED);
        apply(MIRAGE_READY);
        self.extra -= 1;
        self.restore_at = None;
        self.restored_at = Some(now);
        info!("cold mirage: restored, extra left {}", self.extra);
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 跑一帧，返回施加的效果
    fn step(m: &mut ColdMirage, ids: &[i32], now: Instant) -> Vec<i32> {
        let mut applied = Vec::new();
        m.check(&Frame { ids, hp: (500, 1000), runes: 0 }, now, |id| applied.push(id), |_| {});
        applied
    }

    /// 触发的那一帧：「无敌隐身」与「已用」出现，返回施加的效果
    fn trigger(m: &mut ColdMirage, base: &[i32], at: Instant) -> Vec<i32> {
        let ids: Vec<i32> = base.iter().copied().chain([MIRAGE_ACTIVE, MIRAGE_USED]).collect();
        step(m, &ids, at)
    }

    #[test]
    fn grants_own_ready_and_buffs_on_trigger() {
        let mut m = ColdMirage::default();
        let t0 = Instant::now();
        assert_eq!(step(&mut m, &[GOLD_MIRAGE], t0), vec![MIRAGE_READY]);
        step(&mut m, &[GOLD_MIRAGE, MIRAGE_READY], t0);
        assert_eq!(trigger(&mut m, &[GOLD_MIRAGE], t0), vec![MIRAGE_HEAL, MIRAGE_BUFF]);
        // 没有原版：用掉就没有了
        step(&mut m, &[GOLD_MIRAGE, MIRAGE_USED], t0 + RESTORE_DELAY);
        assert!(step(&mut m, &[GOLD_MIRAGE, MIRAGE_USED], t0 + RESTORE_DELAY * 3).is_empty());
    }

    #[test]
    fn vanilla_first_gets_a_second_use_refilled_at_grace() {
        let mut m = ColdMirage::default();
        let t0 = Instant::now();
        step(&mut m, &[VANILLA_READY], t0);
        assert_eq!(step(&mut m, &[GOLD_MIRAGE, VANILLA_READY], t0), vec![MIRAGE_READY]);
        step(&mut m, &[GOLD_MIRAGE, VANILLA_READY, MIRAGE_READY], t0);
        trigger(&mut m, &[GOLD_MIRAGE, VANILLA_READY], t0);
        // 隐身结束、等满延迟才挂回
        assert!(step(&mut m, &[GOLD_MIRAGE, VANILLA_READY, MIRAGE_USED], t0 + RESTORE_DELAY).is_empty());
        assert_eq!(step(&mut m, &[GOLD_MIRAGE, VANILLA_READY, MIRAGE_USED], t0 + RESTORE_DELAY * 2), vec![MIRAGE_READY]);
        step(&mut m, &[GOLD_MIRAGE, VANILLA_READY, MIRAGE_READY], t0 + RESTORE_DELAY * 2);
        trigger(&mut m, &[GOLD_MIRAGE, VANILLA_READY], t0 + RESTORE_DELAY * 3);
        step(&mut m, &[GOLD_MIRAGE, VANILLA_READY, MIRAGE_USED], t0 + RESTORE_DELAY * 4);
        assert!(step(&mut m, &[GOLD_MIRAGE, VANILLA_READY, MIRAGE_USED], t0 + RESTORE_DELAY * 6).is_empty());
        // 赐福：两次都恢复（引擎挂回一次，DLL 补回额外一次）
        step(&mut m, &[GOLD_MIRAGE, VANILLA_READY, MIRAGE_READY, GRACE_RELOAD], t0 + RESTORE_DELAY * 7);
        trigger(&mut m, &[GOLD_MIRAGE, VANILLA_READY], t0 + RESTORE_DELAY * 8);
        step(&mut m, &[GOLD_MIRAGE, VANILLA_READY, MIRAGE_USED], t0 + RESTORE_DELAY * 9);
        assert_eq!(step(&mut m, &[GOLD_MIRAGE, VANILLA_READY, MIRAGE_USED], t0 + RESTORE_DELAY * 10), vec![MIRAGE_READY]);
    }

    #[test]
    fn vanilla_picked_after_gold_is_recognised() {
        let mut m = ColdMirage::default();
        let t0 = Instant::now();
        step(&mut m, &[GOLD_MIRAGE], t0);
        step(&mut m, &[GOLD_MIRAGE, MIRAGE_READY], t0);
        // 后拿原版：引擎读的是金6 那一对，原版效果多出来
        step(&mut m, &[GOLD_MIRAGE, MIRAGE_READY, VANILLA_READY], t0);
        trigger(&mut m, &[GOLD_MIRAGE, VANILLA_READY], t0);
        step(&mut m, &[GOLD_MIRAGE, VANILLA_READY, MIRAGE_USED], t0 + RESTORE_DELAY);
        assert_eq!(step(&mut m, &[GOLD_MIRAGE, VANILLA_READY, MIRAGE_USED], t0 + RESTORE_DELAY * 2), vec![MIRAGE_READY]);
    }

    #[test]
    fn vanilla_used_before_gold_becomes_the_marker() {
        let mut m = ColdMirage::default();
        let t0 = Instant::now();
        step(&mut m, &[VANILLA_USED], t0);
        assert_eq!(step(&mut m, &[GOLD_MIRAGE, VANILLA_USED], t0), vec![VANILLA_READY, MIRAGE_READY]);
    }

    #[test]
    fn nothing_without_the_buff() {
        let mut m = ColdMirage::default();
        let t0 = Instant::now();
        assert!(step(&mut m, &[VANILLA_READY], t0).is_empty());
        assert!(step(&mut m, &[MIRAGE_ACTIVE, VANILLA_USED], t0).is_empty());
    }
}
