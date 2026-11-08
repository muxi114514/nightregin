//! 单个角色的原版成长表：HeroStatusParam 关键帧，中间等级 = 线性插值后向下取整（已用 wiki 逐级表验证）。

use super::STAT_COUNT;

pub const MAX_LEVEL: u32 = 15;
/// 参与升级加点的 7 项（不含感应）
pub const GROWTH_STATS: usize = 7;
pub const ARCANE: usize = 7;

pub type Stats = [u32; STAT_COUNT];

#[derive(Debug, Clone)]
pub struct HeroGrowth {
    levels: Vec<u32>,
    frames: Vec<Stats>,
}

impl HeroGrowth {
    /// 关键帧等级必须严格递增、从 1 级开始并覆盖到 15 级
    pub fn new(levels: Vec<u32>, frames: Vec<Stats>) -> Result<Self, String> {
        if levels.len() != frames.len() || levels.len() < 2 {
            return Err("keyframe count mismatch".into());
        }
        if levels[0] != 1 || *levels.last().unwrap() < MAX_LEVEL || levels.windows(2).any(|w| w[0] >= w[1]) {
            return Err(format!("invalid keyframe levels {levels:?}"));
        }
        Ok(Self { levels, frames })
    }

    /// level 超出 1..=15 时按边界取值
    pub fn stats_at(&self, level: u32) -> Stats {
        let level = level.clamp(1, MAX_LEVEL);
        let hi = self.levels.iter().position(|&l| l >= level).unwrap_or(self.levels.len() - 1);
        if self.levels[hi] == level || hi == 0 {
            return self.frames[hi];
        }
        let lo = hi - 1;
        let (l0, l1) = (self.levels[lo] as i64, self.levels[hi] as i64);
        let mut out = [0; STAT_COUNT];
        for (i, o) in out.iter_mut().enumerate() {
            let (a, b) = (self.frames[lo][i] as i64, self.frames[hi][i] as i64);
            // floor(a + (b - a) * t)，整数运算避免浮点误差
            *o = (a + ((b - a) * (level as i64 - l0)).div_euclid(l1 - l0)).max(0) as u32;
        }
        out
    }

    /// 某级 7 项（不含感应）的新增点数：sum(L) - sum(L-1)
    pub fn gain_at(&self, level: u32) -> u32 {
        let sum = |s: Stats| s[..GROWTH_STATS].iter().sum::<u32>();
        sum(self.stats_at(level)).saturating_sub(sum(self.stats_at(level - 1)))
    }
}
