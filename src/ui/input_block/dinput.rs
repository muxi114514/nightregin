//! 键盘：钩住 IDirectInputDevice8 的 GetDeviceState / GetDeviceData，屏蔽期间清空结果。
//! 所有 DirectInput 设备共用同一张虚表，用临时创建的键盘设备取得函数地址即可。

use std::ffi::c_void;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

use hudhook::mh::MhHook;
use log::info;
use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress, LoadLibraryW};
use windows_sys::core::GUID;

use super::{active, create_hook, should_block, trace};

/// 已记录过的数据大小（位掩码），首次见到时写一行日志，确认游戏走的是哪个接口
static SEEN: AtomicU32 = AtomicU32::new(0);

static STATE_TRAMPOLINES: [AtomicUsize; 2] = [AtomicUsize::new(0), AtomicUsize::new(0)];
static DATA_TRAMPOLINES: [AtomicUsize; 2] = [AtomicUsize::new(0), AtomicUsize::new(0)];

const IID_DIRECTINPUT8W: GUID = GUID::from_u128(0xBF798031_483A_4DA2_AA99_5D64ED369700);
const IID_DIRECTINPUT8A: GUID = GUID::from_u128(0xBF798030_483A_4DA2_AA99_5D64ED369700);
const GUID_SYS_KEYBOARD: GUID = GUID::from_u128(0x6F1D2B61_D5A0_11CF_BFC7_444553540000);
const DIRECTINPUT_VERSION: u32 = 0x0800;

// COM 虚表下标
const VT_RELEASE: usize = 2;
const VT_CREATE_DEVICE: usize = 3;
const VT_GET_DEVICE_STATE: usize = 9;
const VT_GET_DEVICE_DATA: usize = 10;

const KEYBOARD_STATE_SIZE: u32 = 256;
/// DIMOUSESTATE / DIMOUSESTATE2：按键从偏移 12 开始
const MOUSE_STATE_SIZES: [u32; 2] = [16, 20];
const MOUSE_BUTTONS_OFFSET: usize = 12;

type DirectInput8CreateFn = unsafe extern "system" fn(isize, u32, *const GUID, *mut *mut c_void, *mut c_void) -> i32;
type CreateDeviceFn = unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void, *mut c_void) -> i32;
type ReleaseFn = unsafe extern "system" fn(*mut c_void) -> u32;
type GetStateFn = unsafe extern "system" fn(*mut c_void, u32, *mut c_void) -> i32;
type GetDataFn = unsafe extern "system" fn(*mut c_void, u32, *mut c_void, *mut u32, u32) -> i32;

pub fn create_hooks() -> Result<Vec<MhHook>, String> {
    let targets = unsafe { device_vtable_targets() }?;
    let mut hooks = Vec::new();
    for (i, &(state_fn, data_fn)) in targets.iter().enumerate() {
        // A / W 两个版本可能共用实现，同一个地址只钩一次
        if i == 1 && targets[0] == targets[1] {
            break;
        }
        hooks.push(create_hook(state_fn, detour_state(i), &STATE_TRAMPOLINES[i])?);
        hooks.push(create_hook(data_fn, detour_data(i), &DATA_TRAMPOLINES[i])?);
    }
    Ok(hooks)
}

fn detour_state(i: usize) -> *mut c_void {
    if i == 0 { get_device_state::<0> as *mut c_void } else { get_device_state::<1> as *mut c_void }
}

fn detour_data(i: usize) -> *mut c_void {
    if i == 0 { get_device_data::<0> as *mut c_void } else { get_device_data::<1> as *mut c_void }
}

