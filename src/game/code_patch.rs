//! 改写游戏代码段里的单个字节：VirtualProtect 临时放开写权限，写完恢复原保护并刷新指令缓存。
//! 单字节写入是原子的，游戏线程同时执行到这里时只会看到改前或改后的指令。

use std::ffi::c_void;

use windows_sys::Win32::System::Diagnostics::Debug::FlushInstructionCache;
use windows_sys::Win32::System::Memory::{PAGE_EXECUTE_READWRITE, PAGE_PROTECTION_FLAGS, VirtualProtect};
use windows_sys::Win32::System::Threading::GetCurrentProcess;

/// 把 addr 处的代码字节改成 value；放开写权限失败时不改动并返回 false
pub fn write_code_byte(addr: usize, value: u8) -> bool {
    let target = addr as *const c_void;
    let mut old: PAGE_PROTECTION_FLAGS = 0;
    if unsafe { VirtualProtect(target, 1, PAGE_EXECUTE_READWRITE, &mut old) } == 0 {
        return false;
    }
    unsafe { (addr as *mut u8).write_volatile(value) };
    let mut ignored: PAGE_PROTECTION_FLAGS = 0;
    unsafe {
        VirtualProtect(target, 1, old, &mut ignored);
        FlushInstructionCache(GetCurrentProcess(), target, 1);
    }
    true
}
