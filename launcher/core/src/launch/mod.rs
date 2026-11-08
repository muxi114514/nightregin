//! 启动流程：预检 → 备份 → 写配置 → 唤起 me3，等游戏窗口出现，再等游戏退出并判定是否异常。
//! 进度以 LaunchView 交给界面；进程和时钟都经 Os 调用，测试时换成假的

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime};

use serde::Serialize;

use crate::backup::BackupOutcome;
use crate::dumpinfo::DumpSummary;
use crate::os::{LaunchCommand, Os};
use crate::preflight::Issue;

mod session;

use self::session::Session;

/// 轮询间隔
pub const POLL: Duration = Duration::from_millis(500);
/// me3 以 0 退出后，等游戏进程出现的上限
pub const NO_GAME_AFTER_ME3_EXIT: Duration = Duration::from_secs(15);
/// 点开始后过了这么久还没进游戏，界面提示「游戏仍在载入」
pub const SLOW_AFTER: Duration = Duration::from_secs(120);
/// 游戏正常退出时的退出码
pub const NORMAL_EXIT_CODES: &[u32] = &[0]; // 正常退出带的 0xC0000005 由转储分类放过，见 judge

/// 启动的四步
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Step {
    Preflight,
    Backup,
    LoadConfig,
    Wake,
}

/// 启动失败的原因
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum FailReason {
    /// 预检发现的阻断问题
    Preflight {
        issue: Issue,
    },
    /// 存档正被别的程序占用
    SaveBusy,
    /// 确定不了账号；accounts 是候选的 SteamID64
    AccountUnknown {
        accounts: Vec<String>,
    },
    Backup {
        message: String,
    },
    ConfigWrite {
        message: String,
    },
    SpawnFailed {
        message: String,
    },
    /// me3 以非零码退出，且没见过游戏进程
    Me3Exit {
        code: u32,
    },
    /// me3 以 0 退出后 15 秒仍没有游戏进程
    GameNeverStarted,
    /// 游戏进程在窗口出现前退出；读不到退出码时为 None
    GameExitedEarly {
        code: Option<u32>,
    },
}

