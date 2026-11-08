//! 进程与窗口：拼 me3 启动命令、唤起进程、按 exe 名找 / 结束进程、找进程的可见窗口、列崩溃转储。
//! 只有这里碰真实的进程和窗口；启动流程经由 Os trait 调用，测试时换成假的

use std::ffi::OsString;
use std::fs;
use std::io;
use std::os::windows::io::{AsRawHandle, HandleOrInvalid, HandleOrNull, OwnedHandle};
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant, SystemTime};

use windows_sys::Win32::Foundation::{
    ERROR_NO_MORE_FILES, FALSE, HWND, LPARAM, TRUE, WAIT_OBJECT_0, WAIT_TIMEOUT,
};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::System::Threading::{
    CREATE_NO_WINDOW, GetExitCodeProcess, OpenProcess, PROCESS_ACCESS_RIGHTS, PROCESS_NAME_WIN32,
    PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE, PROCESS_TERMINATE,
    QueryFullProcessImageNameW, TerminateProcess, WaitForSingleObject,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowThreadProcessId, IsIconic, IsWindowVisible, SW_RESTORE,
    SetForegroundWindow, ShowWindowAsync,
};
use windows_sys::core::BOOL;

use crate::paths::{GameDir, PackLayout};
use crate::retry;

// 与 paths 里的文件位置共用同一份定义
pub use crate::paths::{GAME_EXE, ME3_EXE};
pub const ME3_LAUNCHER_EXE: &str = "me3-launcher.exe";
pub const STEAM_EXE: &str = "steam.exe";

/// 崩溃转储的扩展名，比较时不区分大小写
const DUMP_EXT: &str = "dmp";
/// kill_by_name 结束进程时给的退出码
const KILLED_EXIT_CODE: u32 = 1;
/// 进程映像路径的缓冲区长度（UTF-16 单元）：NT 路径最长 32767 个字符
const MAX_IMAGE_PATH: usize = 32_768;

/// 要唤起的程序：参数逐个交给 Command，不拼成一条命令行
#[derive(Clone, Debug, PartialEq)]
pub struct LaunchCommand {
    pub program: PathBuf,
    pub args: Vec<OsString>,
    pub cwd: PathBuf,
}

/// <整合包>\bin\me3.exe launch -g nightreign -p <褪色燃烧.me3> -e <游戏根>\Game\nightreign.exe，
/// 工作目录是整合包根目录
pub fn me3_command(pack: &PackLayout, game: &GameDir) -> LaunchCommand {
    let args = vec![
        OsString::from("launch"),
        OsString::from("-g"),
        OsString::from("nightreign"),
        OsString::from("-p"),
        pack.profile().into_os_string(),
        OsString::from("-e"),
        game.exe().into_os_string(),
    ];
    LaunchCommand {
        program: pack.me3_exe(),
        args,
        cwd: pack.root.clone(),
    }
}

/// 唤起或按 pid 打开的进程
pub trait ChildProc: Send {
    fn pid(&self) -> u32;
    /// 还在运行时为 None；恰好以 259（与 STILL_ACTIVE 同值）退出的也如实返回
    fn try_exit_code(&mut self) -> Option<u32>;
}

/// 启动流程用到的系统能力：时钟、进程、窗口
pub trait Os: Send + Sync {
    fn now(&self) -> Instant;
    fn system_time(&self) -> SystemTime;
    fn sleep(&self, d: Duration);
    /// 按 exe 名（不区分大小写）找进程，有多个时取快照里的第一个
    fn find_process(&self, exe_name: &str) -> Option<u32>;
    /// pid 有没有可见的顶层窗口
    fn has_visible_window(&self, pid: u32) -> bool;
    /// 把 pid 的可见顶层窗口切到前台，最小化的先还原；找到窗口且切换成功才为 true
    fn focus_process_window(&self, pid: u32) -> bool;
    /// 不弹控制台窗口，标准输入输出都接空设备
    fn spawn(&self, cmd: &LaunchCommand) -> io::Result<Box<dyn ChildProc>>;
    /// 按 pid 打开已有进程以读退出码；进程不存在或无权访问时为 None
    fn open_process(&self, pid: u32) -> Option<Box<dyn ChildProc>>;
    /// 结束全部同名进程（不区分大小写，当前进程除外），返回结束了几个
    fn kill_by_name(&self, exe_name: &str) -> usize;
}

/// 真实的 Windows 实现
pub struct WinOs;

impl Os for WinOs {
    fn now(&self) -> Instant {
        Instant::now()
    }

