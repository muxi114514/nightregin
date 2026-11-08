//! state.rs 的单元测试

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use launcher_core::config::game::SettingChange;
use launcher_core::launch::{FailReason, LaunchView, Step};
use launcher_core::os::WinOs;
use launcher_core::paths::PackLayout;
use launcher_core::settings::{Settings, SettingsStore};
use tempfile::TempDir;

use super::{ALREADY_LAUNCHING, AppState, lock, write};

fn step(step: Step) -> LaunchView {
    LaunchView::Step { step }
}

/// 整合包和设置文件都在临时目录里，不需要 Tauri
fn temp_state() -> (TempDir, AppState) {
    let tmp = TempDir::new().unwrap();
    let pack = PackLayout {
        root: tmp.path().to_path_buf(),
    };
    let store = SettingsStore::new(pack.settings_file());
    (
        tmp,
        AppState::new(pack, Arc::new(WinOs), store, Settings::default()),
    )
}

/// 切完永夜王马上按 F：防抖还没到期，几率也要是新的
#[test]
fn wiki_odds_see_pending_lords_change() {
    let (tmp, state) = temp_state();
    let mut ini = String::from("[everdarkChances]\nenabled = true\n");
    for depth in 1..=5 {
        ini += &format!("[everdarkChances.depth{depth}]\n");
        for lord in ["Gladius", "Adel", "Gnoster", "Maris", "Libra", "Fulghor", "Caligo", "Harmonia"] {
            ini += &format!("{lord} = 25\n");
        }
    }
    std::fs::create_dir_all(tmp.path().join("dll")).unwrap();
    std::fs::write(tmp.path().join("dll").join("ChanceSetter.ini"), ini).unwrap();
    state.set_game_setting(SettingChange::Lords(vec![6])).unwrap();
    assert_eq!(state.wiki_odds().everdark.unwrap()["Caligo"], [100.0; 5]);
}

/// 任务书 start_game 第 1 步与裁定 E：启动进行中再开始返回「已在启动中」，每个终态之后又能开始
#[test]
fn begin_launch_rejects_reentry_until_terminal() {
    let failed = LaunchView::Failed {
        step: Step::Backup,
        reason: FailReason::SaveBusy,
    };
    for end in [
        failed,
        LaunchView::Cancelled,
        LaunchView::Finished { abnormal: false },
    ] {
        let (_tmp, state) = temp_state();
        let first = state.begin_launch().unwrap();
        assert_eq!(
            state.begin_launch().err().as_deref(),
            Some(ALREADY_LAUNCHING)
        );
        first.record(&end);
        assert!(state.begin_launch().is_ok(), "{end:?}");
    }
}

/// LC-2：退出时取消唤起之前的启动，唤起之后不取消（裁定 D）；两种情况都照常存盘
#[test]
fn quit_cancels_launch_only_before_wake() {
    for (view, cancelled) in [(step(Step::Backup), true), (step(Step::Wake), false)] {
        let (_tmp, state) = temp_state();
        let handle = state.begin_launch().unwrap();
        handle.record(&view);
        state.prepare_quit();
        assert_eq!(handle.cancel.load(Ordering::SeqCst), cancelled, "{view:?}");
        assert!(state.pack.settings_file().is_file(), "{view:?}");
    }
}

/// LC-4：快照在存盘锁里取，写盘也在锁里，所以落盘顺序就是取快照的顺序
#[test]
fn save_snapshots_and_writes_under_save_lock() {
    let (_tmp, state) = temp_state();
    let file = state.pack.settings_file();
    let later = PathBuf::from(r"D:\之后才改的");
    std::thread::scope(|scope| {
        let held = lock(&state.save_lock);
        let saver = scope.spawn(|| state.save_settings());
        // 存盘线程卡在锁上：还没写盘，也还没取快照
        std::thread::sleep(Duration::from_millis(200));
        assert!(!file.exists());
        write(&state.settings).game_dir = Some(later.clone());
        drop(held);
        saver.join().unwrap();
    });
    let saved: Settings = serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
    assert_eq!(saved.game_dir, Some(later));
}

fn step_handle(state: &AppState, view: LaunchView) -> crate::launch_thread::LaunchHandle {
    let handle = state.begin_launch().unwrap();
    handle.record(&view);
    handle
}

