//! 容错内存读写：通过 ReadProcessMemory / WriteProcessMemory 访问本进程，目标不可访问时返回 None / false 而不是崩溃。
//! 不要在热路径用 VirtualQuery 做可读性检查：它会逐页统计所在区域的大小，
//! 在游戏几 GB 的堆上一次要几毫秒（实测每次对账累计 200~300 ms，造成明显卡顿）。

use std::ffi::c_void;
use std::mem::{MaybeUninit, size_of};

use windows_sys::Win32::System::Diagnostics::Debug::{ReadProcessMemory, WriteProcessMemory};
use windows_sys::Win32::System::Memory::{
    MEM_COMMIT, MEMORY_BASIC_INFORMATION, PAGE_EXECUTE_READ, PAGE_EXECUTE_READWRITE, PAGE_EXECUTE_WRITECOPY,
    PAGE_GUARD, PAGE_NOACCESS, PAGE_READONLY, PAGE_READWRITE, PAGE_WRITECOPY, VirtualQuery,
};
use windows_sys::Win32::System::Threading::GetCurrentProcess;

const READABLE: u32 =
    PAGE_READONLY | PAGE_READWRITE | PAGE_WRITECOPY | PAGE_EXECUTE_READ | PAGE_EXECUTE_READWRITE | PAGE_EXECUTE_WRITECOPY;

/// 用户态地址的粗筛，过滤掉明显不是指针的值
pub fn looks_like_pointer(value: usize) -> bool {
    (0x10000..0x7FFF_FFFF_0000).contains(&value) && value % 8 == 0
}

/// 从 addr 复制 len 字节到 dst；任何一页不可读都返回 false
fn copy_from(addr: usize, dst: *mut c_void, len: usize) -> bool {
    if addr == 0 || len == 0 || addr.checked_add(len).is_none() {
        return false;
    }
    let mut copied = 0usize;
    let ok = unsafe { ReadProcessMemory(GetCurrentProcess(), addr as *const c_void, dst, len, &mut copied) };
    ok != 0 && copied == len
}

pub fn read<T: Copy>(addr: usize) -> Option<T> {
    let mut value = MaybeUninit::<T>::uninit();
    copy_from(addr, value.as_mut_ptr().cast(), size_of::<T>()).then(|| unsafe { value.assume_init() })
}

pub fn read_ptr(addr: usize) -> Option<usize> {
    read::<usize>(addr)
}

/// 写入一个值；目标不可写时返回 false
pub fn write<T: Copy>(addr: usize, value: T) -> bool {
    if addr == 0 || addr.checked_add(size_of::<T>()).is_none() {
        return false;
    }
    let mut written = 0usize;
    let src = &value as *const T as *const c_void;
    let ok = unsafe { WriteProcessMemory(GetCurrentProcess(), addr as *const c_void, src, size_of::<T>(), &mut written) };
    ok != 0 && written == size_of::<T>()
}

/// 一次写入一段字节；目标不可写时返回 false
pub fn write_bytes(addr: usize, bytes: &[u8]) -> bool {
    if addr == 0 || bytes.is_empty() || addr.checked_add(bytes.len()).is_none() {
        return false;
    }
    let mut written = 0usize;
    let ok = unsafe { WriteProcessMemory(GetCurrentProcess(), addr as *const c_void, bytes.as_ptr().cast(), bytes.len(), &mut written) };
    ok != 0 && written == bytes.len()
}

/// 读取一段字节；不可读时返回 None
pub fn read_bytes(addr: usize, len: usize) -> Option<Vec<u8>> {
    let mut buf = vec![0u8; len];
    copy_from(addr, buf.as_mut_ptr().cast(), len).then_some(buf)
}

/// [addr, addr + len) 是否整体可读。会走 VirtualQuery，只用于启动时检查模块代码段这类一次性操作
pub fn is_readable(addr: usize, len: usize) -> bool {
    if addr == 0 || len == 0 {
        return false;
    }
    let Some(end) = addr.checked_add(len) else { return false };
    let mut cursor = addr;
    while cursor < end {
        match readable_region(cursor) {
            Some(region_end) if region_end > cursor => cursor = region_end,
            _ => return false,
        }
    }
    true
}

/// addr 所在可读区域的终点
fn readable_region(addr: usize) -> Option<usize> {
    let mut info = MaybeUninit::<MEMORY_BASIC_INFORMATION>::zeroed();
    let written =
        unsafe { VirtualQuery(addr as *const c_void, info.as_mut_ptr(), size_of::<MEMORY_BASIC_INFORMATION>()) };
    if written == 0 {
        return None;
    }
    let info = unsafe { info.assume_init() };
    if info.State != MEM_COMMIT || info.Protect & (PAGE_GUARD | PAGE_NOACCESS) != 0 || info.Protect & READABLE == 0 {
        return None;
    }
    Some(info.BaseAddress as usize + info.RegionSize)
}

/// 读取以 0 结尾的 UTF-16 字符串（最多 max 个字符）
pub fn read_wstr(addr: usize, max: usize) -> Option<String> {
    let mut out = Vec::new();
    for i in 0..max {
        let c = read::<u16>(addr + i * 2)?;
        if c == 0 {
            return String::from_utf16(&out).ok();
        }
        out.push(c);
    }
    None
}

/// 读取以 0 结尾的 ASCII 字符串（最多 max 字节）
pub fn read_cstr(addr: usize, max: usize) -> Option<String> {
    let mut out = Vec::new();
    for i in 0..max {
        let b = read::<u8>(addr + i)?;
        if b == 0 {
            return Some(String::from_utf8_lossy(&out).into_owned());
        }
        out.push(b);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_valid_and_rejects_invalid_memory() {
        let value: u64 = 0x1122_3344_5566_7788;
        assert_eq!(read::<u64>(&value as *const u64 as usize), Some(value));
        assert_eq!(read::<u64>(0x10), None);
        assert_eq!(read::<u64>(usize::MAX - 3), None);
    }

    #[test]
    fn writes_valid_and_rejects_invalid_memory() {
        let mut value: i32 = -1;
        assert!(write(&mut value as *mut i32 as usize, 8670001i32));
        assert_eq!(value, 8670001);
        assert!(!write(0x10, 1i32));
    }
}
