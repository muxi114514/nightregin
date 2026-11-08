//! 应用状态：整合包、设置（读写与存盘）、当前这次启动的控制柄

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard};

use launcher_core::config::everdark::LORD_COUNT;
use launcher_core::config::game::{GameSettings, SettingChange};
use launcher_core::config::service::{ConfigFlush, ConfigService};
use launcher_core::launch::LaunchView;
use launcher_core::os::{GAME_EXE, Os, WinOs};
use launcher_core::paths::PackLayout;
use launcher_core::preflight::{HomeState, PackState, home_state};
use launcher_core::saves::nightreign_save_root;
use launcher_core::settings::{AfterLaunch, Settings, SettingsStore};
use launcher_core::steam::{locate_game_via_steam, read_active_user, steam_id64};
use launcher_core::wiki::odds::{WikiOdds, read_wiki_odds};
use tauri::{AppHandle, Manager};

use crate::launch_thread::LaunchHandle;

/// 已在启动中时 start_game 的错误
const ALREADY_LAUNCHING: &str = "已在启动中";

pub struct AppState {
    pack: PackLayout,
    os: Arc<dyn Os>,
    /// 全程只用这一个 store
    store: SettingsStore,
    settings: RwLock<Settings>,
    /// 存盘时取快照和写盘都在它里面，落盘顺序就是取快照的顺序。
    /// 「不持锁跨慢 IO」的特许例外：只有存盘的线程争它，core 的 IO 锁本来就让它们排队；
    /// 不能在持有 settings 锁时去拿它，持有它时也不发事件
    save_lock: Mutex<()>,
    launch: Mutex<Option<LaunchHandle>>,
    config: Arc<ConfigService>,
    /// 问题记录正在生成
    report_busy: AtomicBool,
    /// 前端收到「整理问题记录」的次数：托盘据此判断界面还活着
    report_acks: AtomicU64,
    /// 正在恢复存档或从原版复制：和开始游戏互斥
    save_op: AtomicBool,
    /// 最近一次生成的问题记录：reveal_path 只认它
    last_report: Mutex<Option<PathBuf>>,
    /// --smoke：不找整合包、不写设置
    smoke: bool,
}

/// 标记位的持有者，析构时放开
pub struct FlagGuard<'a>(&'a AtomicBool);

impl Drop for FlagGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

impl AppState {
    pub fn new(
        pack: PackLayout,
        os: Arc<dyn Os>,
        store: SettingsStore,
        settings: Settings,
    ) -> AppState {
        let config = ConfigService::new(pack.clone());
        AppState {
            pack,
            os,
            store,
            settings: RwLock::new(settings),
            save_lock: Mutex::new(()),
            launch: Mutex::new(None),
            smoke: false,
            config,
            report_busy: AtomicBool::new(false),
            report_acks: AtomicU64::new(0),
            save_op: AtomicBool::new(false),
            last_report: Mutex::new(None),
        }
    }

    /// 冒烟模式不找整合包：pack 只是占位，指向 exe 所在目录；设置用默认值，从不存盘
    pub fn smoke(exe_dir: PathBuf) -> AppState {
        let pack = PackLayout { root: exe_dir };
        let store = SettingsStore::new(pack.settings_file());
        AppState {
            smoke: true,
            ..AppState::new(pack, Arc::new(WinOs), store, Settings::default())
        }
    }

    pub fn pack(&self) -> &PackLayout {
        &self.pack
    }

    pub fn is_smoke(&self) -> bool {
        self.smoke
    }

    /// 按当前设置读一遍整合包现状
    pub fn home(&self) -> HomeState {
        let state = self.pack_state();
        // 游戏运行时存档本来就占着，不用查
        if self.os.find_process(GAME_EXE).is_some() {
            return home_state(&state);
        }
        home_state(&state.probe_save(
            nightreign_save_root().as_deref(),
            read_active_user().and_then(steam_id64),
        ))
    }

    pub fn game_settings(&self) -> GameSettings {
        let mut view = self.config.view();
        view.game_running = self.os.find_process(GAME_EXE).is_some();
        view.lord_picks = self.settings().everdark_picks;
        view
    }

    /// 校验后入队；永夜王 1~6 位的组合记进「已挑选」
    pub fn set_game_setting(&self, change: SettingChange) -> Result<(), String> {
        let change = change.validated()?;
        self.remember_picks(&change);
        self.config.change(change);
        Ok(())
    }

    /// 先把排队的改动写盘：切完永夜王马上看图鉴，几率要是新的
    pub fn wiki_odds(&self) -> WikiOdds {
        if let Err(e) = self.config.flush() {
            log::warn!("图鉴读几率前写回失败：{e}");
        }
        read_wiki_odds(&self.pack)
    }

    fn remember_picks(&self, change: &SettingChange) {
        if let SettingChange::Lords(list) = change
            && !list.is_empty()
            && list.len() < usize::from(LORD_COUNT)
        {
            let picks = list.clone();
            self.update_settings(|s| {
                s.everdark_picks = Some(picks);
                true
            });
        }
    }

    /// 首次运行没走完时，托盘的「开始游戏」只显示窗口
    pub fn can_start_from_tray(&self) -> bool {
        read(&self.settings).first_run_done
    }

