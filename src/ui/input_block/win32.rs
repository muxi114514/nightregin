//! 鼠标：游戏的鼠标轮询用 GetCursorPos 取位置、用 GetKeyboardState 读左 / 右 / 中键（反汇编确认），另有 GetKeyState / GetAsyncKeyState。
//! 这份状态在取出消息时就已更新，窗口消息过滤拦不住，所以屏蔽期间：清掉鼠标按键位，并把指针位置冻结在屏蔽开始时，
//! 避免面板下面的遗物跟着高亮。hudhook / ImGui 不调用这几个函数，不受影响。
//! 出征中（玩法状态）游戏每帧用 SetCursorPos 把光标拉回窗口中心、按读到的位置与中心之差转镜头（rva 0xF673D0）：
//! 屏蔽期间不真的移动光标（面板要用），只把冻结位置改成它设定的点，读到的差值就是 0；解除时把光标放回该点，镜头不会甩一下。

use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicUsize, Ordering};

use hudhook::mh::MhHook;
use windows_sys::Win32::Foundation::POINT;
use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};

use super::{active, create_hook, should_block, trace};

/// VK_LBUTTON / VK_RBUTTON / VK_MBUTTON / VK_XBUTTON1 / VK_XBUTTON2
const MOUSE_VKS: [usize; 5] = [0x01, 0x02, 0x04, 0x05, 0x06];

static KEYBOARD_STATE_TRAMPOLINE: AtomicUsize = AtomicUsize::new(0);
static KEY_STATE_TRAMPOLINE: AtomicUsize = AtomicUsize::new(0);
static ASYNC_KEY_STATE_TRAMPOLINE: AtomicUsize = AtomicUsize::new(0);
static CURSOR_POS_TRAMPOLINE: AtomicUsize = AtomicUsize::new(0);
static SET_CURSOR_POS_TRAMPOLINE: AtomicUsize = AtomicUsize::new(0);

static FROZEN: AtomicBool = AtomicBool::new(false);
static FROZEN_X: AtomicI32 = AtomicI32::new(0);
static FROZEN_Y: AtomicI32 = AtomicI32::new(0);
/// 屏蔽期间游戏要求过移动光标（玩法状态的拉回中心）
static RECENTERED: AtomicBool = AtomicBool::new(false);

type GetKeyboardStateFn = unsafe extern "system" fn(*mut u8) -> i32;
type GetKeyStateFn = unsafe extern "system" fn(i32) -> i16;
type GetCursorPosFn = unsafe extern "system" fn(*mut POINT) -> i32;
type SetCursorPosFn = unsafe extern "system" fn(i32, i32) -> i32;

pub fn create_hooks() -> Result<Vec<MhHook>, String> {
    let name: Vec<u16> = "user32.dll\0".encode_utf16().collect();
    let user32 = unsafe { GetModuleHandleW(name.as_ptr()) };
    if user32 == 0 {
        return Err("user32.dll not loaded".into());
    }
    let export = |sym: &std::ffi::CStr| {
        unsafe { GetProcAddress(user32, sym.as_ptr().cast()) }.map(|f| f as usize).ok_or(format!("{sym:?} missing"))
    };
    Ok(vec![
        create_hook(export(c"GetKeyboardState")?, get_keyboard_state as *mut c_void, &KEYBOARD_STATE_TRAMPOLINE)?,
        create_hook(export(c"GetKeyState")?, get_key_state as *mut c_void, &KEY_STATE_TRAMPOLINE)?,
        create_hook(export(c"GetAsyncKeyState")?, get_async_key_state as *mut c_void, &ASYNC_KEY_STATE_TRAMPOLINE)?,
        create_hook(export(c"GetCursorPos")?, get_cursor_pos as *mut c_void, &CURSOR_POS_TRAMPOLINE)?,
        create_hook(export(c"SetCursorPos")?, set_cursor_pos as *mut c_void, &SET_CURSOR_POS_TRAMPOLINE)?,
    ])
}

