//! 启动流程测试用的假系统：虚拟时钟加一份剧本，sleep 只推进虚拟时钟。
//! 剧本按虚拟时间写 me3 何时以什么码退出、游戏进程何时出现、何时出窗口、何时以什么码退出；
//! 另有常用剧本，以及按假系统时间写崩溃转储的小工具

use std::fs::File;
use std::io;
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, Weak};
use std::time::{Duration, Instant, SystemTime};

use launcher_core::os::{ChildProc, GAME_EXE, LaunchCommand, ME3_EXE, Os};

pub const ME3_PID: u32 = 100;
pub const GAME_PID: u32 = 200;
/// Script::already_running 里的进程都用这个 pid
pub const RUNNING_PID: u32 = 300;
/// 被 kill_by_name 结束的进程的退出码
pub const KILLED_CODE: u32 = 1;
/// 虚拟时间超过它就当作启动流程死循环，直接 panic，免得整个测试卡住
const TIME_LIMIT: Duration = Duration::from_secs(3600);

/// 虚拟时间 0 对应的系统时间：整秒（NTFS 存修改时间不会截断），早于真实时间
pub fn epoch() -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(1_793_000_000)
}

/// 秒数（可带小数）：剧本里的时刻、虚拟时间都这么写
pub fn secs(s: f64) -> Duration {
    Duration::from_secs_f64(s)
}

/// 顺利进游戏的剧本：me3 一直在运行，3 秒出游戏进程，10 秒出窗口，60 秒以 code 退出
pub fn game_exits_with(code: u32) -> Script {
    Script {
        game_start: Some(secs(3.0)),
        game_window: Some(secs(10.0)),
        game_exit: Some((secs(60.0), code)),
        ..Script::default()
    }
}

/// dir 里建一个崩溃转储，修改时间按假系统时间给（如 epoch() + 30 秒）。
/// 空文件读不出异常信息，按真崩溃算；要分类的转储见 minidump::write_dump
pub fn dump(dir: &Path, name: &str, modified: SystemTime) {
    File::create(dir.join(name))
        .unwrap()
        .set_modified(modified)
        .unwrap();
}

/// 时刻都是虚拟时间，建 FakeOs 时为 0（spawn 之前不耗时的话，也就是从唤起 me3 算起）
#[derive(Clone, Debug, Default)]
pub struct Script {
    /// me3 何时以什么码退出；None 为一直在运行
    pub me3_exit: Option<(Duration, u32)>,
    /// 游戏进程何时出现
    pub game_start: Option<Duration>,
    /// 游戏窗口何时出现
    pub game_window: Option<Duration>,
    /// 游戏进程何时以什么码退出
    pub game_exit: Option<(Duration, u32)>,
    /// open_process 打不开游戏进程
    pub game_unopenable: bool,
    /// 在这些虚拟时刻拍进程快照失败：find_process 什么都找不到，进程其实照常运行
    pub snapshot_misses: Vec<Duration>,
    /// spawn 以这个错误失败
    pub spawn_error: Option<io::ErrorKind>,
    /// 与剧本无关、一直在运行的进程名（如 steam.exe）：find_process 交出 RUNNING_PID，kill_by_name 结束不了
    pub already_running: Vec<&'static str>,
}

pub struct FakeOs {
    /// 虚拟时钟，只有 sleep 推进它
    clock: Mutex<Duration>,
    script: Mutex<Script>,
    /// 指向自己：spawn / open_process 交出的假进程要看时钟和剧本
    me: Weak<FakeOs>,
    /// 虚拟时间 0 对应的 Instant
    base: Instant,
    spawned: Mutex<Vec<LaunchCommand>>,
    /// 依次被结束的进程名和结束的虚拟时刻
    killed: Mutex<Vec<(String, Duration)>>,
    opens: AtomicU32,
}

impl FakeOs {
    pub fn new(script: Script) -> Arc<FakeOs> {
        Arc::new_cyclic(|me| FakeOs {
            clock: Mutex::new(Duration::ZERO),
            script: Mutex::new(script),
            me: me.clone(),
            base: Instant::now(),
            spawned: Mutex::default(),
            killed: Mutex::default(),
            opens: AtomicU32::new(0),
        })
    }

    /// 当前虚拟时间
    pub fn elapsed(&self) -> Duration {
        *self.clock.lock().unwrap()
    }

    /// spawn 收到的全部命令（含失败的）
    pub fn spawned(&self) -> Vec<LaunchCommand> {
        self.spawned.lock().unwrap().clone()
    }

