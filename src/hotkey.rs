//! 热键边沿检测；只在游戏窗口处于前台时响应，避免切到别的程序时误触。

use windows_sys::Win32::System::Threading::GetCurrentProcessId;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
use windows_sys::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

pub struct HotkeyState {
    keys: Vec<(u32, bool)>,
}

impl HotkeyState {
    /// 键码 0 表示关闭（GetAsyncKeyState(0) 偶尔会报按下，不能拿去轮询）
    pub fn new(keys: &[u32]) -> Self {
        Self { keys: keys.iter().filter(|&&k| k != 0).map(|&k| (k, false)).collect() }
    }

    /// 每帧调用一次，返回本帧刚按下的键
    pub fn poll(&mut self) -> Vec<u32> {
        let focused = game_window_focused();
        let mut pressed = Vec::new();
        for (vk, was_down) in self.keys.iter_mut() {
            let down = focused && (unsafe { GetAsyncKeyState(*vk as i32) } as u16 & 0x8000) != 0;
            if down && !*was_down {
                pressed.push(*vk);
            }
            *was_down = down;
        }
        pressed
    }
}

fn game_window_focused() -> bool {
    let mut pid = 0u32;
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd == 0 {
            return false;
        }
        GetWindowThreadProcessId(hwnd, &mut pid);
        pid == GetCurrentProcessId()
    }
}
