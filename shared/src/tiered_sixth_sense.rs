//! 追踪者12：第六感多 1 次（同 tiered_runes：regulation 里只放标记，DLL 动手）。
//! 原版第六感「可用」SIXTH_SENSE 与「已用」SIXTH_SENSE_USED（PlayerCommonParam unknown_27 / unknown_28）由引擎按第六感组件里的次数每帧同步：
//! 次数 > 0 挂「可用」、为 0 挂「已用」，触发时次数清零，出击开始与赐福休息时补回 1。DLL 在「已用」出现后把次数改回 1 一次，
//! 引擎自己挂回「可用」时额外次数补满（单独挂效果没用，见 src\game\sixth_sense.rs）

pub const EXTRA_SIXTH_SENSE: i32 = 9_971_850;
pub const SIXTH_SENSE: i32 = 700_600;
pub const SIXTH_SENSE_USED: i32 = 700_601;