    /// 依次被 kill_by_name 的进程名
    pub fn killed(&self) -> Vec<String> {
        self.killed
            .lock()
            .unwrap()
            .iter()
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// open_process 被调用的次数
    pub fn opens(&self) -> u32 {
        self.opens.load(Ordering::SeqCst)
    }

    fn me3_exit(&self) -> Option<u32> {
        let scripted = self.script.lock().unwrap().me3_exit;
        self.exit_code(ME3_EXE, scripted)
    }

    fn me3_running(&self) -> bool {
        let spawn_failed = self.script.lock().unwrap().spawn_error.is_some();
        let spawned = !self.spawned.lock().unwrap().is_empty();
        spawned && !spawn_failed && self.me3_exit().is_none()
    }

    fn game_exit(&self) -> Option<u32> {
        let scripted = self.script.lock().unwrap().game_exit;
        self.exit_code(GAME_EXE, scripted)
    }

    /// 游戏进程已出现、还没退出
    fn game_alive(&self) -> bool {
        let start = self.script.lock().unwrap().game_start;
        start.is_some_and(|at| at <= self.elapsed()) && self.game_exit().is_none()
    }

    /// 剧本里的退出和被结束，取已到时且先发生的那个；还在运行为 None
    fn exit_code(&self, name: &str, scripted: Option<(Duration, u32)>) -> Option<u32> {
        let now = self.elapsed();
        let killed = self.kill_time(name).map(|at| (at, KILLED_CODE));
        [scripted, killed]
            .into_iter()
            .flatten()
            .filter(|&(at, _)| at <= now)
            .min_by_key(|&(at, _)| at)
            .map(|(_, code)| code)
    }

    fn kill_time(&self, name: &str) -> Option<Duration> {
        let killed = self.killed.lock().unwrap();
        killed
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, at)| *at)
    }

    fn child(&self, pid: u32) -> Box<dyn ChildProc> {
        let os = self.me.upgrade().expect("FakeOs 要用 FakeOs::new 建");
        Box::new(FakeChild { os, pid })
    }
}

impl Os for FakeOs {
    fn now(&self) -> Instant {
        self.base + self.elapsed()
    }

    fn system_time(&self) -> SystemTime {
        epoch() + self.elapsed()
    }

    fn sleep(&self, d: Duration) {
        let mut clock = self.clock.lock().unwrap();
        *clock += d;
        assert!(
            *clock <= TIME_LIMIT,
            "虚拟时间超过 {TIME_LIMIT:?}：启动流程可能死循环"
        );
    }

    fn find_process(&self, exe_name: &str) -> Option<u32> {
        let now = self.elapsed();
        let (missed, running) = {
            let script = self.script.lock().unwrap();
            let running = script
                .already_running
                .iter()
                .any(|name| name.eq_ignore_ascii_case(exe_name));
            (script.snapshot_misses.contains(&now), running)
        };
        // 快照失败时什么都找不到，常驻的进程也一样
        if missed {
            return None;
        }
        if running {
            return Some(RUNNING_PID);
        }
        let found = exe_name.eq_ignore_ascii_case(GAME_EXE) && self.game_alive();
        found.then_some(GAME_PID)
    }

    fn has_visible_window(&self, pid: u32) -> bool {
        let window = self.script.lock().unwrap().game_window;
        pid == GAME_PID && self.game_alive() && window.is_some_and(|at| at <= self.elapsed())
    }

    fn focus_process_window(&self, pid: u32) -> bool {
        self.has_visible_window(pid)
    }

    fn spawn(&self, cmd: &LaunchCommand) -> io::Result<Box<dyn ChildProc>> {
        self.spawned.lock().unwrap().push(cmd.clone());
        let error = self.script.lock().unwrap().spawn_error;
        match error {
            Some(kind) => Err(io::Error::from(kind)),
            None => Ok(self.child(ME3_PID)),
        }
    }

    fn open_process(&self, pid: u32) -> Option<Box<dyn ChildProc>> {
        self.opens.fetch_add(1, Ordering::SeqCst);
        let unopenable = self.script.lock().unwrap().game_unopenable;
        (pid == GAME_PID && !unopenable && self.game_alive()).then(|| self.child(GAME_PID))
    }

    /// 结束后进程就不在了，退出码为 KILLED_CODE；me3-launcher 不在剧本里，总是 0 个
    fn kill_by_name(&self, exe_name: &str) -> usize {
        let alive = if exe_name.eq_ignore_ascii_case(ME3_EXE) {
            self.me3_running()
        } else {
            self.find_process(exe_name).is_some()
        };
        self.killed
            .lock()
            .unwrap()
            .push((exe_name.to_owned(), self.elapsed()));
        usize::from(alive)
    }
}

/// spawn / open_process 交出的假进程，按剧本和虚拟时钟回答退出码
struct FakeChild {
    os: Arc<FakeOs>,
    pid: u32,
}

impl ChildProc for FakeChild {
    fn pid(&self) -> u32 {
        self.pid
    }

    fn try_exit_code(&mut self) -> Option<u32> {
        if self.pid == ME3_PID {
            self.os.me3_exit()
        } else {
            self.os.game_exit()
        }
    }
}
