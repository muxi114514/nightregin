//! 午夜的游戏线程 ↔ 叠加层共享状态：游戏线程每帧发布快照，界面发来切换请求与打开主界面请求。
//! 只用原子量与短暂持有的锁；界面线程从不调用游戏函数。

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use nr_stat_alloc_shared::midnight::LevelProgress;
use nr_stat_alloc_shared::midnight::curse::CurseId;

use super::mode::RosterStatus;

#[derive(Debug, Clone, PartialEq)]
pub struct RosterRow {
    pub slot: u8,
    /// Steam 昵称（读不到为 None）
    pub name: Option<String>,
    pub level: u32,
    pub status: RosterStatus,
}

/// 给界面的一次性提示（文字在 ui::midnight::text，保证字形在字体图集里）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Notice {
    RelicsBlocked,
    TeammateUnmodded,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct MidnightSnapshot {
    pub active: bool,
    pub is_host: bool,
    pub can_toggle: bool,
    /// 出击菜单开着且右侧是深夜面板
    pub deep_panel: bool,
    pub in_expedition: bool,
    /// 自己带着遗物
    pub relics: bool,
    /// 联机身份已知（单人或已确定房主 / 队友）
    pub session_known: bool,
    pub level: LevelProgress,
    pub points: u64,
    pub wins: u32,
    pub losses: u32,
    pub roster: Vec<RosterRow>,
    /// 这局的诅咒（0 = 无）
    pub curse: CurseId,
    /// 选角界面正在揭晓诅咒
    pub reveal: bool,
}

#[derive(Default)]
pub struct MidnightBridge {
    snapshot: Mutex<MidnightSnapshot>,
    toggle: AtomicBool,
    notice: Mutex<Option<Notice>>,
}

impl MidnightBridge {
    pub fn publish(&self, s: MidnightSnapshot) {
        *self.snapshot.lock().unwrap_or_else(|e| e.into_inner()) = s;
    }

    pub fn snapshot(&self) -> MidnightSnapshot {
        self.snapshot.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    pub fn request_toggle(&self) {
        self.toggle.store(true, Ordering::Release);
    }

    pub fn take_toggle(&self) -> bool {
        self.toggle.swap(false, Ordering::AcqRel)
    }

    pub fn notify(&self, notice: Notice) {
        *self.notice.lock().unwrap_or_else(|e| e.into_inner()) = Some(notice);
    }

    pub fn take_notice(&self) -> Option<Notice> {
        self.notice.lock().unwrap_or_else(|e| e.into_inner()).take()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_request_taken_once() {
        let b = MidnightBridge::default();
        assert!(!b.take_toggle());
        b.request_toggle();
        assert!(b.take_toggle());
        assert!(!b.take_toggle());
    }

    #[test]
    fn teammate_notice_taken_once() {
        let b = MidnightBridge::default();
        b.notify(Notice::TeammateUnmodded);
        assert_eq!(b.take_notice(), Some(Notice::TeammateUnmodded));
        assert_eq!(b.take_notice(), None);
    }

    #[test]
    fn notice_taken_once() {
        let b = MidnightBridge::default();
        b.notify(Notice::RelicsBlocked);
        assert_eq!(b.take_notice(), Some(Notice::RelicsBlocked));
        assert_eq!(b.take_notice(), None);
    }
}
