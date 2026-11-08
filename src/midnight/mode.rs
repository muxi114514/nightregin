//! 午夜开关规则（纯函数）：房主未准备时跟本地开关、准备后锁定；队友跟房主广播；诅咒的目标与揭晓；拦准备；侧栏状态。

use nr_stat_alloc_shared::midnight::curse::{CurseId, NO_CURSE};
use nr_stat_alloc_shared::midnight::wire::MemberState;

/// 本帧午夜是否打开：房主未准备时跟本地开关，准备后锁定为上一帧；队友只跟房主广播，收不到就关
pub fn next_active(local_toggle: bool, is_host: bool, host_ready: bool, prev: bool, remote: Option<bool>) -> bool {
    if is_host {
        if host_ready { prev } else { local_toggle }
    } else {
        remote.unwrap_or(false)
    }
}

/// 午夜是否真正生效：房主要选着深夜栏（选普通夜王时开关不起作用）；队友不选目标，跟房主广播
pub fn effective(active: bool, is_host: bool, deep_selected: bool) -> bool {
    active && (deep_selected || !is_host)
}

/// 发给队友的午夜状态：圆桌里跟实际生效状态，出征中保持（房主先读完图时面板已复位，不能让还在读图的队友被关掉）
pub fn published_mode(in_hub: bool, effective: bool, prev: bool) -> bool {
    if in_hub { effective } else { prev }
}

/// 这次出征是不是真的午夜出击（圆桌里午夜生效且按了准备或单人出击，出征中保持）。
/// 开着午夜去训练场这类不走出击列表的地方，不算午夜局
pub fn armed(in_hub: bool, prev: bool, effective: bool, departing: bool) -> bool {
    if in_hub { effective && departing } else { prev }
}

/// 本局是否计午夜（带遗物出击的人不计）
pub fn counted(active: bool, has_relics: bool) -> bool {
    active && !has_relics
}

