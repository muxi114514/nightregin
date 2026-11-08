//! 进 Tauri 之前的启动步骤：单实例预检、找整合包、检查 WebView2、读设置并补查上次退出；
//! 这时还没有窗口，出错用原生对话框提示

use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr::null_mut;

use launcher_core::exit_check::settle;
use launcher_core::os::{GAME_EXE, ME3_EXE, ME3_LAUNCHER_EXE, Os};
use launcher_core::paths::{
    GameDir, PACK_ROOT_ENV, PROFILE_FILE, PackLayout, PathsError, find_game_dir,
};
use launcher_core::settings::{Settings, SettingsStore};
use launcher_core::steam::locate_game_via_steam;
use launcher_core::text::wide;
use windows_sys::Win32::Foundation::{CloseHandle, ERROR_ACCESS_DENIED, ERROR_SUCCESS, FALSE};
use windows_sys::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ, RegGetValueW,
};
use windows_sys::Win32::System::Threading::{OpenMutexW, SYNCHRONIZATION_SYNCHRONIZE};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    IDYES, MB_ICONERROR, MB_ICONINFORMATION, MB_ICONQUESTION, MB_OK, MB_SETFOREGROUND, MB_YESNO,
    MESSAGEBOX_RESULT, MESSAGEBOX_STYLE, MessageBoxW,
};

/// WebView2 运行时的版本号（pv）登记处：先查全机安装，再查当前用户安装
const WEBVIEW2_KEYS: [(HKEY, &str); 2] = [
    (
        HKEY_LOCAL_MACHINE,
        r"SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}",
    ),
    (
        HKEY_CURRENT_USER,
        r"Software\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}",
    ),
];
/// 整合包自带的 WebView2 安装程序，在 <整合包>\launcher\redist 下
const WEBVIEW2_SETUP: &str = "MicrosoftEdgeWebview2Setup.exe";
/// pv 这类短字符串的读取上限（UTF-16 单元）
const REG_SZ_MAX: usize = 128;
/// 任一还在运行就算上次的启动没结束：Wake 之后关掉又马上重开时，me3 可能还没把游戏拉起来
const LAUNCH_PROCESSES: [&str; 3] = [GAME_EXE, ME3_EXE, ME3_LAUNCHER_EXE];

const WEBVIEW2_INSTALL_PROMPT: &str = "没有检测到 WebView2 运行时，启动器需要它来显示界面。\n\n现在安装吗？安装程序需要联网，装完后启动器会继续打开。";
const WEBVIEW2_MISSING: &str = "没有检测到 WebView2 运行时，启动器无法显示界面。\n\n请到群文件下载「WebView2 离线安装包」，安装后再运行启动器。";

/// 第 0 步：单实例插件的互斥体已存在，说明已有启动器在运行
pub fn instance_running(identifier: &str) -> bool {
    let name = wide(&instance_mutex_name(identifier));
    // SAFETY: name 以 NUL 结尾，调用期间有效
    let handle = unsafe { OpenMutexW(SYNCHRONIZATION_SYNCHRONIZE, FALSE, name.as_ptr()) };
    if handle.is_null() {
        // 已有的实例以管理员身份运行时打不开，但互斥体同样存在
        let error = std::io::Error::last_os_error().raw_os_error();
        return error == Some(ERROR_ACCESS_DENIED as i32);
    }
    // SAFETY: handle 是刚打开的有效句柄，只关这一次
    unsafe { CloseHandle(handle) };
    true
}

/// 单实例插件（tauri-plugin-single-instance 2.5，未开 semver）的互斥体名：应用标识符 + "-sim"
pub fn instance_mutex_name(identifier: &str) -> String {
    format!("{identifier}-sim")
}

/// 第 1 步：先看 NRGA_PACK_ROOT 再看 exe 所在目录；找不到就弹框说明，返回 None
pub fn discover_pack(title: &str) -> Option<PackLayout> {
    let env = std::env::var_os(PACK_ROOT_ENV).filter(|value| !value.is_empty());
    match PackLayout::discover(&exe_dir(), env.as_deref().map(Path::new)) {
        Ok(pack) => Some(pack),
        Err(PathsError::PackNotFound { searched }) => {
            message_box(title, &missing_pack_text(&searched), MB_ICONERROR | MB_OK);
            None
        }
    }
}

