//! 游戏自己的窗口过程（窗口类注册的那个函数，出征中的鼠标按键就在这里处理）：hudhook 在窗口过程链前面按面板状态过滤键鼠消息，
//! 这里在游戏的函数上再拦一道，面板打开时到达游戏的键鼠消息一律丢弃，不依赖 hudhook 子类化的位置；拦下的会记进诊断日志。

use std::ffi::c_void;
use std::sync::atomic::{AtomicUsize, Ordering};

use hudhook::mh::MhHook;
use windows_sys::Win32::Foundation::{BOOL, HWND, LPARAM, RECT};
use windows_sys::Win32::System::Threading::GetCurrentProcessId;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GCLP_WNDPROC, GetClassLongPtrW, GetWindowRect, GetWindowThreadProcessId, IsWindowVisible, WM_KEYFIRST, WM_KEYLAST,
    WM_MOUSEFIRST, WM_MOUSELAST,
};

use super::{active, create_hook, should_block, trace};

static TRAMPOLINE: AtomicUsize = AtomicUsize::new(0);

type WndProcFn = unsafe extern "system" fn(HWND, u32, usize, isize) -> isize;

pub fn create_hooks() -> Result<Vec<MhHook>, String> {
    let hwnd = main_window().ok_or("game window not found")?;
    let target = unsafe { GetClassLongPtrW(hwnd, GCLP_WNDPROC) };
    if target == 0 {
        return Err("window class procedure not found".into());
    }
    Ok(vec![create_hook(target, wnd_proc as *mut c_void, &TRAMPOLINE)?])
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: usize, lparam: isize) -> isize {
    let input = (WM_KEYFIRST..=WM_KEYLAST).contains(&msg) || (WM_MOUSEFIRST..=WM_MOUSELAST).contains(&msg);
    if input && active() && should_block(false) {
        trace::note("WndProc", msg as i64);
        return 0;
    }
    let orig: WndProcFn = unsafe { std::mem::transmute(TRAMPOLINE.load(Ordering::Acquire)) };
    unsafe { orig(hwnd, msg, wparam, lparam) }
}

/// 本进程最大的可见顶层窗口
fn main_window() -> Option<HWND> {
    struct Best {
        hwnd: HWND,
        area: i64,
    }
    unsafe extern "system" fn visit(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let best = unsafe { &mut *(lparam as *mut Best) };
        let mut pid = 0;
        unsafe { GetWindowThreadProcessId(hwnd, &mut pid) };
        if pid == unsafe { GetCurrentProcessId() } && unsafe { IsWindowVisible(hwnd) } != 0 {
            let mut r = RECT { left: 0, top: 0, right: 0, bottom: 0 };
            unsafe { GetWindowRect(hwnd, &mut r) };
            let area = i64::from(r.right - r.left) * i64::from(r.bottom - r.top);
            if area > best.area {
                *best = Best { hwnd, area };
            }
        }
        1
    }
    let mut best = Best { hwnd: 0, area: 0 };
    unsafe { EnumWindows(Some(visit), &mut best as *mut Best as LPARAM) };
    (best.hwnd != 0).then_some(best.hwnd)
}
