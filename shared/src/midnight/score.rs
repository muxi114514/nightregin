//! 午夜积分：胜负加减分，积分最低到 0；胜利另给货币。

const WIN_CURRENCY: u64 = 10;
const EVERDARK_CURRENCY: u64 = 15;

/// 一局午夜的结果；Void = 闪退 / 掉线作废
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Win { everdark: bool },
    Loss,
    Void,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Scoring {
    pub win: u32,
    pub everdark_multiplier: f32,
    pub loss: u32,
}

impl Default for Scoring {
    fn default() -> Self {
        Self { win: 100, everdark_multiplier: 1.5, loss: 50 }
    }
}

impl Scoring {
    pub fn delta(&self, outcome: Outcome) -> i64 {
        match outcome {
            Outcome::Win { everdark: false } => self.win as i64,
            Outcome::Win { everdark: true } => (self.win as f64 * self.everdark_multiplier as f64).round() as i64,
            Outcome::Loss => -(self.loss as i64),
            Outcome::Void => 0,
        }
    }

    /// 这局给的货币（只累计，数值等天赋树时再调）
    pub fn currency(&self, outcome: Outcome) -> u64 {
        match outcome {
            Outcome::Win { everdark: false } => WIN_CURRENCY,
            Outcome::Win { everdark: true } => EVERDARK_CURRENCY,
            Outcome::Loss | Outcome::Void => 0,
        }
    }
}

pub fn apply(points: u64, delta: i64) -> u64 {
    points.saturating_add_signed(delta)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::midnight::Curve;

    #[test]
    fn deltas() {
        let s = Scoring::default();
        assert_eq!(s.delta(Outcome::Win { everdark: false }), 100);
        assert_eq!(s.delta(Outcome::Win { everdark: true }), 150);
        assert_eq!(s.delta(Outcome::Loss), -50);
        assert_eq!(s.delta(Outcome::Void), 0);
    }

    #[test]
    fn currency_by_outcome() {
        let s = Scoring::default();
        assert_eq!(s.currency(Outcome::Win { everdark: false }), 10);
        assert_eq!(s.currency(Outcome::Win { everdark: true }), 15);
        assert_eq!(s.currency(Outcome::Loss), 0);
        assert_eq!(s.currency(Outcome::Void), 0);
    }

    #[test]
    fn points_never_negative() {
        assert_eq!(apply(30, -50), 0);
        assert_eq!(apply(30, 100), 130);
    }

    #[test]
    fn loss_can_drop_level() {
        assert_eq!(Curve::default().level_of(apply(730, -50)), 5);
    }
}
