//! 守护者6 / 7（shared::tiered_guardian）：绝招期间每秒给自己挂一次圣律加成并发给队友；
//! 冰风暴只保留与「有没有延长技艺」相符的那个状态效果，动作事件据此发普通或延长版冰风暴。

use std::time::{Duration, Instant};

use nr_stat_alloc_shared::tiered_guardian::{
    GUARDIAN_ICE, GUARDIAN_SACRED, ICE_LONG, ICE_SHORT, SACRED_BUFF, SACRED_SPAWN, SKILL_EXTENSIONS, ULT_ACTIVE,
};

use super::Frame;

/// 圣律加成 1.5 秒，每秒补一次，离开范围或绝招结束后很快消失
const SACRED_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Default)]
pub struct Guardian {
    next_sacred: Option<Instant>,
}

impl Guardian {
    pub fn check(&mut self, frame: &Frame, mut apply: impl FnMut(i32), mut remove: impl FnMut(i32)) {
        self.sacred(frame, &mut apply);
        let want = ice_state(frame.ids);
        for id in [ICE_SHORT, ICE_LONG] {
            let has = frame.ids.contains(&id);
            if want == Some(id) && !has {
                apply(id);
            } else if want != Some(id) && has {
                remove(id);
            }
        }
    }

    pub fn reset(&mut self) {
        self.next_sacred = None;
    }

    fn sacred(&mut self, frame: &Frame, apply: &mut impl FnMut(i32)) {
        if !(frame.ids.contains(&GUARDIAN_SACRED) && frame.ids.contains(&ULT_ACTIVE)) {
            self.next_sacred = None;
            return;
        }
        let now = Instant::now();
        if self.next_sacred.is_none_or(|t| now >= t) {
            apply(SACRED_BUFF);
            apply(SACRED_SPAWN);
            self.next_sacred = Some(now + SACRED_INTERVAL);
        }
    }
}

/// 冰风暴该挂的状态效果：有延长技艺的遗物或锻体时用延长版
fn ice_state(ids: &[i32]) -> Option<i32> {
    ids.contains(&GUARDIAN_ICE).then(|| if SKILL_EXTENSIONS.iter().any(|id| ids.contains(id)) { ICE_LONG } else { ICE_SHORT })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(ids: &[i32]) -> Frame<'_> {
        Frame { ids, hp: (1000, 1000), runes: 0 }
    }

    #[test]
    fn ice_follows_skill_extension() {
        assert_eq!(ice_state(&[GUARDIAN_ICE]), Some(ICE_SHORT));
        assert_eq!(ice_state(&[GUARDIAN_ICE, 7_011_000]), Some(ICE_LONG));
        assert_eq!(ice_state(&[7_011_000]), None);
        let mut g = Guardian::default();
        let (mut applied, mut removed) = (Vec::new(), Vec::new());
        g.check(&frame(&[GUARDIAN_ICE, ICE_SHORT, 9_971_320]), |id| applied.push(id), |id| removed.push(id));
        assert_eq!((applied, removed), (vec![ICE_LONG], vec![ICE_SHORT]));
    }

    #[test]
    fn sacred_pulses_only_during_ult() {
        let mut g = Guardian::default();
        let mut applied = Vec::new();
        g.check(&frame(&[GUARDIAN_SACRED]), |id| applied.push(id), |_| {});
        assert!(applied.is_empty());
        g.check(&frame(&[GUARDIAN_SACRED, ULT_ACTIVE]), |id| applied.push(id), |_| {});
        assert_eq!(applied, vec![SACRED_BUFF, SACRED_SPAWN]);
        applied.clear();
        // 间隔内不重复
        g.check(&frame(&[GUARDIAN_SACRED, ULT_ACTIVE]), |id| applied.push(id), |_| {});
        assert!(applied.is_empty());
    }
}
