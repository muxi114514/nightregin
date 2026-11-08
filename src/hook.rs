//! MinHook 小工具：创建钩子并保存跳板地址、批量启用。MinHook 由 hudhook 初始化，必须在叠加层安装之后使用；
//! 叠加层关闭时先调用 ensure_initialized。

use std::ffi::c_void;
use std::sync::atomic::{AtomicUsize, Ordering};

use hudhook::mh::{MH_ApplyQueued, MH_Initialize, MH_STATUS, MhHook};

/// 已初始化时 MinHook 返回 ALREADY_INITIALIZED，视为成功
pub fn ensure_initialized() -> bool {
    matches!(unsafe { MH_Initialize() }, MH_STATUS::MH_OK | MH_STATUS::MH_ERROR_ALREADY_INITIALIZED)
}

/// 创建钩子并把跳板地址存入 slot（启用前必须先存好，钩子函数通过它调用原函数）
pub fn create(target: usize, detour: *mut c_void, slot: &AtomicUsize) -> Result<MhHook, String> {
    let hook = unsafe { MhHook::new(target as *mut c_void, detour) }.map_err(|e| format!("hook {target:#x} failed: {e:?}"))?;
    slot.store(hook.trampoline() as usize, Ordering::Release);
    Ok(hook)
}

/// 一次性启用全部钩子（MinHook 统一挂起线程再改写入口）
pub fn enable(hooks: &[MhHook]) -> bool {
    hooks.iter().all(|h| unsafe { h.queue_enable() }.is_ok()) && unsafe { MH_ApplyQueued() }.ok().is_ok()
}
