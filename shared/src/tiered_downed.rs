//! 三档锻体里靠 DLL 实现的倒地类词条（同 tiered_runes：regulation 里只放标记，DLL 动手）。
//! - 蓝6 DOWNED_BURST：血量从正数掉到 0（倒地 / 死亡）的那一刻给自己挂 DOWNED_BURST_SPAWN，
//!   由它发出火焰爆炸子弹（照原版 3 级振奋香的追加爆炸：效果状态 275 + 行为 → 子弹，攻击行固定火属性伤害）

pub const DOWNED_BURST: i32 = 9_971_665;
pub const DOWNED_BURST_SPAWN: i32 = 9_971_666;