/// exe 所在目录；取不到时用当前目录
pub fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_default()
}

/// 找不到整合包时的提示，列出找过的目录
pub fn missing_pack_text(searched: &[PathBuf]) -> String {
    let dirs: Vec<String> = searched
        .iter()
        .map(|dir| dir.display().to_string())
        .collect();
    format!(
        "找不到整合包：下面的目录里都没有「{PROFILE_FILE}」。\n\n{}\n\n请从整合包目录运行启动器：它要和「{PROFILE_FILE}」放在同一个文件夹里。",
        dirs.join("\n")
    )
}

/// 第 3 步：没装 WebView2 时，整合包带了安装程序就问是否安装（不提权，等它结束再查）；
/// 仍然没有就提示去群文件下载离线安装包，返回 false
pub fn ensure_webview2(pack: &PackLayout, title: &str) -> bool {
    if webview2_installed() {
        return true;
    }
    log::warn!("没有检测到 WebView2 运行时");
    let setup = pack.launcher_dir().join("redist").join(WEBVIEW2_SETUP);
    if setup.is_file()
        && message_box(title, WEBVIEW2_INSTALL_PROMPT, MB_ICONQUESTION | MB_YESNO) == IDYES
    {
        install_webview2(&setup);
        if webview2_installed() {
            return true;
        }
    }
    log::error!("WebView2 运行时仍然缺失，退出");
    message_box(title, WEBVIEW2_MISSING, MB_ICONERROR | MB_OK);
    false
}

/// 注册表里的 pv 是 WebView2 运行时的版本号；没有、为空或 0.0.0.0 都算没装
pub fn pv_is_installed(pv: Option<&str>) -> bool {
    pv.map(str::trim)
        .is_some_and(|pv| !pv.is_empty() && pv != "0.0.0.0")
}

fn webview2_installed() -> bool {
    WEBVIEW2_KEYS
        .iter()
        .any(|&(root, key)| pv_is_installed(read_reg_sz(root, key, "pv").as_deref()))
}

fn install_webview2(setup: &Path) {
    log::info!("运行 WebView2 安装程序：{}", setup.display());
    match Command::new(setup).args(["/silent", "/install"]).status() {
        Ok(status) => log::info!("WebView2 安装程序已退出：{status}"),
        Err(e) => log::error!("无法运行 WebView2 安装程序：{e}"),
    }
}

/// 第 4 步：读设置。上次游戏启动后启动器就关了（exit_check_pending）、游戏已不在运行时，
/// 按启动之后有没有新的真崩溃转储（退出时的已知崩溃不算）补判，写回 last_exit_abnormal 并清掉待查标记
pub fn load_settings(pack: &PackLayout, os: &dyn Os) -> (SettingsStore, Settings) {
    let store = SettingsStore::new(pack.settings_file());
    let (mut settings, note) = store.load();
    if let Some(note) = note {
        log::warn!("{note}");
    }
    if settle_exit_check(&mut settings, pack, os)
        && let Err(e) = store.save(&settings)
    {
        log::error!("保存设置失败：{e}");
    }
    (store, settings)
}

/// 补查上次游戏退出：这里只查进程、找游戏目录，判定交给 settle；改了设置时返回 true
fn settle_exit_check(settings: &mut Settings, pack: &PackLayout, os: &dyn Os) -> bool {
    if !settings.exit_check_pending {
        return false;
    }
    let running = LAUNCH_PROCESSES
        .iter()
        .find_map(|&name| os.find_process(name).map(|pid| (name, pid)));
    if let Some((name, pid)) = running {
        log::info!("上次启动的 {name} 还在运行（pid {pid}），下次启动再补查");
    }
    // 还在运行时用不上转储目录，不去找游戏（可能要查 Steam）
    let dumps = match running {
        Some(_) => None,
        None => find_game_dir(
            &pack.root,
            settings.game_dir.as_deref(),
            &locate_game_via_steam,
        )
        .as_ref()
        .map(GameDir::dump_reports),
    };
    let changed = settle(settings, running.is_some(), dumps.as_deref());
    if changed {
        log::info!(
            "补查上次游戏退出：{}",
            if settings.last_exit_abnormal {
                "异常"
            } else {
                "正常"
            }
        );
    }
    changed
}

