//! 诊断：面板打开（屏蔽输入）期间游戏仍在调用的键鼠读取接口，记下接口与调用位置（模块 + 偏移）。
//! 每个 (接口, 调用位置) 只记一次，整个会话最多 MAX_NOTES 条；"操作穿透到游戏"时按日志找漏掉的路径。

use std::ffi::c_void;
use std::sync::Mutex;

use log::info;
use windows_sys::Win32::System::Diagnostics::Debug::RtlCaptureStackBackTrace;
use windows_sys::Win32::System::LibraryLoader::{
    GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT, GetModuleFileNameW, GetModuleHandleExW,
};

const MAX_NOTES: usize = 48;
const MAX_FRAMES: usize = 8;

static SEEN: Mutex<Vec<(&'static str, usize)>> = Mutex::new(Vec::new());

/// 在钩子里调用（游戏线程）；锁被占用时直接放弃，不阻塞游戏
pub fn note(api: &'static str, detail: i64) {
    let caller = external_caller();
    let Ok(mut seen) = SEEN.try_lock() else { return };
    if seen.len() >= MAX_NOTES || seen.contains(&(api, caller)) {
        return;
    }
    seen.push((api, caller));
    drop(seen);
    info!("input while panel open: {api} ({detail:#x}) from {}", describe(caller));
}

/// 栈上第一个不属于本 DLL 的返回地址，即调用被钩接口的游戏代码
fn external_caller() -> usize {
    let mut frames = [std::ptr::null_mut::<c_void>(); MAX_FRAMES];
    let n = unsafe { RtlCaptureStackBackTrace(0, MAX_FRAMES as u32, frames.as_mut_ptr(), std::ptr::null_mut()) } as usize;
    let own = module_of(external_caller as *const () as usize);
    frames[..n.min(MAX_FRAMES)].iter().map(|&f| f as usize).find(|&a| module_of(a) != own).unwrap_or(0)
}

fn module_of(addr: usize) -> isize {
    let mut module = 0;
    let flags = GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT;
    unsafe { GetModuleHandleExW(flags, addr as *const u16, &mut module) };
    module
}

fn describe(addr: usize) -> String {
    let module = module_of(addr);
    if module == 0 {
        return format!("{addr:#x}");
    }
    let mut buf = [0u16; 260];
    let len = unsafe { GetModuleFileNameW(module, buf.as_mut_ptr(), buf.len() as u32) } as usize;
    let path = String::from_utf16_lossy(&buf[..len]);
    let name = path.rsplit('\\').next().unwrap_or(&path);
    format!("{name}+{:#x}", addr - module as usize)
}