    fn system_time(&self) -> SystemTime {
        SystemTime::now()
    }

    fn sleep(&self, d: Duration) {
        std::thread::sleep(d);
    }

    fn find_process(&self, exe_name: &str) -> Option<u32> {
        process_ids(exe_name).into_iter().next()
    }

    fn has_visible_window(&self, pid: u32) -> bool {
        visible_window(pid).is_some()
    }

    fn focus_process_window(&self, pid: u32) -> bool {
        visible_window(pid).is_some_and(bring_to_front)
    }

    fn spawn(&self, cmd: &LaunchCommand) -> io::Result<Box<dyn ChildProc>> {
        let child = Command::new(&cmd.program)
            .args(&cmd.args)
            .current_dir(&cmd.cwd)
            .creation_flags(CREATE_NO_WINDOW)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        Ok(Box::new(SpawnedProcess(child)))
    }

    fn open_process(&self, pid: u32) -> Option<Box<dyn ChildProc>> {
        // 只要查询和等待两项权限：对提权运行的进程也打得开
        match open(pid, PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE) {
            Ok(handle) => Some(Box::new(OpenedProcess { pid, handle })),
            Err(e) => {
                log::warn!("打开进程 {pid} 失败：{e}");
                None
            }
        }
    }

    fn kill_by_name(&self, exe_name: &str) -> usize {
        let me = std::process::id();
        let others = process_ids(exe_name).into_iter().filter(|&pid| pid != me);
        others
            .map(|pid| terminate(pid, exe_name))
            .filter(|&killed| killed)
            .count()
    }
}

/// spawn 起的进程；析构只关句柄，不结束进程
struct SpawnedProcess(Child);

impl ChildProc for SpawnedProcess {
    fn pid(&self) -> u32 {
        self.0.id()
    }

    fn try_exit_code(&mut self) -> Option<u32> {
        match self.0.try_wait() {
            // Windows 上 code() 总有值；崩溃码（如 0xC0000005）在 i32 里是负数，按位转回 u32
            Ok(status) => status.and_then(|s| s.code()).map(i32::cast_unsigned),
            Err(e) => {
                log::warn!("读取进程 {} 的退出码失败：{e}", self.0.id());
                None
            }
        }
    }
}

/// open_process 打开的进程；句柄析构时关闭
struct OpenedProcess {
    pid: u32,
    handle: OwnedHandle,
}

impl ChildProc for OpenedProcess {
    fn pid(&self) -> u32 {
        self.pid
    }

    /// 先看句柄有没有触发（进程已结束），再读退出码：只看退出码分不清「还在」和「以 259 退出」
    fn try_exit_code(&mut self) -> Option<u32> {
        let handle = self.handle.as_raw_handle();
        // SAFETY: handle 是带 SYNCHRONIZE 权限的有效进程句柄；超时 0 不阻塞
        let wait = unsafe { WaitForSingleObject(handle, 0) };
        if wait != WAIT_OBJECT_0 {
            if wait != WAIT_TIMEOUT {
                log::warn!("等待进程 {} 失败：{}", self.pid, io::Error::last_os_error());
            }
            return None;
        }
        let mut code = 0u32;
        // SAFETY: handle 带 PROCESS_QUERY_LIMITED_INFORMATION 权限；code 可写
        if unsafe { GetExitCodeProcess(handle, &mut code) } == FALSE {
            log::warn!(
                "读取进程 {} 的退出码失败：{}",
                self.pid,
                io::Error::last_os_error()
            );
            return None;
        }
        Some(code)
    }
}

/// 按 pid 以给定权限打开进程
fn open(pid: u32, access: PROCESS_ACCESS_RIGHTS) -> io::Result<OwnedHandle> {
    // SAFETY: 不涉及指针
    let raw = unsafe { OpenProcess(access, FALSE, pid) };
    // SAFETY: 失败时为 NULL，成功时是归调用方所有的新句柄
    let handle = unsafe { HandleOrNull::from_raw_handle(raw) };
    OwnedHandle::try_from(handle).map_err(|_| io::Error::last_os_error())
}