#[test]
fn close_confirms_while_waking() {
    for view in [
        step(Step::Preflight),
        step(Step::Wake),
        LaunchView::Waking {
            slow: false,
            game_loading: true,
        },
    ] {
        let (_tmp, state) = temp_state();
        let _h = step_handle(&state, view.clone());
        assert!(state.launch_in_progress(), "{view:?}");
    }
}

#[test]
fn close_not_confirmed_when_running_or_idle() {
    let (_tmp, state) = temp_state();
    assert!(!state.launch_in_progress());
    let _h = step_handle(&state, LaunchView::Running);
    assert!(!state.launch_in_progress());
    let (_tmp, state) = temp_state();
    let _h = step_handle(&state, LaunchView::Finished { abnormal: false });
    assert!(!state.launch_in_progress());
}

#[test]
fn prepare_quit_flushes_pending() {
    let (_tmp, state) = temp_state();
    let nighter = state.pack.nighter_json();
    std::fs::create_dir_all(nighter.parent().unwrap()).unwrap();
    std::fs::write(
        &nighter,
        r#"{"forceUnlockDeepNight":true,"forceDeepNight":{"enable":true,"level":5},"superNightLordList":[0]}"#,
    )
    .unwrap();
    state.config.change(SettingChange::DeepNightLevel(2));
    state.prepare_quit();
    let config = launcher_core::nighter::read_nighter(&nighter).unwrap();
    assert_eq!(config.level, 2);
}

#[test]
fn report_success_clears_abnormal() {
    let (_tmp, state) = temp_state();
    write(&state.settings).last_exit_abnormal = true;
    state.report_done(PathBuf::from(r"D:\a.zip"));
    assert!(!state.settings().last_exit_abnormal);
}

#[test]
fn report_rejects_reentry() {
    let (_tmp, state) = temp_state();
    let guard = state.begin_report().unwrap();
    assert!(state.begin_report().is_err());
    drop(guard);
    assert!(state.begin_report().is_ok());
}

#[test]
fn tray_start_blocked_before_first_run() {
    let (_tmp, state) = temp_state();
    assert!(!state.can_start_from_tray());
    write(&state.settings).first_run_done = true;
    assert!(state.can_start_from_tray());
}

#[test]
fn reveal_only_accepts_last_report() {
    let (_tmp, state) = temp_state();
    let path = PathBuf::from(r"D:\a.zip");
    assert!(!state.may_reveal(&path));
    state.report_done(path.clone());
    assert!(state.may_reveal(&path));
    assert!(!state.may_reveal(std::path::Path::new(r"C:\Windows\notepad.exe")));
}

#[test]
fn lords_picks_saved_only_for_partial_lists() {
    let (_tmp, state) = temp_state();
    state
        .set_game_setting(SettingChange::Lords(vec![3, 1]))
        .unwrap();
    assert_eq!(state.settings().everdark_picks, Some(vec![1, 3]));
    state
        .set_game_setting(SettingChange::Lords(vec![]))
        .unwrap();
    state
        .set_game_setting(SettingChange::Lords(vec![0, 1, 2, 3, 4, 5, 6, 7]))
        .unwrap();
    assert_eq!(state.settings().everdark_picks, Some(vec![1, 3]));
}

#[test]
fn invalid_setting_is_rejected() {
    let (_tmp, state) = temp_state();
    assert!(
        state
            .set_game_setting(SettingChange::DeepNightLevel(9))
            .is_err()
    );
}

#[test]
fn report_ack_is_counted() {
    let (_tmp, state) = temp_state();
    let before = state.report_acks();
    state.ack_report();
    assert_ne!(state.report_acks(), before);
}

#[test]
fn save_op_refused_while_launching() {
    let (_tmp, state) = temp_state();
    let _h = step_handle(&state, step(Step::Backup));
    assert!(state.begin_save_op().is_err());
}

#[test]
fn launch_refused_while_save_op_runs() {
    let (_tmp, state) = temp_state();
    let guard = state.begin_save_op().unwrap();
    assert!(state.begin_launch().is_err());
    drop(guard);
    assert!(state.begin_launch().is_ok());
}

#[test]
fn save_op_allowed_after_launch_ended() {
    let (_tmp, state) = temp_state();
    let _h = step_handle(&state, LaunchView::Finished { abnormal: false });
    assert!(state.begin_save_op().is_ok());
}
