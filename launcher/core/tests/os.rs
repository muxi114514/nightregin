//! 进程与窗口：me3 启动命令、唤起进程读退出码、按 exe 名找 / 结束进程、可见窗口、崩溃转储

use std::env;
use std::ffi::OsString;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::{Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

use launcher_core::os::{
    ChildProc, GAME_EXE, LaunchCommand, ME3_EXE, ME3_LAUNCHER_EXE, Os, STEAM_EXE, WinOs,
    crash_dumps_since, me3_command,
};
use launcher_core::paths::{GameDir, PackLayout};
use tempfile::TempDir;
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, SW_SHOWNA, ShowWindow, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    WS_POPUP,
};

/// 等子进程退出的上限：PowerShell 5.1 冷启动可能要好几秒
const EXIT_TIMEOUT: Duration = Duration::from_secs(30);
const NO_SUCH_EXE: &str = "nrga-no-such-process.exe";

/// 按名字找当前测试 exe 的测试，与会起同名子进程的测试不能同时跑
static SAME_NAME: Mutex<()> = Mutex::new(());

fn command(program: PathBuf, args: &[&str], cwd: &Path) -> LaunchCommand {
    LaunchCommand {
        program,
        args: args.iter().map(OsString::from).collect(),
        cwd: cwd.to_path_buf(),
    }
}

/// %SystemRoot%\System32 下的程序，不靠 PATH 查找
fn system32(rel: &str) -> PathBuf {
    PathBuf::from(env::var_os("SystemRoot").expect("SystemRoot 未设置"))
        .join("System32")
        .join(rel)
}

/// 每 50ms 读一次退出码；EXIT_TIMEOUT 内没退出为 None
fn wait_exit(child: &mut dyn ChildProc) -> Option<u32> {
    let deadline = Instant::now() + EXIT_TIMEOUT;
    loop {
        if let Some(code) = child.try_exit_code() {
            return Some(code);
        }
        if Instant::now() >= deadline {
            return None;
        }
        thread::sleep(Duration::from_millis(50));
    }
}

/// 当前测试 exe 的文件名，如 os-0123abcd.exe
fn current_exe_name() -> String {
    let exe = env::current_exe().unwrap();
    exe.file_name().unwrap().to_str().unwrap().to_owned()
}

#[test]
fn me3_command_is_exact() {
    let pack = PackLayout {
        root: r"D:\G R\me3\MMV\me3".into(),
    };
    let game = GameDir {
        root: r"D:\G R".into(),
    };
    let c = me3_command(&pack, &game);
    assert_eq!(c.program, PathBuf::from(r"D:\G R\me3\MMV\me3\bin\me3.exe"));
    let want: Vec<OsString> = [
        "launch",
        "-g",
        "nightreign",
        "-p",
        r"D:\G R\me3\MMV\me3\褪色燃烧.me3",
        "-e",
        r"D:\G R\Game\nightreign.exe",
    ]
    .iter()
    .map(OsString::from)
    .collect();
    assert_eq!(c.args, want);
    assert_eq!(c.cwd, pack.root);
}

#[test]
fn exe_names_match_layout() {
    // 按名字找 / 结束进程用的名字，就是 paths 里拼路径用的文件名
    let pack = PackLayout {
        root: r"D:\pack".into(),
    };
    let game = GameDir {
        root: r"D:\game".into(),
    };
    assert_eq!(pack.me3_exe().file_name().unwrap(), ME3_EXE);
    assert_eq!(game.exe().file_name().unwrap(), GAME_EXE);
    assert_eq!([GAME_EXE, ME3_EXE], ["nightreign.exe", "me3.exe"]);
    assert_eq!(
        [ME3_LAUNCHER_EXE, STEAM_EXE],
        ["me3-launcher.exe", "steam.exe"]
    );
}

#[test]
fn spawn_reports_exit_code() {
    let cmd = command(system32("cmd.exe"), &["/c", "exit", "7"], &env::temp_dir());
    let mut child = WinOs.spawn(&cmd).unwrap();
    assert_ne!(child.pid(), 0);
    assert_eq!(wait_exit(child.as_mut()), Some(7));

    // 程序不存在：返回错误而不是 panic
    let tmp = TempDir::new().unwrap();
    let missing = command(tmp.path().join("no-such.exe"), &[], tmp.path());
    assert_eq!(
        WinOs.spawn(&missing).err().map(|e| e.kind()),
        Some(io::ErrorKind::NotFound)
    );
}

#[test]
fn spawn_keeps_args_with_spaces() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().join("a b 中文");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("w.ps1"),
        "Set-Content -LiteralPath $args[0] ok\r\n",
    )
    .unwrap();
    let out = dir.join("out.txt");
    // 脚本按相对路径给：工作目录没设成 dir 就找不到它
    let powershell = system32(r"WindowsPowerShell\v1.0\powershell.exe");
    let args = ["-NoProfile", "-ExecutionPolicy", "Bypass", "-File", "w.ps1"];
    let mut cmd = command(powershell, &args, &dir);
    cmd.args.push(out.clone().into_os_string());
    let mut child = WinOs.spawn(&cmd).unwrap();
    assert_eq!(wait_exit(child.as_mut()), Some(0));
    assert_eq!(fs::read_to_string(&out).unwrap().trim_end(), "ok");
}

#[test]
fn find_process_finds_current_test_binary() {
    let _same_name = SAME_NAME.lock().unwrap_or_else(PoisonError::into_inner);
    let name = current_exe_name();
    let me = std::process::id();
    assert_eq!(WinOs.find_process(&name), Some(me));
    // 不区分大小写
    assert_eq!(WinOs.find_process(&name.to_uppercase()), Some(me));
    assert_eq!(WinOs.find_process(NO_SUCH_EXE), None);
}

