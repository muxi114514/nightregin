//! 启动线程：一次启动的控制柄、跑 launch::run、把进度记账并转发给前端和窗口

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use chrono::NaiveDateTime;
use launcher_core::env::PackEnv;
use launcher_core::exit_check::{apply_view, unix_now_secs};
use launcher_core::launch::{self, LaunchJob, LaunchView, Step};
use launcher_core::os::Os;
use launcher_core::settings::AfterLaunch;
use launcher_core::steam::read_active_user;
use tauri::{AppHandle, Manager};

use crate::commands::{self, HOME_STATE, LAUNCH_VIEW};
use crate::state::{self, AppState, lock};
use crate::window;

/// 一次启动的控制柄，启动线程和命令各持一份
#[derive(Clone)]
pub struct LaunchHandle {
    /// 置位后 launch::run 取消；唤起之后置位会先结束 me3 和游戏进程
    pub(crate) cancel: Arc<AtomicBool>,
    /// 启动线程还没发出终态
    running: Arc<AtomicBool>,
    /// 最近一次发出的进度：Esc 取消、关窗口确认都要看是否已到唤起
    view: Arc<Mutex<LaunchView>>,
}

impl LaunchHandle {
    pub(crate) fn new() -> LaunchHandle {
        LaunchHandle {
            cancel: Arc::new(AtomicBool::new(false)),
            running: Arc::new(AtomicBool::new(true)),
            view: Arc::new(Mutex::new(LaunchView::Idle)),
        }
    }

    pub(crate) fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// 记下发出的进度；终态时让出启动，start_game 可以再来
    pub(crate) fn record(&self, view: &LaunchView) {
        *lock(&self.view) = view.clone();
        if is_terminal(view) {
            self.running.store(false, Ordering::SeqCst);
        }
    }

    /// Esc：还没到唤起才置位取消。判断和置位都在 view 锁里，
    /// 返回 true 时启动线程一定在唤起 me3 之前看到取消
    pub(crate) fn cancel_before_wake(&self) -> bool {
        let view = lock(&self.view);
        let before = self.is_running() && before_wake(&view);
        if before {
            self.cancel.store(true, Ordering::SeqCst);
        }
        before
    }

    /// 「结束 me3 和游戏进程」：启动进行中就置位，run 随后结束三个进程
    pub(crate) fn end_processes(&self) {
        if self.is_running() {
            self.cancel.store(true, Ordering::SeqCst);
        }
    }

    /// 最近一次发出的进度
    pub(crate) fn view(&self) -> LaunchView {
        lock(&self.view).clone()
    }

    #[cfg(test)]
    pub(crate) fn before_wake(&self) -> bool {
        before_wake(&lock(&self.view))
    }

    pub(crate) fn game_running(&self) -> bool {
        self.is_running() && *lock(&self.view) == LaunchView::Running
    }
}

fn before_wake(view: &LaunchView) -> bool {
    match view {
        LaunchView::Idle => true,
        LaunchView::Step { step } => *step != Step::Wake,
        _ => false,
    }
}

fn is_terminal(view: &LaunchView) -> bool {
    matches!(
        view,
        LaunchView::Failed { .. } | LaunchView::Cancelled | LaunchView::Finished { .. }
    )
}

/// 开始游戏：已在启动中返回 Err；否则起线程跑 launch::run，进度逐个交给 on_view
pub fn start_launch(app: &AppHandle, job: LaunchJob) -> Result<(), String> {
    let handle = app.state::<AppState>().begin_launch()?;
    log::info!(
        "开始游戏（跳过备份：{}，忽略版本：{}）",
        job.skip_backup,
        job.ignore_version
    );
    let (thread_app, thread_handle) = (app.clone(), handle.clone());
    let spawned = std::thread::Builder::new()
        .name("launch".into())
        .spawn(move || run_launch(&thread_app, &thread_handle, job));
    spawned.map(drop).map_err(|e| {
        handle.record(&LaunchView::Cancelled);
        log::error!("无法创建启动线程：{e}");
        format!("无法创建启动线程：{e}")
    })
}

/// 启动线程：用当前设置建 PackState 和 PackEnv，跑完整个启动流程
fn run_launch(app: &AppHandle, handle: &LaunchHandle, job: LaunchJob) {
    let _guard = LaunchGuard { app, handle };
    let state = app.state::<AppState>();
    let os: Arc<dyn Os> = state.os();
    let env = PackEnv::new(
        state.pack_state(),
        Arc::clone(&os),
        state.config(),
        read_active_user(),
        local_now,
    );
    let mut emit = |view| on_view(app, handle, view);
    launch::run(&job, &env, &*os, &handle.cancel, &mut emit);
}