/// 圆桌里该写入的诅咒：午夜生效时房主用自己抽的，队友用房主的，否则没有
pub fn target_curse(effective: bool, is_host: bool, own: CurseId, host: Option<MemberState>) -> CurseId {
    match (effective, is_host) {
        (false, _) => NO_CURSE,
        (true, true) => own,
        (true, false) => host.map_or(NO_CURSE, |s| s.curse),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockReason {
    Relics,
    TeammateUnmodded,
}

/// 拦下本机「午夜准备」的原因：带遗物的人都拦；诅咒要全队数据一致，所以有队友没装模组时拦房主
pub fn block_reason(effective: bool, is_host: bool, own_relics: bool, teammates: &[Option<MemberState>]) -> Option<BlockReason> {
    if !effective {
        return None;
    }
    if own_relics {
        return Some(BlockReason::Relics);
    }
    (is_host && teammates.iter().any(Option::is_none)).then_some(BlockReason::TeammateUnmodded)
}

/// 选角界面是否揭晓诅咒
pub fn reveal(active: bool, curse: CurseId, hero_select: bool) -> bool {
    active && curse != NO_CURSE && hero_select
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RosterStatus {
    Ready,
    NotReady,
    Relics,
    /// 带遗物但拦不住准备（钩子不可用），本局按普通深夜算
    Uncounted,
    /// 一直收不到状态，或还没进午夜队伍
    Unmodded,
}

pub fn roster_status(state: Option<MemberState>, blocking_supported: bool) -> RosterStatus {
    match state {
        None => RosterStatus::Unmodded,
        Some(s) if !s.in_midnight => RosterStatus::Unmodded,
        Some(s) if s.has_relics => {
            if blocking_supported { RosterStatus::Relics } else { RosterStatus::Uncounted }
        }
        Some(s) if s.ready => RosterStatus::Ready,
        Some(_) => RosterStatus::NotReady,
    }
}

#[cfg(test)]
mod tests {
    use nr_stat_alloc_shared::midnight::wire::MemberState;

    use super::*;

    #[test]
    fn host_follows_local_toggle() {
        assert!(next_active(true, true, false, false, None));
        assert!(!next_active(false, true, false, true, None));
    }

    #[test]
    fn host_locked_after_ready() {
        assert!(next_active(false, true, true, true, None));
        assert!(!next_active(true, true, true, false, None));
    }

    #[test]
    fn guest_follows_remote() {
        assert!(next_active(false, false, false, false, Some(true)));
        assert!(!next_active(true, false, false, true, Some(false)));
    }

    #[test]
    fn guest_without_remote_is_off() {
        assert!(!next_active(true, false, false, true, None));
    }

    #[test]
    fn host_midnight_needs_deep_entry() {
        assert!(effective(true, true, true));
        assert!(!effective(true, true, false));
        assert!(!effective(false, true, true));
        // 队友不选目标，跟房主广播
        assert!(effective(true, false, false));
    }

    #[test]
    fn published_mode_follows_effective_in_hub() {
        assert!(published_mode(true, true, false));
        assert!(!published_mode(true, false, true));
    }

    /// 房主先读完图进了出征地图时，还在读图的队友不能被关掉午夜
    #[test]
    fn published_mode_is_held_during_expedition() {
        assert!(published_mode(false, false, true));
        assert!(!published_mode(false, true, false));
    }

    #[test]
    fn armed_needs_effective_and_departure() {
        assert!(armed(true, false, true, true));
        assert!(!armed(true, true, true, false));
        assert!(!armed(true, true, false, true));
    }

    #[test]
    fn armed_is_held_outside_the_hub() {
        assert!(armed(false, true, false, false));
        assert!(!armed(false, false, true, true));
    }

    #[test]
    fn relics_make_uncounted() {
        assert!(!counted(true, true));
        assert!(counted(true, false));
        assert!(!counted(false, false));
    }

    fn host(curse: u8) -> Option<MemberState> {
        Some(MemberState { in_midnight: true, curse, ..Default::default() })
    }

    fn mate(in_midnight: bool) -> Option<MemberState> {
        Some(MemberState { in_midnight, ..Default::default() })
    }

    #[test]
    fn target_is_none_when_not_effective() {
        assert_eq!(target_curse(false, true, 5, None), NO_CURSE);
        assert_eq!(target_curse(false, false, 5, host(9)), NO_CURSE);
    }

    #[test]
    fn host_targets_own_curse() {
        assert_eq!(target_curse(true, true, 5, host(9)), 5);
    }

    #[test]
    fn guest_targets_host_curse() {
        assert_eq!(target_curse(true, false, 5, host(9)), 9);
    }

    #[test]
    fn guest_without_host_state_has_none() {
        assert_eq!(target_curse(true, false, 5, None), NO_CURSE);
    }

    #[test]
    fn relics_block_anyone() {
        assert_eq!(block_reason(true, true, true, &[]), Some(BlockReason::Relics));
        assert_eq!(block_reason(true, false, true, &[]), Some(BlockReason::Relics));
    }

    #[test]
    fn relics_reported_before_unmodded() {
        assert_eq!(block_reason(true, true, true, &[None]), Some(BlockReason::Relics));
    }

    #[test]
    fn unmodded_teammate_blocks_host() {
        assert_eq!(block_reason(true, true, false, &[mate(true), None]), Some(BlockReason::TeammateUnmodded));
    }

    #[test]
    fn unmodded_teammate_does_not_block_guest() {
        assert_eq!(block_reason(true, false, false, &[None]), None);
    }

    /// 装了模组但还没收到午夜开关（in_midnight 为假）不算没装
    #[test]
    fn teammate_with_state_never_blocks() {
        assert_eq!(block_reason(true, true, false, &[mate(false), mate(true)]), None);
    }

    #[test]
    fn no_block_when_not_effective() {
        assert_eq!(block_reason(false, true, true, &[None]), None);
    }

    #[test]
    fn reveal_needs_active_curse_and_screen() {
        assert!(reveal(true, 5, true));
        assert!(!reveal(false, 5, true));
        assert!(!reveal(true, NO_CURSE, true));
        assert!(!reveal(true, 5, false));
    }

    #[test]
    fn roster_statuses() {
        assert_eq!(roster_status(None, true), RosterStatus::Unmodded);
        let s = |in_midnight, has_relics, ready| Some(MemberState { in_midnight, has_relics, ready, level: 3, curse: 0 });
        assert_eq!(roster_status(s(true, false, true), true), RosterStatus::Ready);
        assert_eq!(roster_status(s(true, false, false), true), RosterStatus::NotReady);
        assert_eq!(roster_status(s(true, true, false), true), RosterStatus::Relics);
        assert_eq!(roster_status(s(true, true, true), false), RosterStatus::Uncounted);
        assert_eq!(roster_status(s(false, false, false), true), RosterStatus::Unmodded);
    }
}
