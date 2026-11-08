//! 启动流程：run 在假系统上的完整流程。假的启动环境 FakeEnv 在本文件，假系统 FakeOs 在 common

mod common;

use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use common::backup::now;
use common::fake_os::{FakeOs, Script, dump, epoch, game_exits_with, secs};
use common::minidump::{AV, exit_crash, game_read_av, write_dump};
use launcher_core::backup::{BackupEntry, BackupKind, BackupOutcome};
use launcher_core::launch::{
    FailReason, LaunchEnv, LaunchJob, LaunchOutcome, LaunchView, Step, run,
};
use launcher_core::os::{GAME_EXE, LaunchCommand, ME3_EXE, ME3_LAUNCHER_EXE, Os};
use launcher_core::preflight::{Issue, IssueKind};
use tempfile::TempDir;

const STEPS: [Step; 4] = [Step::Preflight, Step::Backup, Step::LoadConfig, Step::Wake];
const LOADING: LaunchView = LaunchView::Waking {
    slow: false,
    game_loading: true,
};

/// 一次 run 的结局和发出的全部视图
type Run = (LaunchOutcome, Vec<LaunchView>);

/// 假的启动环境：每一步的结果预先给定
struct FakeEnv {
    preflight: Result<(), Issue>,
    /// preflight() 收到的选项
    preflight_job: Mutex<Option<LaunchJob>>,
    /// backup() 交出的结果，只能取一次
    backup: Mutex<Option<Result<BackupOutcome, FailReason>>>,
    backup_calls: AtomicUsize,
    /// backup() 里把 cancel 置位
    cancel_in_backup: bool,
    /// backup() 先在这个假系统上耗掉这么久
    backup_takes: Option<(Arc<FakeOs>, Duration)>,
    flush: Result<(), FailReason>,
    command: Result<LaunchCommand, FailReason>,
    dumps: Option<PathBuf>,
    /// 交给 run 的取消标志
    cancel: AtomicBool,
}

impl FakeEnv {
    /// 每一步都成功，备份新建了一份
    fn new() -> FakeEnv {
        let entry = BackupEntry {
            path: PathBuf::from(r"C:\save\黄金时代备份\NR0000_2026-11-06_214005_auto.co2.bak"),
            kind: BackupKind::Auto,
            taken_at: now(),
            size: 1,
        };
        FakeEnv {
            preflight: Ok(()),
            preflight_job: Mutex::new(None),
            backup: Mutex::new(Some(Ok(BackupOutcome::Created(entry)))),
            backup_calls: AtomicUsize::new(0),
            cancel_in_backup: false,
            backup_takes: None,
            flush: Ok(()),
            command: Ok(me3_command()),
            dumps: None,
            cancel: AtomicBool::new(false),
        }
    }

    fn with_backup(result: Result<BackupOutcome, FailReason>) -> FakeEnv {
        FakeEnv {
            backup: Mutex::new(Some(result)),
            ..FakeEnv::new()
        }
    }

    fn with_dumps(dir: &Path) -> FakeEnv {
        FakeEnv {
            dumps: Some(dir.to_path_buf()),
            ..FakeEnv::new()
        }
    }

    /// backup() 在 os 上耗掉 took
    fn backup_taking(self, os: &Arc<FakeOs>, took: Duration) -> FakeEnv {
        FakeEnv {
            backup_takes: Some((Arc::clone(os), took)),
            ..self
        }
    }

    fn backup_calls(&self) -> usize {
        self.backup_calls.load(Ordering::SeqCst)
    }

    fn preflight_job(&self) -> Option<LaunchJob> {
        *self.preflight_job.lock().unwrap()
    }
}

impl LaunchEnv for FakeEnv {
    fn preflight(&self, job: &LaunchJob) -> Result<(), Issue> {
        *self.preflight_job.lock().unwrap() = Some(*job);
        self.preflight.clone()
    }

