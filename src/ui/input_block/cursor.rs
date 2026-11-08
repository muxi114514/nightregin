//! 屏蔽期间的光标外观。游戏的窗口类光标是系统箭头，只有它判定"鼠标活跃"时才在 WM_SETCURSOR 里
//! SetCursor 成自己的蓝色光标（exe 内光标资源 101），闲置一会还会 ShowCursor 隐藏。
//! 面板打开时它读到的指针位置被冻结，会判定为闲置，于是先变成系统箭头、再消失。
//! 所以屏蔽期间由我们接管：WM_SETCURSOR 与 SetCursor 一律换成蓝色光标，忽略隐藏请求并保证光标可见。

use std::ffi::c_void;
use std::sync::atomic::{AtomicIsize, AtomicUsize, Ordering};

use hudhook::mh::MhHook;
use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use windows_sys::Win32::UI::WindowsAndMessaging::{CURSOR_SHOWING, CURSORINFO, GetCursorInfo, HTCLIENT, LoadCursorW, WM_SETCURSOR};

use super::{active, create_hook};

/// nightreign.exe 中唯一的光标资源（RT_GROUP_CURSOR 101），即游戏的蓝色光标
const GAME_CURSOR_ID: usize = 101;
/// 让光标显示计数回到 >= 0 的最多尝试次数
const MAX_SHOW_TRIES: usize = 16;

static GAME_CURSOR: AtomicIsize = AtomicIsize::new(0);
static SET_CURSOR_TRAMPOLINE: AtomicUsize = AtomicUsize::new(0);
static SHOW_CURSOR_TRAMPOLINE: AtomicUsize = AtomicUsize::new(0);
static DEF_WINDOW_PROC_TRAMPOLINE: AtomicUsize = AtomicUsize::new(0);

type SetCursorFn = unsafe extern "system" fn(isize) -> isize;
type ShowCursorFn = unsafe extern "system" fn(i32) -> i32;
type DefWindowProcFn = unsafe extern "system" fn(isize, u32, usize, isize) -> isize;

pub fn create_hooks() -> Result<Vec<MhHook>, String> {
    let cursor = unsafe { LoadCursorW(GetModuleHandleW(std::ptr::null()), GAME_CURSOR_ID as *const u16) };
    if cursor == 0 {
        return Err("game cursor resource 101 not found".into());
    }
    GAME_CURSOR.store(cursor, Ordering::Release);

    let name: Vec<u16> = "user32.dll\0".encode_utf16().collect();
    let user32 = unsafe { GetModuleHandleW(name.as_ptr()) };
    let export = |sym: &std::ffi::CStr| {
        unsafe { GetProcAddress(user32, sym.as_ptr().cast()) }.map(|f| f as usize).ok_or(format!("{sym:?} missing"))
    };
    Ok(vec![
        create_hook(export(c"SetCursor")?, set_cursor as *mut c_void, &SET_CURSOR_TRAMPOLINE)?,
        create_hook(export(c"ShowCursor")?, show_cursor as *mut c_void, &SHOW_CURSOR_TRAMPOLINE)?,
        create_hook(export(c"DefWindowProcW")?, def_window_proc as *mut c_void, &DEF_WINDOW_PROC_TRAMPOLINE)?,
    ])
}

unsafe extern "system" fn set_cursor(cursor: isize) -> isize {
    let orig: SetCursorFn = unsafe { std::mem::transmute(SET_CURSOR_TRAMPOLINE.load(Ordering::Acquire)) };
    if active() {
        ensure_visible();
        return unsafe { orig(GAME_CURSOR.load(Ordering::Acquire)) };
    }
    unsafe { orig(cursor) }
}

/// 屏蔽期间忽略隐藏请求；返回 -1 让游戏"隐藏到计数 < 0 为止"的循环正常结束
unsafe extern "system" fn show_cursor(show: i32) -> i32 {
    if show == 0 && active() {
        return -1;
    }
    let orig: ShowCursorFn = unsafe { std::mem::transmute(SHOW_CURSOR_TRAMPOLINE.load(Ordering::Acquire)) };
    unsafe { orig(show) }
}

/// 游戏判定鼠标闲置时把 WM_SETCURSOR 交给默认处理（显示系统箭头），屏蔽期间改为蓝色光标
unsafe extern "system" fn def_window_proc(hwnd: isize, msg: u32, wparam: usize, lparam: isize) -> isize {
    if msg == WM_SETCURSOR && (lparam & 0xFFFF) as u32 == HTCLIENT && active() {
        unsafe { set_cursor(GAME_CURSOR.load(Ordering::Acquire)) };
        return 1;
    }
    let orig: DefWindowProcFn = unsafe { std::mem::transmute(DEF_WINDOW_PROC_TRAMPOLINE.load(Ordering::Acquire)) };
    unsafe { orig(hwnd, msg, wparam, lparam) }
}

/// 光标当前被隐藏时把显示计数加回 >= 0（只在窗口线程上的 SetCursor 路径调用，计数属于该线程）
fn ensure_visible() {
    let mut info = CURSORINFO { cbSize: size_of::<CURSORINFO>() as u32, flags: 0, hCursor: 0, ptScreenPos: unsafe { std::mem::zeroed() } };
    if unsafe { GetCursorInfo(&mut info) } == 0 || info.flags & CURSOR_SHOWING != 0 {
        return;
    }
    let orig: ShowCursorFn = unsafe { std::mem::transmute(SHOW_CURSOR_TRAMPOLINE.load(Ordering::Acquire)) };
    for _ in 0..MAX_SHOW_TRIES {
        if unsafe { orig(1) } >= 0 {
            break;
        }
    }
}
