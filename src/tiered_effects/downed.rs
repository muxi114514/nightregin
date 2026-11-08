//! 倒地类锻体：蓝6 血量从正数掉到 0 的那一帧引发火焰爆发（一次倒地只触发一次，被救起后重新计）。

use nr_stat_alloc_shared::tiered_downed::DOWNED_BURST;

use super::Frame;

#[derive(Default)]
pub struct DownedBurst {
    alive: bool,
}

impl DownedBurst {
    /// 本帧是否该爆发
    pub fn check(&mut self, frame: &Frame) -> bool {
        let alive = frame.hp.0 > 0;
        let fire = self.alive && !alive && frame.ids.contains(&DOWNED_BURST);
        self.alive = alive;
        fire
    }

    pub fn reset(&mut self) {
        self.alive = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(ids: &[i32], hp: u32) -> Frame<'_> {
        Frame { ids, hp: (hp, 1000), runes: 0 }
    }

    #[test]
    fn bursts_once_per_down() {
        let mut d = DownedBurst::default();
        let ids = [DOWNED_BURST];
        assert!(!d.check(&frame(&ids, 0)));
        assert!(!d.check(&frame(&ids, 500)));
        assert!(d.check(&frame(&ids, 0)));
        assert!(!d.check(&frame(&ids, 0)));
        assert!(!d.check(&frame(&ids, 300)));
        assert!(!d.check(&frame(&[], 0)));
    }
}
