//! 午夜模式的纯规则（不碰游戏内存）：等级曲线、积分、结算状态机、诅咒表、联机状态编码。

pub mod curse;
mod level;
mod score;
mod settle;
pub mod wire;

pub use level::{Curve, LevelProgress, MAX_LEVEL, barren_unlocked};
pub use score::{Outcome, Scoring, apply};
pub use settle::{Place, SettleEvent, Settler, boss_flag_kind, resume};
