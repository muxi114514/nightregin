//! 文本查找钩子：MsgRepositoryImp 按 (分类, 编号) 取文本（rcx 仓库、edx 分类、r8d 编号，返回以 0 结尾的 UTF-16，查不到为空）。
//! 取菜单文本 0x7AC800（570 处调用）→ 0x7B1A00 → 0xE5B070 → 这里，所以钩这一处就覆盖全部文本。
//! 提供函数给出文本就直接返回它（须为 'static 且以 0 结尾），否则交给原函数。可能在多个线程调用。

use std::ffi::c_void;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};

use log::{info, warn};

use crate::hook;

pub type TextProvider = fn(u32, u32) -> Option<&'static [u16]>;
type LookupFn = unsafe extern "C" fn(usize, u32, u32) -> *const u16;

static TRAMPOLINE: AtomicUsize = AtomicUsize::new(0);
static PROVIDER: OnceLock<TextProvider> = OnceLock::new();

/// 需在叠加层之后调用；只生效一次
pub fn install(addr: usize, provider: TextProvider) -> bool {
    if PROVIDER.set(provider).is_err() {
        warn!("msg lookup hook already installed");
        return false;
    }
    if !hook::ensure_initialized() {
        warn!("msg lookup hook disabled: MinHook unavailable");
        return false;
    }
    let ok = match hook::create(addr, lookup_detour as *mut c_void, &TRAMPOLINE) {
        Ok(h) => hook::enable(&[h]),
        Err(e) => {
            warn!("msg lookup not hooked: {e}");
            false
        }
    };
    info!("msg lookup hook installed: {ok}");
    ok
}

unsafe extern "C" fn lookup_detour(repo: usize, category: u32, id: u32) -> *const u16 {
    if let Some(text) = PROVIDER.get().and_then(|p| p(category, id)) {
        return text.as_ptr();
    }
    let original: LookupFn = unsafe { std::mem::transmute(TRAMPOLINE.load(Ordering::Acquire)) };
    unsafe { original(repo, category, id) }
}