/// 每个进度：记账、发给前端，再处理窗口。
/// 非终态先记下（唤起开始后 Esc 不再取消）；终态等写完设置才让出启动，免得和下一次启动的写设置交错
fn on_view(app: &AppHandle, handle: &LaunchHandle, view: LaunchView) {
    let state = app.state::<AppState>();
    let terminal = is_terminal(&view);
    if !terminal {
        handle.record(&view);
    }
    state.update_settings(|s| apply_view(s, &view, unix_now_secs()));
    if terminal {
        handle.record(&view);
    }
    commands::emit(app, LAUNCH_VIEW, &view);
    match &view {
        LaunchView::Running => after_running(app, &state),
        LaunchView::Finished { .. } => {
            window::show_after_end(app, &view);
            commands::emit(app, HOME_STATE, state.home());
        }
        LaunchView::Failed { .. } | LaunchView::Cancelled => window::show_after_end(app, &view),
        _ => {}
    }
}

/// 游戏窗口出现后：Tray 隐入托盘，Keep 不动（两者前端都收到 visible false，暂停动画），Close 保存设置后退出
fn after_running(app: &AppHandle, state: &AppState) {
    match state.after_launch() {
        AfterLaunch::Tray => window::hide_to_tray(app),
        AfterLaunch::Keep => window::sync_visibility(app),
        AfterLaunch::Close => state::quit(app),
    }
}

/// 启动线程没发终态就结束了（panic）：补发 Cancelled，界面回到首页、可以重新开始
struct LaunchGuard<'a> {
    app: &'a AppHandle,
    handle: &'a LaunchHandle,
}

impl Drop for LaunchGuard<'_> {
    fn drop(&mut self) {
        if self.handle.is_running() {
            log::error!("启动线程意外结束");
            on_view(self.app, self.handle, LaunchView::Cancelled);
        }
    }
}

/// 备份文件名用本地时间
fn local_now() -> NaiveDateTime {
    chrono::Local::now().naive_local()
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::Ordering;

    use launcher_core::launch::{FailReason, LaunchView, Step};

    use super::LaunchHandle;

    fn step(step: Step) -> LaunchView {
        LaunchView::Step { step }
    }

    /// 裁定 F：唤起之前 Esc 置位取消并返回 true；唤起开始后返回 false、不置位
    #[test]
    fn cancel_launch_only_before_wake() {
        let handle = LaunchHandle::new();
        handle.record(&step(Step::LoadConfig));
        assert!(handle.before_wake());
        assert!(handle.cancel_before_wake());
        assert!(handle.cancel.load(Ordering::SeqCst));

        for view in [
            step(Step::Wake),
            LaunchView::Waking {
                slow: false,
                game_loading: true,
            },
        ] {
            let handle = LaunchHandle::new();
            handle.record(&view);
            assert!(!handle.before_wake(), "{view:?}");
            assert!(!handle.cancel_before_wake(), "{view:?}");
            assert!(!handle.cancel.load(Ordering::SeqCst), "{view:?}");
        }
    }

    /// 裁定 F：「结束 me3 和游戏进程」唤起之后照样置位；启动结束后不再置位
    #[test]
    fn end_processes_cancels_while_running() {
        let handle = LaunchHandle::new();
        handle.record(&LaunchView::Waking {
            slow: true,
            game_loading: false,
        });
        handle.end_processes();
        assert!(handle.cancel.load(Ordering::SeqCst));

        let handle = LaunchHandle::new();
        handle.record(&LaunchView::Cancelled);
        handle.end_processes();
        assert!(!handle.cancel.load(Ordering::SeqCst));
    }

    /// 裁定 E：每个终态都让出启动，游戏也不再算运行中
    #[test]
    fn terminal_views_release_launch() {
        let failed = LaunchView::Failed {
            step: Step::Wake,
            reason: FailReason::GameNeverStarted,
        };
        for view in [
            failed,
            LaunchView::Cancelled,
            LaunchView::Finished { abnormal: true },
        ] {
            let handle = LaunchHandle::new();
            handle.record(&LaunchView::Running);
            assert!(handle.is_running() && handle.game_running());
            handle.record(&view);
            assert!(!handle.is_running(), "{view:?}");
            assert!(!handle.game_running(), "{view:?}");
        }
    }
}
