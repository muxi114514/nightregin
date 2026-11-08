//! 三档锻体里靠 DLL 实现的执行者兽化词条（同 tiered_runes：regulation 里只放标记，DLL 动手）。
//! - 执行者4 BEAST_HP_LOCK：兽化变身（原版标识 BEAST_FORM）期间锁满血：不限时的不死 BEAST_UNDYING 在兽化期间挂着、结束移除，血量一掉就挂回满 BEAST_HEAL。
//!   之前的参数版 / 脚本版都以 707100（名为 Is Beast Form）判断兽化，诊断日志显示兽化期间根本没有它，所以从未触发

pub const BEAST_HP_LOCK: i32 = 9_971_350;
pub const BEAST_UNDYING: i32 = 9_971_351;
pub const BEAST_HEAL: i32 = 9_971_352;
/// 兽化期间实际在身上的「Beast Health Boost」（707115，原版动作脚本 IsDemonBeastMode 也认它）
pub const BEAST_FORM: i32 = 707_115;
/// 执行者8 锻体把绝招换成的熔炉形态（tools\tiered_crucible.ps1），执行者4 同样锁血
pub const CRUCIBLE_FORM: i32 = 9_971_870;
/// 熔炉形态踏地中（按绝招到踏地结束）：招式表不看拔没拔刀，直接用熔炉妖刀的（src\crucible_sword.rs）
pub const CRUCIBLE_ACTIVE: i32 = 9_971_879;
