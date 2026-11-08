//! 唤起 me3 之后的会话：盯着 me3 和游戏进程，等窗口出现、等退出，取消时结束进程

use std::sync::atomic::AtomicBool;
use std::time::Instant;

use super::{
    FailReason, Flow, LaunchOutcome, LaunchView, Observation, POLL, Step, WakeDecision, WakeState,
    code_text, failed, is_set,
};
use crate::os::{ChildProc, GAME_EXE, LaunchCommand, ME3_EXE, ME3_LAUNCHER_EXE, Os};

/// 唤起之后取消时依次结束的进程
const KILL_ON_CANCEL: [&str; 3] = [ME3_EXE, ME3_LAUNCHER_EXE, GAME_EXE];
/// 没有句柄时，连续这么多轮在进程列表里找不到游戏进程才算它已退出（拍快照偶尔会失败、返回空）
const VANISH_AFTER_MISSES: u32 = 3;

/// 唤起 me3 之后：盯着 me3 和游戏进程，取消时结束三个进程
pub(super) struct Session<'a> {
    os: &'a dyn Os,
    cancel: &'a AtomicBool,
    me3: Box<dyn ChildProc>,
    /// 读到一次就不再读
    me3_exit: Option<u32>,
    game: Option<GameProc>,
}

impl<'a> Session<'a> {
    pub(super) fn start(
        os: &'a dyn Os,
        cancel: &'a AtomicBool,
        cmd: &LaunchCommand,
    ) -> Flow<Session<'a>> {
        // 路径含空格、中文时出问题好查
        log::info!("唤起 me3：{} {:?}", cmd.program.display(), cmd.args);
        let me3 = os.spawn(cmd).map_err(|e| {
            failed(
                Step::Wake,
                FailReason::SpawnFailed {
                    message: e.to_string(),
                },
            )
        })?;
        log::info!("已唤起 me3（pid {}）", me3.pid());
        Ok(Session {
            os,
            cancel,
            me3,
            me3_exit: None,
            game: None,
        })
    }

    /// 等游戏窗口出现，发 Running。Waking 只在 (slow, game_loading) 变了时发；
    /// 初值 (false, false) 即界面还没收到 Waking 时的样子。started 是进入 run 的时刻
    pub(super) fn wake(&mut self, started: Instant, emit: &mut dyn FnMut(LaunchView)) -> Flow<()> {
        let mut state = WakeState::new(started);
        let mut shown = (false, false);
        loop {
            self.check_cancel()?;
            let obs = self.observe();
            match state.decide(&obs, self.os.now()) {
                WakeDecision::Wait { slow, game_loading } => {
                    if (slow, game_loading) != shown {
                        shown = (slow, game_loading);
                        emit(LaunchView::Waking { slow, game_loading });
                    }
                }
                WakeDecision::Running { pid } => {
                    log::info!("游戏窗口已出现（pid {pid}）");
                    emit(LaunchView::Running);
                    return Ok(());
                }
                WakeDecision::Fail(reason) => return Err(failed(Step::Wake, reason)),
            }
            self.os.sleep(POLL);
        }
    }

    /// 运行中：等游戏进程退出，交出退出码（没有句柄时为 None）
    pub(super) fn wait_exit(&mut self) -> Flow<Option<u32>> {
        loop {
            self.check_cancel()?;
            // me3 可能比游戏晚退出，照样记下它的退出码
            self.poll_me3();
            let status = match &mut self.game {
                Some(game) => game.status(self.os),
                // 进得了运行阶段就一定找到过游戏进程；万一没有，按已不在处理
                None => GameStatus::Vanished,
            };
            match status {
                GameStatus::Alive => {}
                GameStatus::Exited(code) => {
                    log::info!("游戏已退出，退出码 {}", code_text(code));
                    return Ok(Some(code));
                }
                GameStatus::Vanished => {
                    log::info!("游戏进程已不在，退出码未知");
                    return Ok(None);
                }
            }
            self.os.sleep(POLL);
        }
    }

    /// 一次轮询：me3 退出了没有；游戏进程第一次找到时打开，之后只看它退出了没有；窗口出来了没有
    fn observe(&mut self) -> Observation {
        let me3_exit = self.poll_me3();
        let (pid, status) = match &mut self.game {
            Some(game) => (game.pid, game.status(self.os)),
            None => match self.os.find_process(GAME_EXE) {
                // 刚找到的进程这一轮按还在算
                Some(pid) => {
                    self.game = Some(GameProc::open(self.os, pid));
                    (pid, GameStatus::Alive)
                }
                None => {
                    return Observation {
                        me3_exit,
                        ..Observation::default()
                    };
                }
            },
        };
        match status {
            GameStatus::Alive => Observation {
                me3_exit,
                game_pid: Some(pid),
                game_window: self.os.has_visible_window(pid),
                game_exit: None,
            },
            GameStatus::Exited(code) => Observation {
                me3_exit,
                game_pid: Some(pid),
                game_window: false,
                game_exit: Some(code),
            },
            GameStatus::Vanished => Observation {
                me3_exit,
                ..Observation::default()
            },
        }
    }

    /// 读到一次后沿用，只在第一次读到时记日志
    fn poll_me3(&mut self) -> Option<u32> {
        if self.me3_exit.is_none() {
            self.me3_exit = self.me3.try_exit_code();
            if let Some(code) = self.me3_exit {
                log::info!("me3 已退出，退出码 {}", code_text(code));
            }
        }
        self.me3_exit
    }

    /// 置位了就依次结束 me3、me3-launcher 和游戏，交出 Cancelled
    fn check_cancel(&self) -> Flow<()> {
        if !is_set(self.cancel) {
            return Ok(());
        }
        let killed: Vec<String> = KILL_ON_CANCEL
            .iter()
            .map(|name| format!("{name} {} 个", self.os.kill_by_name(name)))
            .collect();
        log::info!("结束进程：{}", killed.join("，"));
        Err(LaunchOutcome::Cancelled)
    }
}

/// 盯着的游戏进程：找到时打开一次句柄并一直持有（打开失败每次都记警告，不能每轮重开）；
/// 打不开时改看这个 pid 还在不在
struct GameProc {
    pid: u32,
    handle: Option<Box<dyn ChildProc>>,
    /// 没有句柄时，连续几轮没在进程列表里找到这个 pid
    misses: u32,
}

enum GameStatus {
    Alive,
    /// 有句柄，读到了退出码
    Exited(u32),
    /// 没有句柄，连续 VANISH_AFTER_MISSES 轮找不到这个 pid
    Vanished,
}

impl GameProc {
    fn open(os: &dyn Os, pid: u32) -> GameProc {
        let handle = os.open_process(pid);
        if handle.is_some() {
            log::info!("找到游戏进程（pid {pid}）");
        } else {
            log::info!("找到游戏进程（pid {pid}），打不开句柄，改看进程还在不在");
        }
        GameProc {
            pid,
            handle,
            misses: 0,
        }
    }

    fn status(&mut self, os: &dyn Os) -> GameStatus {
        if let Some(handle) = &mut self.handle {
            return handle
                .try_exit_code()
                .map_or(GameStatus::Alive, GameStatus::Exited);
        }
        // 一次没找到不算，找到就重新计数
        if os.find_process(GAME_EXE) == Some(self.pid) {
            self.misses = 0;
        } else {
            self.misses = self.misses.saturating_add(1);
        }
        if self.misses >= VANISH_AFTER_MISSES {
            GameStatus::Vanished
        } else {
            GameStatus::Alive
        }
    }
}