    fn backup(&self) -> Result<BackupOutcome, FailReason> {
        self.backup_calls.fetch_add(1, Ordering::SeqCst);
        if self.cancel_in_backup {
            self.cancel.store(true, Ordering::SeqCst);
        }
        if let Some((os, took)) = &self.backup_takes {
            os.sleep(*took);
        }
        self.backup
            .lock()
            .unwrap()
            .take()
            .expect("backup 只应调用一次")
    }

    fn flush_config(&self) -> Result<(), FailReason> {
        self.flush.clone()
    }

    fn command(&self) -> Result<LaunchCommand, FailReason> {
        self.command.clone()
    }

    fn dumps_dir(&self) -> Option<PathBuf> {
        self.dumps.clone()
    }
}

fn me3_command() -> LaunchCommand {
    LaunchCommand {
        program: PathBuf::from(r"D:\pack\bin\me3.exe"),
        args: vec!["launch".into()],
        cwd: PathBuf::from(r"D:\pack"),
    }
}

/// 跑一遍 run；cancel_on 认出的视图一发出就置位 cancel（模拟玩家点结束）。
/// 交出结局和发出的全部视图，每个视图带上发出时距进入 run 的虚拟时间
fn start_timed(
    job: LaunchJob,
    env: &FakeEnv,
    os: &FakeOs,
    cancel_on: impl Fn(&LaunchView) -> bool,
) -> (LaunchOutcome, Vec<(LaunchView, Duration)>) {
    let clicked = os.now();
    let mut views = Vec::new();
    let mut emit = |view: LaunchView| {
        if cancel_on(&view) {
            env.cancel.store(true, Ordering::SeqCst);
        }
        views.push((view, os.now() - clicked));
    };
    let outcome = run(&job, env, os, &env.cancel, &mut emit);
    (outcome, views)
}

/// 同 start_timed，不要时间
fn start(
    job: LaunchJob,
    env: &FakeEnv,
    os: &FakeOs,
    cancel_on: impl Fn(&LaunchView) -> bool,
) -> Run {
    let (outcome, timed) = start_timed(job, env, os, cancel_on);
    (outcome, timed.into_iter().map(|(view, _)| view).collect())
}

fn never(_: &LaunchView) -> bool {
    false
}

/// 前 n 步的 Step 视图，后面接上 rest
fn steps_then(n: usize, rest: impl IntoIterator<Item = LaunchView>) -> Vec<LaunchView> {
    let steps = STEPS[..n].iter().map(|&step| LaunchView::Step { step });
    steps.chain(rest).collect()
}

/// 顺利进游戏、游戏退出时应得的结局和全部视图
fn happy(abnormal: bool) -> Run {
    let views = steps_then(
        4,
        [
            LOADING,
            LaunchView::Running,
            LaunchView::Finished { abnormal },
        ],
    );
    (LaunchOutcome::Finished { abnormal }, views)
}

/// 停在 step 失败时应得的结局和全部视图：到 step 为止的 Step 视图、before，最后是 Failed
fn failed_at(step: Step, reason: FailReason, before: impl IntoIterator<Item = LaunchView>) -> Run {
    let shown = STEPS.iter().position(|&s| s == step).unwrap() + 1;
    let failed = LaunchView::Failed {
        step,
        reason: reason.clone(),
    };
    let views = steps_then(shown, before.into_iter().chain([failed]));
    (LaunchOutcome::Failed { step, reason }, views)
}

/// 被取消时应得的结局和全部视图：前 n 步的 Step 视图、before，最后是 Cancelled
fn cancelled_after(n: usize, before: impl IntoIterator<Item = LaunchView>) -> Run {
    let views = steps_then(n, before.into_iter().chain([LaunchView::Cancelled]));
    (LaunchOutcome::Cancelled, views)
}