/// 第 2 步装不上日志时的提示：日志和设置都在 <整合包>\launcher 下，设置多半同样存不下
pub fn log_unavailable_text(dir: &Path, error: &std::io::Error) -> String {
    format!(
        "无法写入日志目录：\n{}\n\n原因：{error}\n\n启动器会照常运行，但这次不会留下日志，设置也可能无法保存。",
        dir.display()
    )
}

/// 是 / 否对话框，选「是」返回 true
pub fn confirm(title: &str, text: &str) -> bool {
    message_box(title, text, MB_ICONQUESTION | MB_YESNO) == IDYES
}

pub fn show_info(title: &str, text: &str) {
    message_box(title, text, MB_ICONINFORMATION | MB_OK);
}

/// 错误提示框（建窗口失败等）
pub fn show_error(title: &str, text: &str) {
    message_box(title, text, MB_ICONERROR | MB_OK);
}

/// 原生对话框，返回按下的按钮
fn message_box(title: &str, text: &str, style: MESSAGEBOX_STYLE) -> MESSAGEBOX_RESULT {
    let (title, text) = (wide(title), wide(text));
    // SAFETY: 两个字符串以 NUL 结尾，调用期间有效；没有父窗口
    unsafe {
        MessageBoxW(
            null_mut(),
            text.as_ptr(),
            title.as_ptr(),
            style | MB_SETFOREGROUND,
        )
    }
}

/// 读 REG_SZ 值；键或值不存在、类型不对或超长时为 None
fn read_reg_sz(root: HKEY, key: &str, value: &str) -> Option<String> {
    let (key, value) = (wide(key), wide(value));
    let mut buf = [0u16; REG_SZ_MAX];
    let mut len = size_of_val(&buf) as u32;
    // SAFETY: key、value 以 NUL 结尾；buf 可写 len 字节；不取值类型，传空指针
    let status = unsafe {
        RegGetValueW(
            root,
            key.as_ptr(),
            value.as_ptr(),
            RRF_RT_REG_SZ,
            null_mut(),
            buf.as_mut_ptr().cast(),
            &mut len,
        )
    };
    if status != ERROR_SUCCESS {
        return None;
    }
    // len 是写入的字节数，含结尾 NUL
    let chars = (len as usize / 2).min(REG_SZ_MAX);
    Some(
        String::from_utf16_lossy(&buf[..chars])
            .trim_end_matches('\0')
            .to_owned(),
    )
}

#[cfg(test)]
mod tests {
    use std::fs::{self, File};
    use std::io;
    use std::path::{Path, PathBuf};
    use std::time::{Duration, Instant, SystemTime};

    use launcher_core::os::{ChildProc, GAME_EXE, LaunchCommand, ME3_EXE, ME3_LAUNCHER_EXE, Os};
    use launcher_core::paths::{GameDir, PackLayout};
    use launcher_core::settings::Settings;
    use tempfile::TempDir;

    use super::{
        instance_mutex_name, log_unavailable_text, missing_pack_text, pv_is_installed,
        settle_exit_check,
    };

    #[test]
    fn pv_checks() {
        assert!(!pv_is_installed(None));
        assert!(!pv_is_installed(Some("")));
        assert!(!pv_is_installed(Some("0.0.0.0")));
        assert!(pv_is_installed(Some("131.0.2903.70")));
    }

    #[test]
    fn startup_error_text_for_missing_pack() {
        let searched = [
            PathBuf::from(r"D:\游戏\黑夜君临\me3"),
            PathBuf::from(r"C:\Users\玩家\Desktop"),
        ];
        let text = missing_pack_text(&searched);
        assert!(text.contains("褪色燃烧.me3"), "{text}");
        assert!(text.contains("请从整合包目录运行"), "{text}");
        for dir in &searched {
            assert!(text.contains(&*dir.to_string_lossy()), "{text}");
        }
    }