/// 确认 pid 现在仍是 exe_name 才结束它：快照之后原进程可能已退出、pid 被别的进程复用。
/// 拿到句柄后进程身份就固定了，核对与结束之间没有竞争；打不开、对不上或结束失败都返回 false
fn terminate(pid: u32, exe_name: &str) -> bool {
    let process = match open(pid, PROCESS_TERMINATE | PROCESS_QUERY_LIMITED_INFORMATION) {
        Ok(process) => process,
        Err(e) => {
            log::warn!("打开进程 {exe_name}（pid {pid}）失败：{e}");
            return false;
        }
    };
    match image_path(&process) {
        Ok(path) if image_named(&path, exe_name) => {}
        Ok(_) => {
            log::warn!("pid {pid} 已不是 {exe_name}，不结束");
            return false;
        }
        Err(e) => {
            log::warn!("读取进程 {exe_name}（pid {pid}）的映像路径失败，不结束：{e}");
            return false;
        }
    }
    // SAFETY: process 是带 PROCESS_TERMINATE 权限的有效句柄
    if unsafe { TerminateProcess(process.as_raw_handle(), KILLED_EXIT_CODE) } == FALSE {
        log::warn!(
            "结束进程 {exe_name}（pid {pid}）失败：{}",
            io::Error::last_os_error()
        );
        return false;
    }
    log::info!("已结束进程 {exe_name}（pid {pid}）");
    true
}

/// 进程映像的完整路径（Win32 格式的 UTF-16，不含结尾 NUL）
fn image_path(process: &OwnedHandle) -> io::Result<Vec<u16>> {
    let mut path = vec![0u16; MAX_IMAGE_PATH];
    let mut len = MAX_IMAGE_PATH as u32;
    // SAFETY: process 带 PROCESS_QUERY_LIMITED_INFORMATION 权限；path 可写 len 个 UTF-16 单元
    let ok = unsafe {
        QueryFullProcessImageNameW(
            process.as_raw_handle(),
            PROCESS_NAME_WIN32,
            path.as_mut_ptr(),
            &mut len,
        )
    };
    if ok == FALSE {
        return Err(io::Error::last_os_error());
    }
    // 成功时 len 是写入的字符数，不含 NUL
    path.truncate(len as usize);
    Ok(path)
}

/// 路径的最后一段（文件名）与 exe_name 是否相同，不区分大小写
fn image_named(path: &[u16], exe_name: &str) -> bool {
    let start = path
        .iter()
        .rposition(|&c| c == u16::from(b'\\'))
        .map_or(0, |i| i + 1);
    same_name(&path[start..], exe_name)
}

/// 进程快照里 exe 名相同（不区分大小写）的全部 pid，按快照顺序；拍快照失败时为空
fn process_ids(exe_name: &str) -> Vec<u32> {
    let snapshot = match process_snapshot() {
        Ok(snapshot) => snapshot,
        Err(e) => {
            log::warn!("拍进程快照失败：{e}");
            return Vec::new();
        }
    };
    let mut entry = PROCESSENTRY32W {
        dwSize: size_of::<PROCESSENTRY32W>() as u32,
        ..Default::default()
    };
    let mut pids = Vec::new();
    // SAFETY: snapshot 是有效的进程快照句柄；entry.dwSize 已设为结构体大小
    let mut more = unsafe { Process32FirstW(snapshot.as_raw_handle(), &mut entry) } != FALSE;
    while more {
        if same_name(&entry.szExeFile, exe_name) {
            pids.push(entry.th32ProcessID);
        }
        // SAFETY: 同上
        more = unsafe { Process32NextW(snapshot.as_raw_handle(), &mut entry) } != FALSE;
    }
    // 正常列完时最后一次调用报 ERROR_NO_MORE_FILES；别的错误说明列表不全
    let end = io::Error::last_os_error();
    if end.raw_os_error() != Some(ERROR_NO_MORE_FILES as i32) {
        log::warn!("列举进程时出错，结果可能不全：{end}");
    }
    pids
}

fn process_snapshot() -> io::Result<OwnedHandle> {
    // SAFETY: 不涉及指针
    let raw = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    // SAFETY: 失败时为 INVALID_HANDLE_VALUE，成功时是归调用方所有的新句柄
    let handle = unsafe { HandleOrInvalid::from_raw_handle(raw) };
    OwnedHandle::try_from(handle).map_err(|_| io::Error::last_os_error())
}

/// UTF-16 的 exe 名（有 NUL 时只看 NUL 之前）与 name 是否相同，不区分大小写
fn same_name(wide: &[u16], name: &str) -> bool {
    let len = wide.iter().position(|&c| c == 0).unwrap_or(wide.len());
    char::decode_utf16(wide[..len].iter().copied())
        .map(|c| c.unwrap_or(char::REPLACEMENT_CHARACTER))
        .flat_map(char::to_lowercase)
        .eq(name.chars().flat_map(char::to_lowercase))
}

/// EnumWindows 回调的上下文
struct WindowSearch {
    pid: u32,
    found: Option<HWND>,
}