/// 创建临时的 DirectInput8（W 与 A）与键盘设备，读出 GetDeviceState / GetDeviceData 的地址
unsafe fn device_vtable_targets() -> Result<[(usize, usize); 2], String> {
    let name: Vec<u16> = "dinput8.dll\0".encode_utf16().collect();
    let module = unsafe { LoadLibraryW(name.as_ptr()) };
    if module == 0 {
        return Err("dinput8.dll not loadable".into());
    }
    let create = unsafe { GetProcAddress(module, c"DirectInput8Create".as_ptr().cast()) }.ok_or("DirectInput8Create missing")?;
    let create: DirectInput8CreateFn = unsafe { std::mem::transmute(create) };
    let instance = unsafe { GetModuleHandleW(std::ptr::null()) };
    let mut out = [(0, 0); 2];
    for (slot, iid) in out.iter_mut().zip([IID_DIRECTINPUT8W, IID_DIRECTINPUT8A]) {
        let mut di: *mut c_void = std::ptr::null_mut();
        let hr = unsafe { create(instance, DIRECTINPUT_VERSION, &iid, &mut di, std::ptr::null_mut()) };
        if hr < 0 || di.is_null() {
            return Err(format!("DirectInput8Create failed {hr:#x}"));
        }
        let di_vt = unsafe { *(di as *const *const usize) };
        let create_device: CreateDeviceFn = unsafe { std::mem::transmute(*di_vt.add(VT_CREATE_DEVICE)) };
        let mut dev: *mut c_void = std::ptr::null_mut();
        let hr = unsafe { create_device(di, &GUID_SYS_KEYBOARD, &mut dev, std::ptr::null_mut()) };
        if hr >= 0 && !dev.is_null() {
            let dev_vt = unsafe { *(dev as *const *const usize) };
            *slot = unsafe { (*dev_vt.add(VT_GET_DEVICE_STATE), *dev_vt.add(VT_GET_DEVICE_DATA)) };
            unsafe { release(dev) };
        }
        unsafe { release(di) };
        if hr < 0 || slot.0 == 0 {
            return Err(format!("CreateDevice(keyboard) failed {hr:#x}"));
        }
    }
    Ok(out)
}

unsafe fn release(obj: *mut c_void) {
    let vt = unsafe { *(obj as *const *const usize) };
    let f: ReleaseFn = unsafe { std::mem::transmute(*vt.add(VT_RELEASE)) };
    unsafe { f(obj) };
}

unsafe extern "system" fn get_device_state<const I: usize>(dev: *mut c_void, size: u32, data: *mut c_void) -> i32 {
    let orig: GetStateFn = unsafe { std::mem::transmute(STATE_TRAMPOLINES[I].load(Ordering::Acquire)) };
    let hr = unsafe { orig(dev, size, data) };
    if hr >= 0 && !data.is_null() {
        note_seen("GetDeviceState", size);
        let bytes = unsafe { std::slice::from_raw_parts_mut(data as *mut u8, size as usize) };
        if active() && should_block(any_pressed(bytes)) {
            trace::note("GetDeviceState", i64::from(size));
            bytes.fill(0);
        }
    }
    hr
}

unsafe extern "system" fn get_device_data<const I: usize>(
    dev: *mut c_void,
    object_size: u32,
    events: *mut c_void,
    count: *mut u32,
    flags: u32,
) -> i32 {
    let orig: GetDataFn = unsafe { std::mem::transmute(DATA_TRAMPOLINES[I].load(Ordering::Acquire)) };
    let hr = unsafe { orig(dev, object_size, events, count, flags) };
    if hr >= 0 && !count.is_null() && unsafe { *count } > 0 {
        note_seen("GetDeviceData", object_size | 0x8000_0000);
        // 缓冲事件被丢弃即可：屏蔽期间按下的键游戏看不到按下，也就不需要看到松开
        if should_block(false) {
            trace::note("GetDeviceData", i64::from(object_size));
            unsafe { *count = 0 };
        }
    }
    hr
}

/// 键盘 / 鼠标状态里是否还有按下的键（其它设备不解析）
fn any_pressed(state: &[u8]) -> bool {
    match state.len() as u32 {
        KEYBOARD_STATE_SIZE => state.iter().any(|b| b & 0x80 != 0),
        n if MOUSE_STATE_SIZES.contains(&n) => state[MOUSE_BUTTONS_OFFSET..].iter().any(|b| b & 0x80 != 0),
        _ => false,
    }
}

fn note_seen(api: &str, size: u32) {
    let bit = match size & 0x7FFF_FFFF {
        KEYBOARD_STATE_SIZE => 0,
        n if MOUSE_STATE_SIZES.contains(&n) => 1,
        _ => 2,
    } + if size & 0x8000_0000 != 0 { 3 } else { 0 };
    if SEEN.fetch_or(1 << bit, Ordering::AcqRel) & (1 << bit) == 0 {
        info!("game reads input via {api} (data size {})", size & 0x7FFF_FFFF);
    }
}
