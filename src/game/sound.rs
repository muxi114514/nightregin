//! Wwise 音频引擎：nightreign.exe 直接导出了 AK::SoundEngine 的函数（C++ 修饰名），按导出名取地址调用。
//! SoundEngine 的接口只往音频线程的消息队列投递，官方保证线程安全，渲染线程也可以直接调用。

use std::ffi::{CStr, c_char, c_void};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

use log::{info, warn};
use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};

pub const POST_EVENT: &CStr = c"?PostEvent@SoundEngine@AK@@YAKK_KKP6AXW4AkCallbackType@@PEAUAkCallbackInfo@@@ZPEAXKPEAUAkExternalSourceInfo@@K@Z";
pub const GET_ID_FROM_STRING: &CStr = c"?GetIDFromString@SoundEngine@AK@@YAKPEBD@Z";
const REGISTER_GAME_OBJ: &CStr = c"?RegisterGameObj@SoundEngine@AK@@YA?AW4AKRESULT@@_KPEBD@Z";

/// 本模组自己的 Wwise 游戏对象。游戏用对象地址作 ID，取一个不可能是堆地址的小值避免冲突；
/// 新对象自动挂到游戏设置的默认监听者上，菜单音效是 2D 的，不需要设置位置
pub const OWN_OBJECT: u64 = 0x4E53_4100;

/// 菜单音效（实测：遗物装配界面移动光标 / 左键确定 / 右键返回时游戏播放的事件）
pub mod menu {
    use super::event_id;

    pub const CURSOR: u32 = event_id("Play_f000000000");
    pub const CONFIRM: u32 = event_id("Play_f000000100");
    pub const CANCEL: u32 = event_id("Play_f000000200");
}

/// Wwise 事件名 -> ID：名字转小写后的 32 位 FNV-1 哈希（与 GetIDFromString 相同）
pub const fn event_id(name: &str) -> u32 {
    let bytes = name.as_bytes();
    let mut hash: u32 = 2166136261;
    let mut i = 0;
    while i < bytes.len() {
        hash = hash.wrapping_mul(16777619) ^ bytes[i].to_ascii_lowercase() as u32;
        i += 1;
    }
    hash
}

/// PostEvent(事件 ID, 游戏对象, 回调标志, 回调, 回调参数, 外部源数量, 外部源, 播放 ID) -> 播放 ID（0 为失败）
pub type PostEventFn = unsafe extern "system" fn(u32, u64, u32, *const c_void, *mut c_void, u32, *const c_void, u32) -> u32;
pub type GetIdFn = unsafe extern "system" fn(*const c_char) -> u32;
type RegisterFn = unsafe extern "system" fn(u64, *const c_char) -> i32;

struct Engine {
    post_event: PostEventFn,
    register: RegisterFn,
}

static ENGINE: OnceLock<Option<Engine>> = OnceLock::new();
static WARNED: AtomicBool = AtomicBool::new(false);

/// 游戏主模块导出的函数地址
pub fn export(name: &CStr) -> Option<usize> {
    let module = unsafe { GetModuleHandleW(std::ptr::null()) };
    unsafe { GetProcAddress(module, name.as_ptr().cast()) }.map(|f| f as usize)
}

fn engine() -> Option<&'static Engine> {
    ENGINE
        .get_or_init(|| {
            let (post, register) = (export(POST_EVENT)?, export(REGISTER_GAME_OBJ)?);
            info!("Wwise exports: PostEvent {post:#x}, RegisterGameObj {register:#x}");
            Some(unsafe { Engine { post_event: std::mem::transmute(post), register: std::mem::transmute(register) } })
        })
        .as_ref()
}

/// 在本模组的游戏对象上播放事件，返回播放 ID（0 为失败）。
/// 每次都先注册对象：读图时游戏可能注销全部对象，重复注册只是一条队列消息，代价可以忽略
pub fn post(event: u32) -> u32 {
    let Some(e) = engine() else {
        if !WARNED.swap(true, Ordering::Relaxed) {
            warn!("Wwise exports not found, sound effects disabled");
        }
        return 0;
    };
    unsafe {
        (e.register)(OWN_OBJECT, c"NRStatAlloc".as_ptr());
        (e.post_event)(event, OWN_OBJECT, 0, std::ptr::null(), std::ptr::null_mut(), 0, std::ptr::null(), 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_ids_match_game() {
        // 诊断日志里游戏 GetIDFromString 返回的值
        assert_eq!(menu::CURSOR, 0x7ea5_9d80);
        assert_eq!(menu::CONFIRM, 0x84a7_e589);
        assert_eq!(menu::CANCEL, 0x0aaa_f772);
    }
}
