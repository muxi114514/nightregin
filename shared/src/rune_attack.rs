//! 锻体「持有卢恩越多，攻击力越高」：regulation 里的新锻体效果（mmv_relics 写入）只带状态值 STATE，
//! 增伤由 DLL 钩住利普拉「恶魔的添翼」的倍率计算实现（exe 写死了状态值 2118 与曲线 60，参数表做不出另一条曲线）。
//! 两边共用这里的常量与合成规则。

/// 新锻体效果的 stateInfo：原版 / MMV / 空城的参数表与 exe 的状态检查都没用过
pub const STATE: u16 = 2400;
/// 利普拉按持有卢恩换算攻击倍率的曲线（CalcCorrectGraph）
pub const CURVE_ID: u32 = 60;
/// 新锻体的加成占利普拉的比例
pub const SHARE: f32 = 0.5;

/// 合成攻击倍率。libra_multiplier：曲线按持有卢恩算出的利普拉倍率；同时持有时两份加成相加
pub fn blend(libra_multiplier: f32, libra: bool, ours: bool) -> f32 {
    let share = if libra { 1.0 } else { 0.0 } + if ours { SHARE } else { 0.0 };
    1.0 + (libra_multiplier - 1.0) * share
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-5
    }

    #[test]
    fn half_of_libra_and_additive_stacking() {
        // 10 万卢恩时利普拉 ×1.3
        assert!(close(blend(1.3, true, false), 1.3));
        assert!(close(blend(1.3, false, true), 1.15));
        assert!(close(blend(1.3, true, true), 1.45));
        assert!(close(blend(1.3, false, false), 1.0));
    }
}