unsafe extern "system" fn get_keyboard_state(state: *mut u8) -> i32 {
    let orig: GetKeyboardStateFn = unsafe { std::mem::transmute(KEYBOARD_STATE_TRAMPOLINE.load(Ordering::Acquire)) };
    let ok = unsafe { orig(state) };
    if ok != 0 && !state.is_null() && active() {
        let keys = unsafe { std::slice::from_raw_parts_mut(state, 256) };
        let pressed = MOUSE_VKS.iter().any(|&vk| keys[vk] & 0x80 != 0);
        if should_block(pressed) {
            if pressed {
                trace::note("GetKeyboardState", 0);
            }
            for vk in MOUSE_VKS {
                keys[vk] = 0;
            }
        }
    }
    ok
}

unsafe extern "system" fn get_key_state(vk: i32) -> i16 {
    let orig: GetKeyStateFn = unsafe { std::mem::transmute(KEY_STATE_TRAMPOLINE.load(Ordering::Acquire)) };
    block_mouse_key("GetKeyState", vk, unsafe { orig(vk) })
}

unsafe extern "system" fn get_async_key_state(vk: i32) -> i16 {
    let orig: GetKeyStateFn = unsafe { std::mem::transmute(ASYNC_KEY_STATE_TRAMPOLINE.load(Ordering::Acquire)) };
    block_mouse_key("GetAsyncKeyState", vk, unsafe { orig(vk) })
}

/// 屏蔽期间鼠标键一律读作松开（键盘键不动：本模组与无缝联机的热键都用 GetAsyncKeyState）
fn block_mouse_key(api: &'static str, vk: i32, state: i16) -> i16 {
    let mouse = usize::try_from(vk).is_ok_and(|v| MOUSE_VKS.contains(&v));
    if mouse && active() && should_block(state < 0) {
        trace::note(api, i64::from(vk));
        return 0;
    }
    state
}

unsafe extern "system" fn get_cursor_pos(point: *mut POINT) -> i32 {
    let orig: GetCursorPosFn = unsafe { std::mem::transmute(CURSOR_POS_TRAMPOLINE.load(Ordering::Acquire)) };
    let ok = unsafe { orig(point) };
    if ok == 0 || point.is_null() {
        return ok;
    }
    let p = unsafe { &mut *point };
    if active() && should_block(false) {
        trace::note("GetCursorPos", 0);
        if FROZEN.swap(true, Ordering::AcqRel) {
            p.x = FROZEN_X.load(Ordering::Acquire);
            p.y = FROZEN_Y.load(Ordering::Acquire);
        } else {
            FROZEN_X.store(p.x, Ordering::Release);
            FROZEN_Y.store(p.y, Ordering::Release);
        }
    } else if FROZEN.swap(false, Ordering::AcqRel) && RECENTERED.swap(false, Ordering::AcqRel) {
        // 解除屏蔽：光标放回游戏最后设定的点，它就不会把面板期间的鼠标移动当成一次镜头转动
        let (x, y) = (FROZEN_X.load(Ordering::Acquire), FROZEN_Y.load(Ordering::Acquire));
        let set: SetCursorPosFn = unsafe { std::mem::transmute(SET_CURSOR_POS_TRAMPOLINE.load(Ordering::Acquire)) };
        unsafe { set(x, y) };
        (p.x, p.y) = (x, y);
    }
    ok
}

unsafe extern "system" fn set_cursor_pos(x: i32, y: i32) -> i32 {
    if active() && should_block(false) {
        trace::note("SetCursorPos", (i64::from(x) << 16) | i64::from(y));
        FROZEN_X.store(x, Ordering::Release);
        FROZEN_Y.store(y, Ordering::Release);
        FROZEN.store(true, Ordering::Release);
        RECENTERED.store(true, Ordering::Release);
        return 1;
    }
    let orig: SetCursorPosFn = unsafe { std::mem::transmute(SET_CURSOR_POS_TRAMPOLINE.load(Ordering::Acquire)) };
    unsafe { orig(x, y) }
}
