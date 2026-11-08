//! Raw Input：游戏导入表里没有，但按名字取用的代码也可能读它（hudhook 不注册 Raw Input，不受影响）。
//! 面板打开时清掉取到的鼠标数据（缓冲读取当作没有输入），并记进诊断日志。

use std::ffi::c_void;
use std::sync::atomic::{AtomicUsize, Ordering};

use hudhook::mh::MhHook;
use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};

use super::{active, create_hook, should_block, trace};

const RID_INPUT: u32 = 0x1000_0003;
const RIM_TYPEMOUSE: u32 = 0;
/// x64 的 RAWINPUTHEADER 大小，RAWMOUSE 紧随其后
const HEADER_SIZE: usize = 24;
const ERROR: u32 = u32::MAX;

static DATA_TRAMPOLINE: AtomicUsize = AtomicUsize::new(0);
static BUFFER_TRAMPOLINE: AtomicUsize = AtomicUsize::new(0);

type GetRawInputDataFn = unsafe extern "system" fn(isize, u32, *mut c_void, *mut u32, u32) -> u32;
type GetRawInputBufferFn = unsafe extern "system" fn(*mut c_void, *mut u32, u32) -> u32;

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
        create_hook(export(c"GetRawInputData")?, get_raw_input_data as *mut c_void, &DATA_TRAMPOLINE)?,
        create_hook(export(c"GetRawInputBuffer")?, get_raw_input_buffer as *mut c_void, &BUFFER_TRAMPOLINE)?,
    ])
}

unsafe extern "system" fn get_raw_input_data(input: isize, command: u32, data: *mut c_void, size: *mut u32, header: u32) -> u32 {
    let orig: GetRawInputDataFn = unsafe { std::mem::transmute(DATA_TRAMPOLINE.load(Ordering::Acquire)) };
    let n = unsafe { orig(input, command, data, size, header) };
    if command == RID_INPUT && !data.is_null() && n != ERROR && n as usize > HEADER_SIZE && active() && should_block(false) {
        let kind = unsafe { *(data as *const u32) };
        trace::note("GetRawInputData", i64::from(kind));
        if kind == RIM_TYPEMOUSE {
            unsafe { std::slice::from_raw_parts_mut((data as *mut u8).add(HEADER_SIZE), n as usize - HEADER_SIZE) }.fill(0);
        }
    }
    n
}

unsafe extern "system" fn get_raw_input_buffer(data: *mut c_void, size: *mut u32, header: u32) -> u32 {
    let orig: GetRawInputBufferFn = unsafe { std::mem::transmute(BUFFER_TRAMPOLINE.load(Ordering::Acquire)) };
    let n = unsafe { orig(data, size, header) };
    if !data.is_null() && n != ERROR && n > 0 && active() && should_block(false) {
        trace::note("GetRawInputBuffer", i64::from(n));
        return 0;
    }
    n
}
