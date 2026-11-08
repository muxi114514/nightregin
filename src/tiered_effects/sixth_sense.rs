//! 追踪者12（shared::tiered_sixth_sense）：第六感用掉（「已用」出现）后若还有额外次数，等回避动作做完把引擎里的次数改回 1
//! （game::sixth_sense），引擎下一帧自己挂回「可用」；拿到锻体时给 1 次，引擎自己挂回「可用」（出击开始、赐福休息）时补满。

use std::time::{Duration, Instant};

use log::{info, warn};
use nr_stat_alloc_shared::tiered_sixth_sense::{EXTRA_SIXTH_SENSE, SIXTH_SENSE, SIXTH_SENSE_USED};

use super::Frame;

/// 触发后的回避动作做完再恢复，免得同一套连击把额外那次也吃掉
const RESTORE_DELAY: Duration = Duration::from_secs(2);
/// 恢复后这段时间内出现的「可用」算自己的，不当作引擎补充
const OWN_WINDOW: Duration = Duration::from_secs(1);
/// 写不进去（组件还没建好等）时隔一会再试
const RETRY: Duration = Duration::from_secs(1);

#[derive(Default)]
pub struct SixthSense {
    /// 剩余的额外次数
    extra: u32,
    owned: bool,
    had_ready: bool,
    restore_at: Option<Instant>,
    restored_at: Option<Instant>,
}

impl SixthSense {
    /// restore：把引擎里的次数改回 1，成功返回 true
    pub fn check(&mut self, frame: &Frame, now: Instant, restore: impl FnOnce() -> bool) {
        let owned = frame.ids.contains(&EXTRA_SIXTH_SENSE);
        let ready = frame.ids.contains(&SIXTH_SENSE);
        let used = frame.ids.contains(&SIXTH_SENSE_USED);
        let gained = owned && !self.owned;
        let engine_refill = owned && ready && !self.had_ready && !self.restored_at.take().is_some_and(|t| now < t + OWN_WINDOW);
        if (gained || engine_refill) && self.extra == 0 {
            self.extra = 1;
            info!("sixth sense: extra use ready");
        }
        self.owned = owned;
        self.had_ready = ready;
        if !(owned && used && !ready && self.extra > 0 && frame.hp.0 > 0) {
            self.restore_at = None;
            return;
        }
        if now < *self.restore_at.get_or_insert(now + RESTORE_DELAY) {
            return;
        }
        if !restore() {
            warn!("sixth sense: cannot write the use count");
            self.restore_at = Some(now + RETRY);
            return;
        }
        self.restore_at = None;
        self.extra -= 1;
        self.restored_at = Some(now);
        info!("sixth sense: restored, extra left {}", self.extra);
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(ids: &[i32]) -> Frame<'_> {
        Frame { ids, hp: (500, 1000), runes: 0 }
    }

    /// 跑一帧，返回这一帧有没有恢复次数
    fn step(s: &mut SixthSense, ids: &[i32], now: Instant) -> bool {
        let mut restored = false;
        s.check(&frame(ids), now, || {
            restored = true;
            true
        });
        restored
    }

    #[test]
    fn restores_once_after_delay_then_waits_for_engine_refill() {
        let mut s = SixthSense::default();
        let t0 = Instant::now();
        step(&mut s, &[EXTRA_SIXTH_SENSE, SIXTH_SENSE], t0);
        // 触发：等满延迟才恢复
        assert!(!step(&mut s, &[EXTRA_SIXTH_SENSE, SIXTH_SENSE_USED], t0));
        assert!(step(&mut s, &[EXTRA_SIXTH_SENSE, SIXTH_SENSE_USED], t0 + RESTORE_DELAY));
        // 自己恢复后出现的「可用」不算补充，再用掉就没有了
        step(&mut s, &[EXTRA_SIXTH_SENSE, SIXTH_SENSE], t0 + RESTORE_DELAY);
        step(&mut s, &[EXTRA_SIXTH_SENSE, SIXTH_SENSE_USED], t0 + RESTORE_DELAY * 2);
        assert!(!step(&mut s, &[EXTRA_SIXTH_SENSE, SIXTH_SENSE_USED], t0 + RESTORE_DELAY * 4));
        // 赐福：引擎挂回「可用」后额外次数补满
        step(&mut s, &[EXTRA_SIXTH_SENSE, SIXTH_SENSE], t0 + RESTORE_DELAY * 5);
        step(&mut s, &[EXTRA_SIXTH_SENSE, SIXTH_SENSE_USED], t0 + RESTORE_DELAY * 6);
        assert!(step(&mut s, &[EXTRA_SIXTH_SENSE, SIXTH_SENSE_USED], t0 + RESTORE_DELAY * 7));
    }

    #[test]
    fn gaining_buff_after_use_restores() {
        let mut s = SixthSense::default();
        let t0 = Instant::now();
        step(&mut s, &[SIXTH_SENSE_USED], t0);
        step(&mut s, &[EXTRA_SIXTH_SENSE, SIXTH_SENSE_USED], t0);
        assert!(step(&mut s, &[EXTRA_SIXTH_SENSE, SIXTH_SENSE_USED], t0 + RESTORE_DELAY));
    }

    #[test]
    fn failed_write_keeps_the_extra_use() {
        let mut s = SixthSense::default();
        let t0 = Instant::now();
        step(&mut s, &[EXTRA_SIXTH_SENSE, SIXTH_SENSE_USED], t0);
        s.check(&frame(&[EXTRA_SIXTH_SENSE, SIXTH_SENSE_USED]), t0 + RESTORE_DELAY, || false);
        assert!(step(&mut s, &[EXTRA_SIXTH_SENSE, SIXTH_SENSE_USED], t0 + RESTORE_DELAY * 2));
    }

    #[test]
    fn nothing_without_buff() {
        let mut s = SixthSense::default();
        let t0 = Instant::now();
        step(&mut s, &[SIXTH_SENSE], t0);
        step(&mut s, &[SIXTH_SENSE_USED], t0);
        assert!(!step(&mut s, &[SIXTH_SENSE_USED], t0 + RESTORE_DELAY * 2));
    }
}
