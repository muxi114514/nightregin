//! 午夜等级曲线：前几级每级固定积分，之后每级比上一级多一个步长；等级由累计积分现算。

pub const MAX_LEVEL: u32 = 100;
/// 静默之域的最高等级，高于它解锁荒芜之域
const SILENT_MAX_LEVEL: u32 = 5;

pub fn barren_unlocked(level: u32) -> bool {
    level > SILENT_MAX_LEVEL
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Curve {
    pub early_levels: u32,
    pub early_cost: u32,
    pub cost_step: u32,
}

impl Default for Curve {
    fn default() -> Self {
        Self { early_levels: 5, early_cost: 120, cost_step: 10 }
    }
}

/// 当前等级、本级已得积分、升到下一级还需本级总积分（满级为 0）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LevelProgress {
    pub level: u32,
    pub into_level: u64,
    pub needed: u64,
}

impl Curve {
    /// 第 level 级本身要的积分
    fn cost_of(&self, level: u32) -> u64 {
        let extra = level.saturating_sub(self.early_levels) as u64;
        self.early_cost as u64 + self.cost_step as u64 * extra
    }

    /// 到达 level 级的累计积分
    pub fn total_for(&self, level: u32) -> u64 {
        let l = level.min(MAX_LEVEL) as u64;
        let e = self.early_levels as u64;
        let over = l.saturating_sub(e);
        self.early_cost as u64 * l + self.cost_step as u64 * over * (over + 1) / 2
    }

    pub fn level_of(&self, points: u64) -> u32 {
        (1..=MAX_LEVEL).take_while(|&l| self.total_for(l) <= points).last().unwrap_or(0)
    }

    pub fn progress(&self, points: u64) -> LevelProgress {
        let level = self.level_of(points);
        if level >= MAX_LEVEL {
            return LevelProgress { level, into_level: 0, needed: 0 };
        }
        LevelProgress { level, into_level: points - self.total_for(level), needed: self.cost_of(level + 1) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn totals_match_spec() {
        let c = Curve::default();
        let got: Vec<u64> = [1, 5, 6, 10, 50, 100].iter().map(|&l| c.total_for(l)).collect();
        assert_eq!(got, vec![120, 600, 730, 1350, 16350, 57600]);
    }

    #[test]
    fn level_boundaries() {
        let c = Curve::default();
        assert_eq!(c.level_of(0), 0);
        assert_eq!(c.level_of(599), 4);
        assert_eq!(c.level_of(600), 5);
        assert_eq!(c.level_of(729), 5);
        assert_eq!(c.level_of(730), 6);
        assert_eq!(c.level_of(57599), 99);
        assert_eq!(c.level_of(57600), 100);
        assert_eq!(c.level_of(u64::MAX), 100);
    }

    #[test]
    fn progress_at_max_needs_nothing() {
        let p = Curve::default().progress(57600);
        assert_eq!((p.level, p.needed), (100, 0));
    }

    #[test]
    fn progress_inside_level() {
        // 第 6 级要 130 分：730 起算，已拿 20
        assert_eq!(Curve::default().progress(750), LevelProgress { level: 6, into_level: 20, needed: 140 });
    }

    #[test]
    fn barren_gate() {
        assert!(!barren_unlocked(5));
        assert!(barren_unlocked(6));
    }
}