    /// 进行中再调用返回 Err
    pub fn begin_report(&self) -> Result<FlagGuard<'_>, String> {
        if self.report_busy.swap(true, Ordering::SeqCst) {
            return Err("正在整理问题记录".to_owned());
        }
        Ok(FlagGuard(&self.report_busy))
    }

    pub fn ack_report(&self) {
        self.report_acks.fetch_add(1, Ordering::SeqCst);
    }

    pub fn report_acks(&self) -> u64 {
        self.report_acks.load(Ordering::SeqCst)
    }

    /// 改存档文件的操作：启动进行中拒绝；持有期间不能开始游戏
    pub fn begin_save_op(&self) -> Result<FlagGuard<'_>, String> {
        if self.save_op.swap(true, Ordering::SeqCst) {
            return Err("正在处理存档，请稍候".to_owned());
        }
        let guard = FlagGuard(&self.save_op);
        if self.launch_in_progress() {
            return Err("正在启动游戏，不能改动存档".to_owned());
        }
        Ok(guard)
    }

    /// 记下路径，清掉「上次游戏异常退出」
    pub fn report_done(&self, path: PathBuf) {
        *lock(&self.last_report) = Some(path);
        self.update_settings(|s| std::mem::take(&mut s.last_exit_abnormal));
    }

    pub fn may_reveal(&self, path: &Path) -> bool {
        lock(&self.last_report).as_deref() == Some(path)
    }

    pub fn set_game_dir(&self, dir: PathBuf) {
        self.update_settings(|s| {
            s.game_dir = Some(dir);
            true
        });
    }

    /// 存盘失败只记日志；冒烟模式不写
    pub fn save_settings(&self) {
        if self.smoke {
            return;
        }
        let _order = lock(&self.save_lock);
        if let Err(e) = self.store.save(&self.settings()) {
            log::error!("保存设置 {} 失败：{e}", self.pack.settings_file().display());
        }
    }

    /// 退出前：唤起之前先取消这次启动（与 Esc 同一操作），免得退出途中启动线程还去唤起 me3；
    /// 唤起之后不取消（裁定 D）。然后把没写回的配置写完、存盘
    pub fn prepare_quit(&self) {
        self.cancel_launch();
        if let Err(e) = self.config.flush() {
            log::error!("退出前写回配置失败：{e}");
        }
        self.config.shutdown();
        self.save_settings();
    }

    pub fn cancel_launch(&self) -> bool {
        self.current_launch()
            .is_some_and(|handle| handle.cancel_before_wake())
    }

    pub fn end_processes(&self) {
        if let Some(handle) = self.current_launch() {
            handle.end_processes();
        }
    }

    /// 把游戏窗口切到前台；没有游戏进程或没切成功时为 false
    pub fn focus_game(&self) -> bool {
        let pid = self.os.find_process(GAME_EXE);
        pid.is_some_and(|pid| self.os.focus_process_window(pid))
    }

    pub fn game_running(&self) -> bool {
        self.current_launch()
            .is_some_and(|handle| handle.game_running())
    }

    /// 启动进行中（到游戏窗口出现之前）：关窗口要确认
    pub fn launch_in_progress(&self) -> bool {
        self.current_launch()
            .is_some_and(|handle| handle.is_running() && handle.view() != LaunchView::Running)
    }

    pub fn launch_view(&self) -> LaunchView {
        self.current_launch()
            .map_or(LaunchView::Idle, |handle| handle.view())
    }

    pub(crate) fn settings(&self) -> Settings {
        read(&self.settings).clone()
    }

    /// 锁内改，锁外存盘（写锁在这一句结束时释放）；change 返回 false 表示没改，不存盘
    pub(crate) fn update_settings(&self, change: impl FnOnce(&mut Settings) -> bool) {
        let changed = change(&mut write(&self.settings));
        if changed {
            self.save_settings();
        }
    }

    pub(crate) fn config(&self) -> Arc<ConfigService> {
        Arc::clone(&self.config)
    }

    pub(crate) fn os(&self) -> Arc<dyn Os> {
        Arc::clone(&self.os)
    }

    pub(crate) fn after_launch(&self) -> AfterLaunch {
        read(&self.settings).after_launch
    }

    /// 读盘，不在任何锁里做
    pub(crate) fn pack_state(&self) -> PackState {
        PackState::load(self.pack.clone(), self.settings(), &locate_game_via_steam)
    }

    pub(crate) fn begin_launch(&self) -> Result<LaunchHandle, String> {
        let mut slot = lock(&self.launch);
        if self.save_op.load(Ordering::SeqCst) {
            return Err("正在处理存档，请稍后再开始游戏".to_owned());
        }
        if slot.as_ref().is_some_and(LaunchHandle::is_running) {
            return Err(ALREADY_LAUNCHING.to_owned());
        }
        let handle = LaunchHandle::new();
        *slot = Some(handle.clone());
        Ok(handle)
    }

    fn current_launch(&self) -> Option<LaunchHandle> {
        lock(&self.launch).clone()
    }
}

/// 保存设置后退出（唤起之前的启动先取消，见 prepare_quit）。
/// 唤起之后启动线程随进程结束，没查到的游戏退出情况留到下次启动补查
pub fn quit(app: &AppHandle) {
    if let Some(state) = app.try_state::<AppState>() {
        state.prepare_quit();
    }
    app.exit(0);
}

/// 持锁的代码只做赋值和拷贝，中毒时数据仍完整，照常取用
pub(crate) fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

fn read<T>(lock: &RwLock<T>) -> RwLockReadGuard<'_, T> {
    lock.read().unwrap_or_else(PoisonError::into_inner)
}

fn write<T>(lock: &RwLock<T>) -> RwLockWriteGuard<'_, T> {
    lock.write().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