    /// 与 tauri-plugin-single-instance 2.5 的 platform_impl\windows.rs 一致：标识符 + "-sim"；
    /// 标识符取自 tauri.conf.json，发布后不能改
    #[test]
    fn instance_mutex_name_matches_plugin() {
        let conf: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        assert_eq!(conf["identifier"], "com.maxz.nrga.launcher");
        assert_eq!(
            instance_mutex_name("com.maxz.nrga.launcher"),
            "com.maxz.nrga.launcher-sim"
        );
    }

    /// F1：提示写明日志目录和原因，并说明设置也可能存不下
    #[test]
    fn log_unavailable_text_names_dir_and_error() {
        let error = io::Error::from(io::ErrorKind::PermissionDenied);
        let text = log_unavailable_text(Path::new(r"D:\游戏\me3\launcher\logs"), &error);
        assert!(text.contains(r"D:\游戏\me3\launcher\logs"), "{text}");
        assert!(
            text.contains(&error.to_string()) && text.contains("设置"),
            "{text}"
        );
    }

    /// 只回答「哪些进程在运行」的假 Os
    struct FakeOs(Vec<&'static str>);

    impl Os for FakeOs {
        fn now(&self) -> Instant {
            Instant::now()
        }
        fn system_time(&self) -> SystemTime {
            SystemTime::now()
        }
        fn sleep(&self, _: Duration) {}
        fn find_process(&self, exe_name: &str) -> Option<u32> {
            self.0
                .iter()
                .any(|name| name.eq_ignore_ascii_case(exe_name))
                .then_some(4242)
        }
        fn has_visible_window(&self, _: u32) -> bool {
            false
        }
        fn focus_process_window(&self, _: u32) -> bool {
            false
        }
        fn spawn(&self, _: &LaunchCommand) -> io::Result<Box<dyn ChildProc>> {
            Err(io::Error::other("测试不唤起进程"))
        }
        fn open_process(&self, _: u32) -> Option<Box<dyn ChildProc>> {
            None
        }
        fn kill_by_name(&self, _: &str) -> usize {
            0
        }
    }

    /// 某次唤起的时刻（Unix 秒）
    const T: i64 = 1_762_400_000;

    fn pending(at: Option<i64>) -> Settings {
        Settings {
            exit_check_pending: true,
            last_launch_at: at,
            ..Settings::default()
        }
    }

    fn unix(secs: i64) -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(u64::try_from(secs).unwrap())
    }

    /// 在 dir 下放一个崩溃转储，修改时间为 at
    fn dump(dir: &Path, at: SystemTime) {
        fs::create_dir_all(dir).unwrap();
        File::create(dir.join("crash.dmp"))
            .unwrap()
            .set_modified(at)
            .unwrap();
    }

    /// 临时整合包 + 手选的游戏目录（有 Game\nightreign.exe，不会去查 Steam）；
    /// 设置待查、在 T 唤起，游戏的转储目录里有一个 T 之后的转储
    fn pending_pack() -> (TempDir, PackLayout, Settings) {
        let tmp = TempDir::new().unwrap();
        let game = GameDir {
            root: tmp.path().join("game"),
        };
        fs::create_dir_all(game.root.join("Game")).unwrap();
        File::create(game.exe()).unwrap();
        dump(&game.dump_reports(), unix(T + 30));
        let pack = PackLayout {
            root: tmp.path().join("pack"),
        };
        (
            tmp,
            pack,
            Settings {
                game_dir: Some(game.root),
                ..pending(Some(T))
            },
        )
    }

    /// LC-6：nightreign.exe、me3.exe、me3-launcher.exe 任一还在运行，都当作上次的启动没结束，不动设置
    #[test]
    fn settle_exit_check_waits_for_any_launch_process() {
        for name in [GAME_EXE, ME3_EXE, ME3_LAUNCHER_EXE] {
            let (_tmp, pack, mut s) = pending_pack();
            let before = s.clone();
            assert!(
                !settle_exit_check(&mut s, &pack, &FakeOs(vec![name])),
                "{name}"
            );
            assert_eq!(s, before, "{name}");
        }
        let (_tmp, pack, mut s) = pending_pack();
        assert!(settle_exit_check(&mut s, &pack, &FakeOs(Vec::new())));
        assert_eq!((s.last_exit_abnormal, s.exit_check_pending), (true, false));
    }
}