#[test]
fn runner_happy_path_emits_in_order() {
    let tmp = TempDir::new().unwrap();
    // 启动前就有的旧转储不算
    dump(tmp.path(), "old.dmp", epoch() - secs(3600.0));
    let env = FakeEnv::with_dumps(tmp.path());
    let os = FakeOs::new(game_exits_with(0));
    let job = LaunchJob {
        ignore_version: true,
        ..Default::default()
    };
    assert_eq!(start(job, &env, &os, never), happy(false));
    // 选项原样交给预检（LaunchJob 没有 PartialEq，逐个比字段）
    let got = env
        .preflight_job()
        .map(|job| (job.skip_backup, job.ignore_version));
    assert_eq!(got, Some((false, true)));
    assert_eq!(env.backup_calls(), 1);
    assert_eq!(os.spawned(), [me3_command()]);
    // 游戏进程只打开一次，之后一直用这个句柄；游戏一退出（60 秒）就结束
    assert_eq!(os.opens(), 1);
    assert_eq!(os.elapsed(), secs(60.0));
    assert!(os.killed().is_empty());
}

#[test]
fn runner_cancel_before_wake() {
    // backup 里置位：之后的步骤都不进行，没有唤起也不结束任何进程
    let env = FakeEnv {
        cancel_in_backup: true,
        ..FakeEnv::new()
    };
    let os = FakeOs::new(game_exits_with(0));
    assert_eq!(
        start(LaunchJob::default(), &env, &os, never),
        cancelled_after(2, [])
    );
    assert!(os.spawned().is_empty());
    assert!(os.killed().is_empty());

    // 发出 Step(Wake) 之后、spawn 之前置位：同样不唤起、不结束
    let os = FakeOs::new(game_exits_with(0));
    let at_wake = |view: &LaunchView| *view == LaunchView::Step { step: Step::Wake };
    let got = start(LaunchJob::default(), &FakeEnv::new(), &os, at_wake);
    assert_eq!(got, cancelled_after(4, []));
    assert!(os.spawned().is_empty());
    assert!(os.killed().is_empty());
}

#[test]
fn runner_end_processes_during_wake() {
    // 游戏进程一直没有窗口；120 秒提示仍在载入时玩家点「结束 me3 和游戏进程」
    let os = FakeOs::new(Script {
        game_start: Some(secs(3.0)),
        ..Script::default()
    });
    let slow = LaunchView::Waking {
        slow: true,
        game_loading: true,
    };
    let got = start(LaunchJob::default(), &FakeEnv::new(), &os, |view| {
        *view == slow
    });
    assert_eq!(got, cancelled_after(4, [LOADING, slow]));
    assert_eq!(os.killed(), [ME3_EXE, ME3_LAUNCHER_EXE, GAME_EXE]);
}

#[test]
fn runner_slow_counts_from_start_click() {
    // 备份耗掉 30 秒才唤起 me3；之后 me3 一直在、游戏进程一直不出现
    let os = FakeOs::new(Script::default());
    let env = FakeEnv::new().backup_taking(&os, secs(30.0));
    let slow = LaunchView::Waking {
        slow: true,
        game_loading: false,
    };
    let (outcome, timed) = start_timed(LaunchJob::default(), &env, &os, |view| *view == slow);
    assert_eq!(outcome, LaunchOutcome::Cancelled);
    let at = |want: LaunchView| {
        timed
            .iter()
            .find(|(view, _)| *view == want)
            .map(|(_, t)| *t)
    };
    assert_eq!(at(LaunchView::Step { step: Step::Wake }), Some(secs(30.0)));
    // 「仍在载入」从点开始（进入 run）起算满 120 秒，不是唤起后再等 120 秒（150 秒）
    assert_eq!(at(slow), Some(secs(120.0)));
}

#[test]
fn runner_backup_failure_and_skip() {
    // 存档被占用：停在 Backup 步，不唤起
    let env = FakeEnv::with_backup(Err(FailReason::SaveBusy));
    let os = FakeOs::new(game_exits_with(0));
    let got = start(LaunchJob::default(), &env, &os, never);
    assert_eq!(got, failed_at(Step::Backup, FailReason::SaveBusy, []));
    assert!(os.spawned().is_empty());

    // 跳过备份：照发 Step(Backup)，只是不调用 backup（调用了就会失败）
    let env = FakeEnv::with_backup(Err(FailReason::SaveBusy));
    let os = FakeOs::new(game_exits_with(0));
    let job = LaunchJob {
        skip_backup: true,
        ..LaunchJob::default()
    };
    assert_eq!(start(job, &env, &os, never), happy(false));
    assert_eq!(env.backup_calls(), 0);
}

