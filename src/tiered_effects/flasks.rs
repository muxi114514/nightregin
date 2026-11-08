//! 圣杯瓶类锻体的判断：只根据本帧数值与自己的记录给出要做的动作，读写内存由 mod.rs 做，便于单测。

use nr_stat_alloc_shared::tiered_flasks::{FLASK_PLUS, FLASK_REFILL, MAX_FLASKS, REVIVED};

use super::Frame;

/// 金2：被救起（102190）出现的那一刻要补满
#[derive(Default)]
pub struct Refill {
    revived: bool,
}

impl Refill {
    pub fn check(&mut self, frame: &Frame) -> bool {
        let revived = frame.ids.contains(&REVIVED);
        let fire = revived && !self.revived && frame.ids.contains(&FLASK_REFILL);
        self.revived = revived;
        fire
    }

    pub fn reset(&mut self) {
        self.revived = false;
    }
}

/// 蓝3：可叠加，份数比本局见过的最多还多时上限 +1；游戏之后把上限改回更小时，在它上面重新加回本局累计的加成
#[derive(Default)]
pub struct Plus {
    seen: Option<usize>,
    bonus: u8,
    written: Option<u8>,
}

impl Plus {
    /// 返回要写的新上限
    pub fn check(&mut self, frame: &Frame, max: u8) -> Option<u8> {
        let count = frame.ids.iter().filter(|&&id| id == FLASK_PLUS).count();
        let Some(seen) = self.seen else {
            self.seen = Some(count);
            return None;
        };
        if count > seen {
            self.seen = Some(count);
            let add = u8::try_from(count - seen).unwrap_or(u8::MAX);
            self.bonus = self.bonus.saturating_add(add);
            return self.write(max.saturating_add(add).min(MAX_FLASKS));
        }
        match self.written {
            Some(w) if max < w && self.bonus > 0 => self.write(max.saturating_add(self.bonus).min(MAX_FLASKS)),
            Some(w) if max > w => {
                self.written = Some(max);
                None
            }
            _ => None,
        }
    }

    fn write(&mut self, value: u8) -> Option<u8> {
        self.written = Some(value);
        Some(value)
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// 紫5：已装且次数达到上限
pub fn full(filled: bool, quantity: u32, max: u8) -> bool {
    filled && max > 0 && quantity >= max as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(ids: &[i32]) -> Frame<'_> {
        Frame { ids, hp: (500, 500), runes: 0 }
    }

    #[test]
    fn refill_fires_once_per_revive() {
        let mut r = Refill::default();
        assert!(!r.check(&frame(&[FLASK_REFILL])));
        assert!(r.check(&frame(&[FLASK_REFILL, REVIVED])));
        assert!(!r.check(&frame(&[FLASK_REFILL, REVIVED])));
        assert!(!r.check(&frame(&[FLASK_REFILL])));
        assert!(!r.check(&frame(&[REVIVED])));
    }

    #[test]
    fn plus_raises_cap_and_restores_after_reset() {
        let mut p = Plus::default();
        assert_eq!(p.check(&frame(&[]), 3), None);
        assert_eq!(p.check(&frame(&[FLASK_PLUS]), 3), Some(4));
        assert_eq!(p.check(&frame(&[FLASK_PLUS]), 4), None);
        // 游戏把上限改回 3：重新加回
        assert_eq!(p.check(&frame(&[FLASK_PLUS]), 3), Some(4));
        // 封顶
        assert_eq!(p.check(&frame(&[FLASK_PLUS, FLASK_PLUS]), 20), Some(20));
    }

    #[test]
    fn full_needs_filled_and_cap() {
        assert!(full(true, 3, 3));
        assert!(!full(true, 2, 3));
        assert!(!full(false, 1, 1));
    }
}
