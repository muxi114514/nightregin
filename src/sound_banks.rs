//! 额外的 Wwise 音频包：把 DLL 旁 sound_banks\*.bnk 读进内存交给游戏的声音引擎（移植武器的专属音效用）。
//! 游戏只会按角色自动加载 cs_cXXXX，武器音效要自己加载；放进 pkg 的 sd\ 反而会顶掉同名的原版包，所以单独放一个目录。
//! nightreign.exe 导出了 AK::SoundEngine（C++ 修饰名）。LoadBankMemoryCopy 是同步调用，引擎自己拷一份，可以在任意线程调用；
//! 包在整局游戏里都留着（队友拿这把武器时也要听得到），不卸载。

use std::ffi::c_void;
use std::path::{Path, PathBuf};
use std::time::Duration;

use log::{error, info, warn};
use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};

const FOLDER: &str = "sound_banks";
const IS_INITIALIZED: &[u8] = b"?IsInitialized@SoundEngine@AK@@YA_NXZ\0";
const LOAD_BANK: &[u8] = b"?LoadBankMemoryCopy@SoundEngine@AK@@YA?AW4AKRESULT@@PEBXKAEAK@Z\0";
const AK_SUCCESS: i32 = 1;
const WAIT_STEP: Duration = Duration::from_secs(1);
const WAIT_LIMIT: u32 = 600;

type IsInitializedFn = unsafe extern "C" fn() -> bool;
/// (包数据, 大小, &输出包编号) -> AKRESULT
type LoadBankFn = unsafe extern "C" fn(*const c_void, u32, *mut u32) -> i32;

/// 有音频包才开线程等声音引擎；目录不存在就是不需要这个功能
pub fn load_in_background(dll_dir: &Path) {
    let banks = list_banks(&dll_dir.join(FOLDER));
    if banks.is_empty() {
        return;
    }
    std::thread::spawn(move || load_all(&banks));
}

fn list_banks(folder: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(folder) else { return Vec::new() };
    entries.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("bnk"))).collect()
}

fn load_all(banks: &[PathBuf]) {
    let Some((is_initialized, load_bank)) = locate() else {
        error!("sound banks disabled: Wwise exports not found");
        return;
    };
    let mut waited = 0;
    while !unsafe { is_initialized() } {
        if waited >= WAIT_LIMIT {
            error!("sound banks disabled: sound engine not initialized after {waited} s");
            return;
        }
        std::thread::sleep(WAIT_STEP);
        waited += 1;
    }
    for path in banks {
        load_one(path, load_bank);
    }
}

fn load_one(path: &Path, load_bank: LoadBankFn) {
    let data = match std::fs::read(path) {
        Ok(d) => d,
        Err(e) => {
            warn!("cannot read sound bank {}: {e}", path.display());
            return;
        }
    };
    let mut bank_id = 0u32;
    let result = unsafe { load_bank(data.as_ptr().cast(), data.len() as u32, &mut bank_id) };
    if result == AK_SUCCESS {
        info!("sound bank {} loaded, id {bank_id}, {} bytes", path.display(), data.len());
    } else {
        warn!("sound bank {} failed: AKRESULT {result}", path.display());
    }
}

fn locate() -> Option<(IsInitializedFn, LoadBankFn)> {
    unsafe {
        let exe = GetModuleHandleW(std::ptr::null());
        let is_initialized = GetProcAddress(exe, IS_INITIALIZED.as_ptr())?;
        let load_bank = GetProcAddress(exe, LOAD_BANK.as_ptr())?;
        Some((std::mem::transmute::<_, IsInitializedFn>(is_initialized), std::mem::transmute::<_, LoadBankFn>(load_bank)))
    }
}