#[test]
fn runner_nosave_is_not_failure() {
    let env = FakeEnv::with_backup(Ok(BackupOutcome::NoSave));
    let os = FakeOs::new(game_exits_with(0));
    assert_eq!(start(LaunchJob::default(), &env, &os, never), happy(false));
    assert_eq!(env.backup_calls(), 1);
}

#[test]
fn runner_abnormal_by_exit_code() {
    let os = FakeOs::new(game_exits_with(1));
    assert_eq!(
        start(LaunchJob::default(), &FakeEnv::new(), &os, never),
        happy(true)
    );
}

#[test]
fn runner_abnormal_by_new_dump() {
    let tmp = TempDir::new().unwrap();
    // 唤起之后、窗口出现（10 秒）之前，也就是载入时写下的转储
    dump(tmp.path(), "nightreign.dmp", epoch() + secs(5.0));
    let env = FakeEnv::with_dumps(tmp.path());
    let os = FakeOs::new(game_exits_with(0));
    assert_eq!(start(LaunchJob::default(), &env, &os, never), happy(true));
}

#[test]
fn runner_exit_crash_dump_excuses_av_code() {
    // 实测：从游戏菜单正常退出也以 0xC0000005 退出，并在退出时留下 ntdll 读访问违例的转储
    let tmp = TempDir::new().unwrap();
    write_dump(tmp.path(), "exit.dmp", &exit_crash(), epoch() + secs(59.0));
    let os = FakeOs::new(game_exits_with(AV));
    assert_eq!(
        start(
            LaunchJob::default(),
            &FakeEnv::with_dumps(tmp.path()),
            &os,
            never
        ),
        happy(false)
    );
    // 退出码未知、只有已知崩溃：同样正常
    let os = FakeOs::new(Script {
        game_unopenable: true,
        ..game_exits_with(AV)
    });
    assert_eq!(
        start(
            LaunchJob::default(),
            &FakeEnv::with_dumps(tmp.path()),
            &os,
            never
        ),
        happy(false)
    );
}

#[test]
fn runner_exit_crash_dump_alone_is_normal_for_any_code() {
    // 只有已知崩溃的转储：退出码 0 正常；按规则字面，别的非正常码（如 1）同样被放过
    let tmp = TempDir::new().unwrap();
    write_dump(tmp.path(), "exit.dmp", &exit_crash(), epoch() + secs(59.0));
    for code in [0, 1] {
        let os = FakeOs::new(game_exits_with(code));
        let got = start(
            LaunchJob::default(),
            &FakeEnv::with_dumps(tmp.path()),
            &os,
            never,
        );
        assert_eq!(got, happy(false), "退出码 {code}");
    }
}

#[test]
fn runner_real_crash_dump_is_abnormal_even_with_code_zero() {
    let tmp = TempDir::new().unwrap();
    write_dump(
        tmp.path(),
        "real.dmp",
        &game_read_av(),
        epoch() + secs(30.0),
    );
    let os = FakeOs::new(game_exits_with(0));
    assert_eq!(
        start(
            LaunchJob::default(),
            &FakeEnv::with_dumps(tmp.path()),
            &os,
            never
        ),
        happy(true)
    );
    // 已知崩溃放过的只是退出码，不抵消同一次的真崩溃
    write_dump(tmp.path(), "exit.dmp", &exit_crash(), epoch() + secs(59.0));
    let os = FakeOs::new(game_exits_with(AV));
    assert_eq!(
        start(
            LaunchJob::default(),
            &FakeEnv::with_dumps(tmp.path()),
            &os,
            never
        ),
        happy(true)
    );
}

#[path = "launch_run/more.rs"]
mod more;
