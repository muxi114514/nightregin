//! 追踪者3 / 送葬者6（shared::tiered_refund）：身上有「待返还」标记、且挡住加量表的效果（量表锁定随「绝招中」结束、
//! 送葬者还要等强化恍惚结束）都不在时，施加返还量表并移除标记。每个标记只还一次，等它从身上消失才会再认。

use log::info;
use nr_stat_alloc_shared::tiered_refund::REFUNDS;

use super::Frame;

#[derive(Default)]
pub struct GaugeRefund {
    /// 已还过、还没从身上消失的标记
    done: Vec<i32>,
}

impl GaugeRefund {
    pub fn check(&mut self, frame: &Frame, mut apply: impl FnMut(i32), mut remove: impl FnMut(i32)) {
        self.done.retain(|p| frame.ids.contains(p));
        for r in &REFUNDS {
            let ready = frame.ids.contains(&r.pending) && !self.done.contains(&r.pending) && !r.blockers.iter().any(|b| frame.ids.contains(b));
            if ready {
                apply(r.gauge);
                remove(r.pending);
                self.done.push(r.pending);
                info!("gauge refund {}", r.gauge);
            }
        }
    }

    pub fn reset(&mut self) {
        self.done.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nr_stat_alloc_shared::tiered_guardian::ULT_ACTIVE;
    use nr_stat_alloc_shared::tiered_refund::{ENHANCED_TRANCE, UNDERTAKER_GAUGE, UNDERTAKER_PENDING, WYLDER_GAUGE, WYLDER_PENDING};

    fn step(r: &mut GaugeRefund, ids: &[i32]) -> (Vec<i32>, Vec<i32>) {
        let (mut applied, mut removed) = (Vec::new(), Vec::new());
        r.check(&Frame { ids, hp: (500, 1000), runes: 0 }, |id| applied.push(id), |id| removed.push(id));
        (applied, removed)
    }

    #[test]
    fn waits_for_ult_to_end() {
        let mut r = GaugeRefund::default();
        assert!(step(&mut r, &[WYLDER_PENDING, ULT_ACTIVE]).0.is_empty());
        assert_eq!(step(&mut r, &[WYLDER_PENDING]), (vec![WYLDER_GAUGE], vec![WYLDER_PENDING]));
    }

    #[test]
    fn undertaker_also_waits_for_enhanced_trance() {
        let mut r = GaugeRefund::default();
        assert!(step(&mut r, &[UNDERTAKER_PENDING, ENHANCED_TRANCE]).0.is_empty());
        // 强化恍惚只挡送葬者
        assert_eq!(step(&mut r, &[UNDERTAKER_PENDING, WYLDER_PENDING, ENHANCED_TRANCE]).0, vec![WYLDER_GAUGE]);
        assert_eq!(step(&mut r, &[UNDERTAKER_PENDING]).0, vec![UNDERTAKER_GAUGE]);
    }

    #[test]
    fn refunds_once_until_marker_leaves() {
        let mut r = GaugeRefund::default();
        step(&mut r, &[WYLDER_PENDING]);
        // 移除还没生效的那一帧不重复还
        assert!(step(&mut r, &[WYLDER_PENDING]).0.is_empty());
        step(&mut r, &[]);
        assert_eq!(step(&mut r, &[WYLDER_PENDING]).0, vec![WYLDER_GAUGE]);
    }
}
