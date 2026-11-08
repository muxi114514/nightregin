//! 追踪者3 / 送葬者6：扣量表后返还一部分（行在 relics 的 tiered::tae）。原版扣量表事件（动作类型 1000）同时开一段类型 1002 的
//! 量表锁定，与「绝招中」（shared::tiered_guardian::ULT_ACTIVE）一起结束，锁定期间加量表无效；送葬者强化恍惚期间量表增长倍率也是 0。
//! 所以动作事件只挂「待返还」标记，DLL 等这些都结束再施加返还量表的效果并移除标记

use crate::tiered_guardian::ULT_ACTIVE;

pub const WYLDER_PENDING: i32 = 9_971_617;
pub const WYLDER_GAUGE: i32 = 9_971_616;
pub const UNDERTAKER_PENDING: i32 = 9_971_622;
pub const UNDERTAKER_GAUGE: i32 = 9_971_621;
/// 送葬者强化恍惚：持续期间 ultimateArtGaugeRate / ultimateArtGaugeChargeRate 都是 0
pub const ENHANCED_TRANCE: i32 = 709_010;

pub struct Refund {
    pub pending: i32,
    /// 返还量表的效果
    pub gauge: i32,
    /// 身上有其中任何一个时量表加不进去，要等它们都消失
    pub blockers: &'static [i32],
}

pub const REFUNDS: [Refund; 2] = [
    Refund { pending: WYLDER_PENDING, gauge: WYLDER_GAUGE, blockers: &[ULT_ACTIVE] },
    Refund { pending: UNDERTAKER_PENDING, gauge: UNDERTAKER_GAUGE, blockers: &[ULT_ACTIVE, ENHANCED_TRANCE] },
];
