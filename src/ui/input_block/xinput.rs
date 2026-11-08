//! 手柄：钩住 XInputGetState（游戏按序号 2 从 xinput1_4 导入），记录最新的按键状态供面板使用；
//! 屏蔽期间给游戏返回空状态，面板下面的遗物界面就不会跟着动。

use std::ffi::c_void;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

use hudhook::mh::MhHook;
use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};

use super::{active, create_hook, should_block};

/// 面板使用的按键位：低 16 位与 XInput 的 wButtons 相同，高位为扳机与左摇杆方向
pub mod pad {
    pub const DPAD_UP: u32 = 0x0001;
    pub const DPAD_DOWN: u32 = 0x0002;
    pub const DPAD_LEFT: u32 = 0x0004;
    pub const DPAD_RIGHT: u32 = 0x0008;
    pub const START: u32 = 0x0010;
    pub const BACK: u32 = 0x0020;
    pub const L3: u32 = 0x0040;
    pub const R3: u32 = 0x0080;
    pub const LB: u32 = 0x0100;
    pub const RB: u32 = 0x0200;
    pub const A: u32 = 0x1000;
    pub const B: u32 = 0x2000;
    pub const X: u32 = 0x4000;
    pub const Y: u32 = 0x8000;
    pub const LT: u32 = 1 << 16;
    pub const RT: u32 = 1 << 17;
    pub const STICK_UP: u32 = 1 << 18;
    pub const STICK_DOWN: u32 = 1 << 19;
    pub const STICK_LEFT: u32 = 1 << 20;
    pub const STICK_RIGHT: u32 = 1 << 21;
    /// 方向键与左摇杆合并
    pub const UP: u32 = DPAD_UP | STICK_UP;
    pub const DOWN: u32 = DPAD_DOWN | STICK_DOWN;
    pub const LEFT: u32 = DPAD_LEFT | STICK_LEFT;
    pub const RIGHT: u32 = DPAD_RIGHT | STICK_RIGHT;

    /// "L3"、"Back"、"LB+RB" 这类写法 -> 按键位组合；有不认识的键名时返回 None
    pub fn parse_combo(text: &str) -> Option<u32> {
        text.split('+').try_fold(0, |acc, name| {
            let bit = match name.trim().to_ascii_uppercase().as_str() {
                "A" => A,
                "B" => B,
                "X" => X,
                "Y" => Y,
                "LB" => LB,
                "RB" => RB,
                "LT" => LT,
                "RT" => RT,
                "L3" | "LS" => L3,
                "R3" | "RS" => R3,
                "BACK" | "VIEW" | "SELECT" => BACK,
                "START" | "MENU" => START,
                _ => return None,
            };
            Some(acc | bit)
        })
    }
}

const STICK_DEADZONE: i16 = 16000;
const TRIGGER_THRESHOLD: u8 = 60;

static TRAMPOLINE: AtomicUsize = AtomicUsize::new(0);
/// 四个手柄槽位各自最新的按键位
static PADS: [AtomicU32; 4] = [AtomicU32::new(0), AtomicU32::new(0), AtomicU32::new(0), AtomicU32::new(0)];

#[repr(C)]
#[derive(Default)]
struct Gamepad {
    buttons: u16,
    left_trigger: u8,
    right_trigger: u8,
    thumb_lx: i16,
    thumb_ly: i16,
    thumb_rx: i16,
    thumb_ry: i16,
}

#[repr(C)]
struct XInputState {
    packet: u32,
    gamepad: Gamepad,
}

type GetStateFn = unsafe extern "system" fn(u32, *mut XInputState) -> u32;

/// 所有已连接手柄的按键位合并（游戏每帧轮询时更新）
pub fn pad_buttons() -> u32 {
    PADS.iter().fold(0, |acc, p| acc | p.load(Ordering::Acquire))
}

pub fn create_hooks() -> Result<Vec<MhHook>, String> {
    let name: Vec<u16> = "xinput1_4.dll\0".encode_utf16().collect();
    let module = unsafe { GetModuleHandleW(name.as_ptr()) };
    if module == 0 {
        return Err("xinput1_4.dll not loaded".into());
    }
    let target = unsafe { GetProcAddress(module, c"XInputGetState".as_ptr().cast()) }.ok_or("XInputGetState missing")?;
    Ok(vec![create_hook(target as usize, get_state as *mut c_void, &TRAMPOLINE)?])
}

fn bits(g: &Gamepad) -> u32 {
    let mut b = g.buttons as u32;
    b |= if g.left_trigger > TRIGGER_THRESHOLD { pad::LT } else { 0 };
    b |= if g.right_trigger > TRIGGER_THRESHOLD { pad::RT } else { 0 };
    b |= if g.thumb_ly > STICK_DEADZONE { pad::STICK_UP } else { 0 };
    b |= if g.thumb_ly < -STICK_DEADZONE { pad::STICK_DOWN } else { 0 };
    b |= if g.thumb_lx < -STICK_DEADZONE { pad::STICK_LEFT } else { 0 };
    b |= if g.thumb_lx > STICK_DEADZONE { pad::STICK_RIGHT } else { 0 };
    b
}

unsafe extern "system" fn get_state(user: u32, state: *mut XInputState) -> u32 {
    let orig: GetStateFn = unsafe { std::mem::transmute(TRAMPOLINE.load(Ordering::Acquire)) };
    let ret = unsafe { orig(user, state) };
    let slot = PADS.get(user as usize);
    if ret != 0 || state.is_null() {
        slot.inspect(|s| s.store(0, Ordering::Release));
        return ret;
    }
    let gamepad = unsafe { &mut (*state).gamepad };
    let b = bits(gamepad);
    slot.inspect(|s| s.store(b, Ordering::Release));
    if active() && should_block(b != 0) {
        *gamepad = Gamepad::default();
    }
    ret
}

#[cfg(test)]
mod tests {
    use super::pad::*;

    #[test]
    fn parses_button_combos() {
        assert_eq!(parse_combo("L3"), Some(L3));
        assert_eq!(parse_combo(" lb + RB "), Some(LB | RB));
        assert_eq!(parse_combo("View"), Some(BACK));
        assert_eq!(parse_combo("Z"), None);
    }
}
