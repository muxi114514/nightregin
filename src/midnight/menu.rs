//! 出击菜单与深夜面板识别：右侧面板由 0x9FFA20 按界面名载入（rdx 指向的结构 +8 为宽字符串名），
//! 只有出击菜单的两处调用（0x9EF256 深夜 02_221_ChaosMatch_Status、0x9EF2BF 夜王 02_220_NightBoss_Status），
//! 钩它记下当前是哪种面板；出击菜单是否打开看当前对话框是不是 Matching* 系列。钩子跑在游戏线程。

use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use log::{info, warn};

use crate::game::addresses::{Addresses, MidnightMenuAddresses};
use crate::game::memory::{read, read_ptr};
use crate::game::menu;
use crate::hook;

const NAME_CAP: usize = 40;
const NAME_OFFSET: usize = 8;
const DEEP_PANEL: &str = "02_221_ChaosMatch_Status";
const NIGHT_BOSS_PANEL: &str = "02_220_NightBoss_Status";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelKind {
    Deep,
    NightBoss,
    Other,
}

pub fn panel_kind(name: &str) -> PanelKind {
    match name {
        DEEP_PANEL => PanelKind::Deep,
        NIGHT_BOSS_PANEL => PanelKind::NightBoss,
        _ => PanelKind::Other,
    }
}

fn wide_to_string(buf: &[u16]) -> String {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end])
}

/// (面板容器, &{类型, 名字…})
type PanelLoaderFn = unsafe extern "C" fn(usize, usize) -> usize;

static TRAMPOLINE: AtomicUsize = AtomicUsize::new(0);
static DEEP_SHOWN: AtomicBool = AtomicBool::new(false);

/// 需在叠加层之后调用
pub fn install(addrs: &MidnightMenuAddresses) {
    let Some(at) = addrs.panel_loader else { return };
    if !hook::ensure_initialized() {
        warn!("midnight menu hook disabled: MinHook unavailable");
        return;
    }
    match hook::create(at, panel_detour as *mut c_void, &TRAMPOLINE) {
        Ok(h) => info!("midnight panel hook installed: {}", hook::enable(&[h])),
        Err(e) => warn!("midnight panel hook unavailable: {e}"),
    }
}

/// 右侧最近载入的是深夜面板
pub fn deep_panel_loaded() -> bool {
    DEEP_SHOWN.load(Ordering::Acquire)
}

/// 出击菜单（Matching 系列对话框）正在显示
pub fn sortie_menu_open(addrs: &Addresses, menu_addrs: &MidnightMenuAddresses) -> bool {
    menu::showing_any(addrs, &menu_addrs.matching_dialogs)
}

/// 出击菜单 = 右侧面板载入那一刻正在显示的对话框（按对象与虚表记，不依赖猜类名）：
/// 实测出击菜单当前对话框不是三个 Matching 类之一，所以在面板载入时把它记下来
#[derive(Debug, Default)]
pub struct SortieTracker {
    dialog: Option<(usize, usize)>,
}

impl SortieTracker {
    /// panel_event = 自上帧以来右侧面板载入过；showing = 当前显示的对话框 (对象, 虚表)；返回出击菜单是否还开着
    pub fn update(&mut self, panel_event: bool, showing: Option<(usize, usize)>) -> bool {
        if panel_event {
            self.dialog = showing;
        }
        showing.is_some() && showing == self.dialog
    }
}

static PANEL_EVENT: AtomicBool = AtomicBool::new(false);

/// 自上次调用以来右侧面板是否载入过
pub fn take_panel_event() -> bool {
    PANEL_EVENT.swap(false, Ordering::AcqRel)
}

/// 出征开始后清掉选择记录（选择要保留到出击那一刻用来判断这局是不是午夜，弹出确认框时也不能丢）
pub fn reset_panel() {
    DEEP_SHOWN.store(false, Ordering::Release);
}

/// 记下右侧载入的面板；返回是否有变化
fn note_panel(name: &str) -> bool {
    PANEL_EVENT.store(true, Ordering::Release);
    let deep = panel_kind(name) == PanelKind::Deep;
    DEEP_SHOWN.swap(deep, Ordering::AcqRel) != deep
}

unsafe extern "C" fn panel_detour(container: usize, desc: usize) -> usize {
    let name = read_ptr(desc + NAME_OFFSET).and_then(|p| read::<[u16; NAME_CAP]>(p)).map(|b| wide_to_string(&b)).unwrap_or_default();
    if note_panel(&name) {
        info!("midnight: right panel {name}");
    }
    let original: PanelLoaderFn = unsafe { std::mem::transmute(TRAMPOLINE.load(Ordering::Acquire)) };
    unsafe { original(container, desc) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_panel_name() {
        assert_eq!(panel_kind("02_221_ChaosMatch_Status"), PanelKind::Deep);
        assert_eq!(panel_kind("02_220_NightBoss_Status"), PanelKind::NightBoss);
        assert_eq!(panel_kind("02_400_Antique"), PanelKind::Other);
        assert_eq!(panel_kind(""), PanelKind::Other);
    }

    #[test]
    fn deep_selection_survives_until_reset() {
        note_panel("02_221_ChaosMatch_Status");
        assert!(deep_panel_loaded());
        note_panel("02_220_NightBoss_Status");
        assert!(!deep_panel_loaded());
        note_panel("02_221_ChaosMatch_Status");
        reset_panel();
        assert!(!deep_panel_loaded());
    }

    #[test]
    fn sortie_dialog_is_the_one_showing_when_panel_loaded() {
        let mut t = SortieTracker::default();
        assert!(!t.update(false, Some((0x100, 0x200))));
        assert!(t.update(true, Some((0x100, 0x200))));
        assert!(t.update(false, Some((0x100, 0x200))));
        // 弹窗关了、换成别的对话框、同地址换了类型都算关
        assert!(!t.update(false, None));
        assert!(!t.update(false, Some((0x300, 0x400))));
        assert!(!t.update(false, Some((0x100, 0x999))));
        assert!(t.update(false, Some((0x100, 0x200))));
    }

    #[test]
    fn wide_name_stops_at_zero() {
        let mut buf = [0u16; NAME_CAP];
        for (i, c) in "02_221_ChaosMatch_Status".encode_utf16().enumerate() {
            buf[i] = c;
        }
        assert_eq!(wide_to_string(&buf), "02_221_ChaosMatch_Status");
    }
}