#[test]
fn crash_dumps_since_filters_by_time_and_ext() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().join("reports");
    fs::create_dir_all(&dir).unwrap();
    // 整秒：NTFS 按 100ns 存修改时间，不会被截断
    let since = SystemTime::UNIX_EPOCH + Duration::from_secs(1_793_000_000);
    let tick = Duration::from_nanos(100);
    let later = since + Duration::from_secs(60);
    let make = |name: &str, modified: SystemTime| {
        let path = dir.join(name);
        File::create(&path).unwrap().set_modified(modified).unwrap();
        path
    };
    make("before.dmp", since - tick);
    let at = make("at.dmp", since);
    let upper = make("LATER.DMP", later);
    make("later.txt", later);
    make("later.dmp.bak", later);
    make("dmp", later);

    let mut got = crash_dumps_since(&dir, since);
    got.sort();
    let mut want = vec![at, upper];
    want.sort();
    assert_eq!(got, want);
    // 目录不存在：空列表
    assert_eq!(
        crash_dumps_since(&tmp.path().join("missing"), since),
        Vec::<PathBuf>::new()
    );
}

#[test]
fn open_process_waits_for_real_exit() {
    // 正在运行的进程（自己）：没有退出码
    let me = std::process::id();
    let mut running = WinOs.open_process(me).expect("应能打开当前进程");
    assert_eq!(running.pid(), me);
    assert_eq!(running.try_exit_code(), None);

    // 以 259（与 STILL_ACTIVE 同值）退出的进程照样读到退出码
    let cmd = command(
        system32("cmd.exe"),
        &["/c", "exit", "259"],
        &env::temp_dir(),
    );
    let mut spawned = WinOs.spawn(&cmd).unwrap();
    // spawned 一直持有进程句柄，进程退出后 pid 也不会被复用
    let mut opened = WinOs
        .open_process(spawned.pid())
        .expect("应能打开刚起的进程");
    assert_eq!(opened.pid(), spawned.pid());
    assert_eq!(wait_exit(opened.as_mut()), Some(259));
    assert_eq!(wait_exit(spawned.as_mut()), Some(259));

    // 内核查 pid 时忽略低 2 位，u32::MAX 按 0xFFFFFFFC 查，远超任何已分配的编号
    assert!(WinOs.open_process(u32::MAX).is_none());
}

/// kill_by_name_spares_current_process 起的同名子进程只跑这一个：睡到被结束。
/// 万一没结束掉，15 秒后自己退出，不会长时间占着 exe 让下次编译链接失败
#[test]
#[ignore = "只给 kill_by_name_spares_current_process 当子进程"]
fn kill_target() {
    thread::sleep(Duration::from_secs(15));
}

/// 会结束所有与本测试 exe 同名的其他进程：这个测试 exe 不能同时跑两份
/// （例如两个终端共用 target 目录同时 cargo test），否则另一份会被当成子进程结束掉
#[test]
fn kill_by_name_spares_current_process() {
    let _same_name = SAME_NAME.lock().unwrap_or_else(PoisonError::into_inner);
    let args = ["kill_target", "--exact", "--ignored"];
    let mut twin = WinOs
        .spawn(&command(
            env::current_exe().unwrap(),
            &args,
            &env::temp_dir(),
        ))
        .unwrap();
    // 名字大小写不同也算；同名的当前进程不结束（结束了这个测试也就中断了）
    assert_eq!(WinOs.kill_by_name(&current_exe_name().to_uppercase()), 1);
    assert!(wait_exit(twin.as_mut()).is_some(), "同名子进程应已结束");
    assert_eq!(WinOs.kill_by_name(NO_SUCH_EXE), 0);
}

#[test]
fn has_visible_window_needs_visible_top_level_window() {
    // 不存在的进程：没有窗口，也切不到前台
    assert!(!WinOs.has_visible_window(u32::MAX));
    assert!(!WinOs.focus_process_window(u32::MAX));

    let me = std::process::id();
    // 控制台窗口可能记在本进程名下，所以只比较建窗口前后
    let before = WinOs.has_visible_window(me);
    let window = TestWindow::hidden();
    assert_eq!(WinOs.has_visible_window(me), before, "隐藏的窗口不算");
    window.show();
    assert!(WinOs.has_visible_window(me));
}

/// 屏幕外 1×1 的工具窗口：不进任务栏，显示时不抢焦点；析构时销毁
struct TestWindow(HWND);

impl TestWindow {
    /// 建好但不显示
    fn hidden() -> TestWindow {
        let class: Vec<u16> = "STATIC".encode_utf16().chain([0]).collect();
        // SAFETY: class 以 NUL 结尾；STATIC 是系统预定义的窗口类，不用注册；其余指针参数允许为空
        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                class.as_ptr(),
                ptr::null(),
                WS_POPUP,
                -32000,
                -32000,
                1,
                1,
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null(),
            )
        };
        assert!(
            !hwnd.is_null(),
            "建窗口失败：{}",
            io::Error::last_os_error()
        );
        TestWindow(hwnd)
    }

    fn show(&self) {
        // SAFETY: 窗口由本线程创建，还没销毁
        unsafe { ShowWindow(self.0, SW_SHOWNA) };
    }
}

impl Drop for TestWindow {
    fn drop(&mut self) {
        // SAFETY: 同上；析构只发生一次
        unsafe { DestroyWindow(self.0) };
    }
}
