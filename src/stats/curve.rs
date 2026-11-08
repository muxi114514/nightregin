//! CalcCorrectGraph 分段曲线（与游戏同一套参数，运行时从参数表读取），用于预览血量 / 专注值 / 精力。
//! 遗物等装备的影响按实时数据推算：属性加成 = 实际属性 - 方案目标；三维倍率 = 实际上限 / 曲线(实际属性)。

use super::STAT_COUNT;
use super::growth::Stats;

/// 三维对应的属性：生命力 -> 血量，集中力 -> 专注值，耐力 -> 精力
pub const VITAL_STATS: [usize; 3] = [0, 1, 2];

#[derive(Debug, Clone, PartialEq)]
pub struct Curve {
    stage_max_val: [f32; 5],
    stage_max_grow_val: [f32; 5],
    adj_pt: [f32; 5],
}

impl Curve {
    pub fn new(stage_max_val: [f32; 5], stage_max_grow_val: [f32; 5], adj_pt: [f32; 5]) -> Result<Self, String> {
        if stage_max_val.windows(2).any(|w| w[0] > w[1]) {
            return Err(format!("curve stages not increasing: {stage_max_val:?}"));
        }
        Ok(Self { stage_max_val, stage_max_grow_val, adj_pt })
    }

    /// 游戏的分段插值：段内比例按 adj_pt 做指数修正（正数为 r^a，负数为 1-(1-r)^-a）
    pub fn eval(&self, x: f32) -> f32 {
        let (s, g) = (&self.stage_max_val, &self.stage_max_grow_val);
        if x <= s[0] {
            return g[0];
        }
        for i in 0..4 {
            if x <= s[i + 1] {
                let span = s[i + 1] - s[i];
                let r = if span > 0.0 { (x - s[i]) / span } else { 1.0 };
                let a = self.adj_pt[i];
                let t = if a >= 0.0 { r.powf(a) } else { 1.0 - (1.0 - r).powf(-a) };
                return g[i] + (g[i + 1] - g[i]) * t;
            }
        }
        g[4]
    }
}

/// 血量 / 专注值 / 精力三条曲线
#[derive(Debug, Clone, PartialEq)]
pub struct VitalCurves(pub [Curve; 3]);

/// 当前装备对预览的修正
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VitalAdjust {
    bonus: [i32; STAT_COUNT],
    ratio: [f32; 3],
}

impl Default for VitalAdjust {
    fn default() -> Self {
        Self { bonus: [0; STAT_COUNT], ratio: [1.0; 3] }
    }
}

impl VitalCurves {
    /// effective：角色当前实际属性；target：当前生效方案在当前等级的目标属性；max：当前三维上限
    pub fn adjust(&self, effective: &Stats, target: &Stats, max: [u32; 3]) -> VitalAdjust {
        let bonus = std::array::from_fn(|i| effective[i] as i32 - target[i] as i32);
        let ratio = std::array::from_fn(|v| {
            let base = self.0[v].eval(effective[VITAL_STATS[v]] as f32);
            if base > 0.0 && max[v] > 0 { max[v] as f32 / base } else { 1.0 }
        });
        VitalAdjust { bonus, ratio }
    }

    /// 某组方案属性下的三维（向下取整，与游戏显示一致）
    pub fn vitals(&self, stats: &Stats, adjust: &VitalAdjust) -> [u32; 3] {
        std::array::from_fn(|v| {
            let stat = (stats[VITAL_STATS[v]] as i32 + adjust.bonus[VITAL_STATS[v]]).max(1);
            // 加一点余量再取整，避免 594.0 因浮点误差变成 593.99 被截成 593
            (self.0[v].eval(stat as f32) * adjust.ratio[v] + 1e-3).floor().max(0.0) as u32
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hp() -> Curve {
        Curve::new([1.0, 25.0, 50.0, 75.0, 99.0], [100.0, 580.0, 1080.0, 1580.0, 2060.0], [1.0; 5]).unwrap()
    }

    #[test]
    fn hp_curve_matches_in_game_values() {
        // 实测：生命力 7 -> 220，13 -> 340，57 -> 1220
        for (vigor, expected) in [(7.0, 220.0), (13.0, 340.0), (57.0, 1220.0)] {
            assert!((hp().eval(vigor) - expected).abs() < 1e-3, "{vigor}");
        }
        assert_eq!(hp().eval(0.0), 100.0);
        assert_eq!(hp().eval(150.0), 2060.0);
    }

    #[test]
    fn exponent_shapes_segment() {
        let c = Curve::new([0.0, 10.0, 10.0, 10.0, 10.0], [0.0, 100.0, 100.0, 100.0, 100.0], [2.0, 1.0, 1.0, 1.0, 1.0]).unwrap();
        assert_eq!(c.eval(5.0), 25.0);
        let c = Curve::new([0.0, 10.0, 10.0, 10.0, 10.0], [0.0, 100.0, 100.0, 100.0, 100.0], [-2.0, 1.0, 1.0, 1.0, 1.0]).unwrap();
        assert_eq!(c.eval(5.0), 75.0);
    }

    #[test]
    fn adjust_carries_relic_bonus_and_ratio() {
        let curves = VitalCurves([hp(), hp(), hp()]);
        // 遗物 +3 生命力、血量 +10%：实际 16 点生命力，上限 440
        let effective = [16, 2, 3, 5, 8, 2, 2, 28];
        let target = [13, 2, 3, 5, 8, 2, 2, 28];
        let adj = curves.adjust(&effective, &target, [440, 120, 140]);
        let v = curves.vitals(&[20, 2, 3, 5, 8, 2, 2, 28], &adj);
        // 方案 20 + 遗物 3 = 23 点 -> 540，再乘 1.1
        assert_eq!(v[0], 594);
    }
}
