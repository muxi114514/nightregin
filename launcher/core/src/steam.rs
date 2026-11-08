//! Steam：当前登录的账号（注册表 ActiveUser）与 Steam 库里的游戏目录

use std::path::PathBuf;

use windows_sys::Win32::Foundation::ERROR_SUCCESS;
use windows_sys::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegGetValueW};

use crate::paths::GameDir;
use crate::text::wide;

/// 32 位账号 ID 加上它就是 SteamID64（存档目录名）
pub const STEAM_ID64_BASE: u64 = 76561197960265728;
pub const NIGHTREIGN_APP_ID: u32 = 2622380;

/// Steam 运行时把当前账号写在这里
const ACTIVE_PROCESS_KEY: &str = r"Software\Valve\Steam\ActiveProcess";
const ACTIVE_USER_VALUE: &str = "ActiveUser";

/// 0 表示没登录，为 None
pub fn steam_id64(account_id: u32) -> Option<u64> {
    (account_id != 0).then(|| STEAM_ID64_BASE + u64::from(account_id))
}

/// HKCU\Software\Valve\Steam\ActiveProcess 的 ActiveUser 原值（0 = Steam 开着但没登录）；
/// 键或值不存在、不是 DWORD 时为 None
pub fn read_active_user() -> Option<u32> {
    let key = wide(ACTIVE_PROCESS_KEY);
    let value = wide(ACTIVE_USER_VALUE);
    let mut data: u32 = 0;
    let mut len = size_of::<u32>() as u32;
    // SAFETY: 两个字符串以 NUL 结尾；data 可写 len 字节；不取值类型，传空指针
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            key.as_ptr(),
            value.as_ptr(),
            RRF_RT_REG_DWORD,
            std::ptr::null_mut(),
            (&raw mut data).cast(),
            &mut len,
        )
    };
    (status == ERROR_SUCCESS).then_some(data)
}

/// Steam 库清单里的游戏根目录，且其中有 Game\nightreign.exe；Steam 或游戏没装、清单读不了时为 None
pub fn locate_game_via_steam() -> Option<PathBuf> {
    let steam = steamlocate::locate().ok()?;
    let (app, library) = match steam.find_app(NIGHTREIGN_APP_ID) {
        Ok(found) => found?,
        Err(e) => {
            log::warn!("读取 Steam 库清单失败：{e}");
            return None;
        }
    };
    let game = GameDir {
        root: library.resolve_app_dir(&app),
    };
    game.exe().is_file().then_some(game.root)
}