/// pid 拥有的第一个可见顶层窗口
fn visible_window(pid: u32) -> Option<HWND> {
    let mut search = WindowSearch { pid, found: None };
    // SAFETY: 回调只在 EnumWindows 返回前同步调用，期间 search 一直有效且没有别的引用
    unsafe { EnumWindows(Some(check_window), (&raw mut search) as LPARAM) };
    search.found
}

/// 找到就返回 FALSE 停止枚举
unsafe extern "system" fn check_window(hwnd: HWND, lparam: LPARAM) -> BOOL {
    // SAFETY: lparam 是 visible_window 传入的 &mut WindowSearch
    let search = unsafe { &mut *(lparam as *mut WindowSearch) };
    let mut owner = 0u32;
    // SAFETY: hwnd 来自 EnumWindows；owner 可写
    let thread = unsafe { GetWindowThreadProcessId(hwnd, &mut owner) };
    // 返回 0 表示窗口已失效，owner 没被写，不能拿来比
    // SAFETY: hwnd 来自 EnumWindows
    if thread != 0 && owner == search.pid && unsafe { IsWindowVisible(hwnd) } != FALSE {
        search.found = Some(hwnd);
        return FALSE;
    }
    TRUE
}

/// 最小化的先还原，再切到前台；返回切换是否成功。
/// 还原用 ShowWindowAsync 投递给目标线程：同步的 ShowWindow 会等对方处理完，游戏卡住时连带卡住调用方
fn bring_to_front(hwnd: HWND) -> bool {
    // SAFETY: hwnd 只作为句柄值交给系统，窗口已关闭时调用只是失败
    if unsafe { IsIconic(hwnd) } != FALSE {
        // SAFETY: 同上
        unsafe { ShowWindowAsync(hwnd, SW_RESTORE) };
    }
    // SAFETY: 同上
    let switched = unsafe { SetForegroundWindow(hwnd) };
    switched != FALSE
}

/// dir 下修改时间不早于 since 的 *.dmp（扩展名不区分大小写，不进子目录），按路径排序；
/// 目录不存在或读不了时为空
pub fn crash_dumps_since(dir: &Path, since: SystemTime) -> Vec<PathBuf> {
    let entries = match retry::transient(|| fs::read_dir(dir)) {
        Ok(entries) => entries,
        Err(e) => {
            if e.kind() != io::ErrorKind::NotFound {
                log::warn!("读取崩溃转储目录 {} 失败：{e}", dir.display());
            }
            return Vec::new();
        }
    };
    let mut dumps: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| is_dump(path) && modified_since(path, since))
        .collect();
    dumps.sort();
    dumps
}

fn is_dump(path: &Path) -> bool {
    path.extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case(DUMP_EXT))
}

/// 是普通文件且修改时间 >= since。按路径重新取属性：目录项里缓存的时间在 NTFS 上可能滞后
fn modified_since(path: &Path, since: SystemTime) -> bool {
    let check = fs::metadata(path).and_then(|meta| Ok(meta.is_file() && meta.modified()? >= since));
    check.unwrap_or_else(|e| {
        log::warn!("读取 {} 的修改时间失败：{e}", path.display());
        false
    })
}

#[cfg(test)]
mod tests {
    use super::{GAME_EXE, image_named, terminate};

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().collect()
    }

    #[test]
    fn image_name_compares_file_name_only() {
        assert!(image_named(
            &wide(r"C:\Game\nightreign.exe"),
            "NIGHTREIGN.EXE"
        ));
        assert!(image_named(&wide("nightreign.exe"), GAME_EXE));
        // 目录同名、文件名多了前后缀、只有目录：都不算
        assert!(!image_named(
            &wide(r"C:\nightreign.exe\other.exe"),
            GAME_EXE
        ));
        assert!(!image_named(&wide(r"C:\Game\my-nightreign.exe"), GAME_EXE));
        assert!(!image_named(&wide(r"C:\Game\nightreign.exe.bak"), GAME_EXE));
        assert!(!image_named(&wide(r"C:\Game\"), GAME_EXE));
        assert!(!image_named(&[], GAME_EXE));
    }

    #[test]
    fn terminate_never_kills_other_image() {
        // 模拟快照之后 pid 被复用：当前进程（launcher_core-*.exe）顶着 nightreign.exe 的 pid。
        // 核对不上就不结束；要是结束了，这个测试进程当场就没了
        assert!(!terminate(std::process::id(), GAME_EXE));
        // 打不开的 pid
        assert!(!terminate(u32::MAX, GAME_EXE));
    }
}
