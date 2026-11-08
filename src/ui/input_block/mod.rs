//! 面板打开（或鼠标停在叠加层上）时屏蔽游戏的键鼠输入。窗口消息过滤之外，还要拦游戏直接读取的路径（实测）：
//! 键盘走 DirectInput8（dinput.rs）；鼠标按键读 GetKeyboardState 的 0x01/0x02/0x04 位与 GetKeyState、指针位置读 GetCursorPos，
//! 出征中还每帧用 SetCursorPos 把光标拉回中心算镜头转动（win32.rs）。
//! 手柄走 XInputGetState（xinput.rs，同时把手柄状态提供给面板）。
//! 冻结指针位置会让游戏以为鼠标闲置而换掉、隐藏光标，所以屏蔽期间光标外观也由我们接管（cursor.rs）。
//! 另在游戏自己的窗口过程上再拦一道键鼠消息（wndproc.rs），Raw Input 也清掉鼠标数据（rawinput.rs）；屏蔽期间仍被调用的读取接口记进日志（trace.rs）。
//! 原函数照常调用，屏蔽期间只改写返回给游戏的结果；解除后继续屏蔽到按键全部松开，避免关面板用的 Esc 等漏给游戏。

mod cursor;
mod dinput;
mod rawinput;
mod trace;
mod win32;
mod wndproc;
mod xinput;

pub use xinput::{pad, pad_buttons};

use std::sync::OnceLock;
use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};
use std::time::Instant;

use log::{info, warn};

use crate::hook::{self, create as create_hook};

const OFF: u8 = 0;
const ON: u8 = 1;
/// 已请求解除，等按键松开
const DRAIN: u8 = 2;
/// 松开后再保持的时间
const DRAIN_MS: u64 = 100;

static STATE: AtomicU8 = AtomicU8::new(OFF);
static LAST_PRESSED_MS: AtomicU64 = AtomicU64::new(0);

/// 叠加层每帧调用
pub fn set_blocked(block: bool) {
    if block {
        STATE.store(ON, Ordering::Release);
    } else if STATE.compare_exchange(ON, DRAIN, Ordering::AcqRel, Ordering::Acquire).is_ok() {
        LAST_PRESSED_MS.store(now_ms(), Ordering::Release);
    }
}

/// 需在 hudhook 初始化 MinHook 之后调用；某一类钩子失败不影响其它类
pub fn install() {
    let mut hooks = Vec::new();
    let groups = [
        ("DirectInput", dinput::create_hooks()),
        ("Win32", win32::create_hooks()),
        ("cursor", cursor::create_hooks()),
        ("XInput", xinput::create_hooks()),
        ("raw input", rawinput::create_hooks()),
        ("window procedure", wndproc::create_hooks()),
    ];
    for (name, result) in groups {
        match result {
            Ok(h) => hooks.extend(h),
            Err(e) => warn!("input blocking ({name}) unavailable: {e}"),
        }
    }
    if hooks.is_empty() {
        return;
    }
    if hook::enable(&hooks) {
        info!("input blocking hooks installed ({} functions)", hooks.len());
    } else {
        warn!("input blocking: enabling hooks failed");
    }
}

fn active() -> bool {
    STATE.load(Ordering::Acquire) != OFF
}

/// pressed：本次读到的设备状态里是否还有按下的键（只在解除过渡期用来判断能否放行）
fn should_block(pressed: bool) -> bool {
    match STATE.load(Ordering::Acquire) {
        ON => true,
        DRAIN => {
            let now = now_ms();
            if pressed {
                LAST_PRESSED_MS.store(now, Ordering::Release);
                true
            } else if now.saturating_sub(LAST_PRESSED_MS.load(Ordering::Acquire)) >= DRAIN_MS {
                let _ = STATE.compare_exchange(DRAIN, OFF, Ordering::AcqRel, Ordering::Acquire);
                false
            } else {
                true
            }
        }
        _ => false,
    }
}

fn now_ms() -> u64 {
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_millis() as u64
}