/// 交给界面的启动进度
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(
    tag = "phase",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum LaunchView {
    Idle,
    Step {
        step: Step,
    },
    /// 等游戏窗口。slow：已等满 SLOW_AFTER；game_loading：游戏进程已在、还没有窗口
    Waking {
        slow: bool,
        game_loading: bool,
    },
    Running,
    Failed {
        step: Step,
        reason: FailReason,
    },
    Cancelled,
    Finished {
        abnormal: bool,
    },
}

/// 唤起阶段一次轮询看到的情况
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Observation {
    /// 还在运行时为 None
    pub me3_exit: Option<u32>,
    /// 盯着的游戏进程；没有句柄时连续几轮找不到就为 None
    pub game_pid: Option<u32>,
    pub game_window: bool,
    /// 有句柄时才读得到
    pub game_exit: Option<u32>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum WakeDecision {
    Wait { slow: bool, game_loading: bool },
    Running { pid: u32 },
    Fail(FailReason),
}

/// 唤起阶段的判定：记着开始时刻、第一次看到 me3 退出的时刻、见过的游戏进程
pub struct WakeState {
    started: Instant,
    me3_exited_at: Option<Instant>,
    game_pid: Option<u32>,
}

impl WakeState {
    pub fn new(started: Instant) -> WakeState {
        WakeState {
            started,
            me3_exited_at: None,
            game_pid: None,
        }
    }

    /// 取第一条命中的规则：
    /// 1. 游戏窗口可见 → Running；
    /// 2. 见过的游戏进程已退出（读到退出码，或没有句柄时进程不在了）→ GameExitedEarly；
    /// 3. 从没见过游戏进程时：me3 非零退出 → Me3Exit，以 0 退出满 15 秒 → GameNeverStarted；
    /// 4. 否则 Wait。游戏进程一出现就以它的结果为准，不再看 me3
    pub fn decide(&mut self, obs: &Observation, now: Instant) -> WakeDecision {
        self.game_pid = self.game_pid.or(obs.game_pid);
        if obs.me3_exit.is_some() && self.me3_exited_at.is_none() {
            self.me3_exited_at = Some(now);
        }
        if let Some(pid) = self.game_pid {
            if obs.game_window {
                return WakeDecision::Running { pid };
            }
            if obs.game_exit.is_some() || obs.game_pid.is_none() {
                return WakeDecision::Fail(FailReason::GameExitedEarly {
                    code: obs.game_exit,
                });
            }
        } else if let Some(code) = obs.me3_exit {
            if code != 0 {
                return WakeDecision::Fail(FailReason::Me3Exit { code });
            }
            let since_exit = self
                .me3_exited_at
                .map(|at| now.saturating_duration_since(at));
            if since_exit.is_some_and(|waited| waited >= NO_GAME_AFTER_ME3_EXIT) {
                return WakeDecision::Fail(FailReason::GameNeverStarted);
            }
        }
        WakeDecision::Wait {
            slow: now.saturating_duration_since(self.started) >= SLOW_AFTER,
            game_loading: self.game_pid.is_some(),
        }
    }
}

/// 这次开始游戏的选项
#[derive(Clone, Copy, Debug, Default)]
pub struct LaunchJob {
    /// 不做开始前的自动备份
    pub skip_backup: bool,
    /// 游戏版本不符也照样启动
    pub ignore_version: bool,
}

/// 启动流程用到的外部操作
pub trait LaunchEnv: Send + Sync {
    /// 有阻断问题时交出它
    fn preflight(&self, job: &LaunchJob) -> Result<(), Issue>;
    /// 开始前的自动备份
    fn backup(&self) -> Result<BackupOutcome, FailReason>;
    /// 把配置写进整合包
    fn flush_config(&self) -> Result<(), FailReason>;
    /// 唤起 me3 的命令
    fn command(&self) -> Result<LaunchCommand, FailReason>;
    /// 游戏的崩溃转储目录；不知道时为 None
    fn dumps_dir(&self) -> Option<PathBuf>;
}

/// 一次启动的结局
#[derive(Clone, Debug, PartialEq)]
pub enum LaunchOutcome {
    Cancelled,
    Failed { step: Step, reason: FailReason },
    Finished { abnormal: bool },
}

impl LaunchOutcome {
    fn view(&self) -> LaunchView {
        match self {
            LaunchOutcome::Cancelled => LaunchView::Cancelled,
            LaunchOutcome::Failed { step, reason } => LaunchView::Failed {
                step: *step,
                reason: reason.clone(),
            },
            LaunchOutcome::Finished { abnormal } => LaunchView::Finished {
                abnormal: *abnormal,
            },
        }
    }
}

/// 提前结束（取消或失败）时 Err 里是结局
type Flow<T> = Result<T, LaunchOutcome>;

/// 依次走完四步，再等游戏退出。每一步、等待中的变化和结局都经 emit 交给界面，结局总是最后一个。
/// cancel 在 spawn 之前置位直接取消；之后置位表示「结束 me3 和游戏进程」，先结束三个进程再取消
pub fn run(
    job: &LaunchJob,
    env: &dyn LaunchEnv,
    os: &dyn Os,
    cancel: &AtomicBool,
    emit: &mut dyn FnMut(LaunchView),
) -> LaunchOutcome {
    let outcome = match launch(job, env, os, cancel, emit) {
        Ok(abnormal) => LaunchOutcome::Finished { abnormal },
        Err(outcome) => outcome,
    };
    match &outcome {
        LaunchOutcome::Failed { step, reason } => log::warn!("启动失败（{step:?}）：{reason:?}"),
        LaunchOutcome::Cancelled => log::info!("已取消启动"),
        LaunchOutcome::Finished { .. } => {}
    }
    emit(outcome.view());
    outcome
}

/// 交出游戏是否异常退出
fn launch(
    job: &LaunchJob,
    env: &dyn LaunchEnv,
    os: &dyn Os,
    cancel: &AtomicBool,
    emit: &mut dyn FnMut(LaunchView),
) -> Flow<bool> {
    // 「点开始后 120 秒」：仍在载入的计时从进入 run 起算，含前几步的耗时
    let started = os.now();
    let cmd = prepare(job, env, cancel, emit)?;
    // spawn 前最后看一次：这时取消不用结束任何进程
    if is_set(cancel) {
        return Err(LaunchOutcome::Cancelled);
    }
    // 崩溃转储只认唤起之后写的
    let launched_at = os.system_time();
    let mut session = Session::start(os, cancel, &cmd)?;
    session.wake(started, emit)?;
    let code = session.wait_exit()?;
    Ok(judge(code, env.dumps_dir().as_deref(), launched_at))
}

/// 前三步，再发 Step(Wake) 并取启动命令（取不到算 Wake 失败）；每步之前看一次取消
fn prepare(
    job: &LaunchJob,
    env: &dyn LaunchEnv,
    cancel: &AtomicBool,
    emit: &mut dyn FnMut(LaunchView),
) -> Flow<LaunchCommand> {
    enter(Step::Preflight, cancel, emit)?;
    env.preflight(job)
        .map_err(|issue| failed(Step::Preflight, FailReason::Preflight { issue }))?;
    enter(Step::Backup, cancel, emit)?;
    if job.skip_backup {
        log::info!("本次跳过备份");
    } else {
        // 没有存档（NoSave）不算失败
        let outcome = env
            .backup()
            .map_err(|reason| failed(Step::Backup, reason))?;
        log_backup(&outcome);
    }
    enter(Step::LoadConfig, cancel, emit)?;
    env.flush_config()
        .map_err(|reason| failed(Step::LoadConfig, reason))?;
    enter(Step::Wake, cancel, emit)?;
    env.command().map_err(|reason| failed(Step::Wake, reason))
}

/// 已取消就不进这一步，否则发出 Step
fn enter(step: Step, cancel: &AtomicBool, emit: &mut dyn FnMut(LaunchView)) -> Flow<()> {
    if is_set(cancel) {
        return Err(LaunchOutcome::Cancelled);
    }
    emit(LaunchView::Step { step });
    Ok(())
}

/// 启动后有真崩溃的转储，或退出码已知、不在 NORMAL_EXIT_CODES 里且没有退出时的已知崩溃，就算异常。
/// 正常退出也会以 0xC0000005 退出并留下已知崩溃的转储，有它时退出码不算数；退出码未知时只看真崩溃
fn judge(code: Option<u32>, dumps_dir: Option<&Path>, launched_at: SystemTime) -> bool {
    let dumps = DumpSummary::since(dumps_dir, launched_at);
    let bad_code = code.is_some_and(|code| !NORMAL_EXIT_CODES.contains(&code));
    let abnormal = dumps.real_crashes > 0 || (bad_code && dumps.exit_crashes == 0);
    log::info!("判定为{}退出", if abnormal { "异常" } else { "正常" });
    abnormal
}

/// dumps_dir 里有修改时间不早于 since 的真崩溃转储（退出时的已知崩溃不算）；没有目录时为 false
pub fn abnormal_since(dumps_dir: Option<&Path>, since: SystemTime) -> bool {
    DumpSummary::since(dumps_dir, since).real_crashes > 0
}

fn log_backup(outcome: &BackupOutcome) {
    match outcome {
        BackupOutcome::Created(entry) => log::info!("已备份存档：{}", entry.path.display()),
        BackupOutcome::Unchanged(entry) => {
            log::info!("存档与备份 {} 相同，不再新建", entry.path.display());
        }
        BackupOutcome::NoSave => log::info!("未备份：没有存档，或已关闭开始前备份"),
    }
}

fn failed(step: Step, reason: FailReason) -> LaunchOutcome {
    LaunchOutcome::Failed { step, reason }
}

/// 取消标志只是个开关，不随它传递别的数据，Relaxed 就够
fn is_set(cancel: &AtomicBool) -> bool {
    cancel.load(Ordering::Relaxed)
}

/// 日志里的退出码：十进制加十六进制，崩溃码（如 0xc0000005）看十六进制才认得出
fn code_text(code: u32) -> String {
    format!("{code}（{code:#x}）")
}
