//! 退出补查链：启动进度的记账（apply_view）与重开后的补判（settle）

use std::fs::{self, File};
use std::path::Path;
use std::time::{Duration, SystemTime};

use launcher_core::exit_check::{apply_view, settle, unix_now_secs};
use launcher_core::launch::{FailReason, LaunchView, Step};
use launcher_core::settings::Settings;
use tempfile::TempDir;

/// 某次唤起的时刻（Unix 秒）
const T: i64 = 1_762_400_000;

fn wake() -> LaunchView {
    LaunchView::Step { step: Step::Wake }
}

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

#[test]
fn apply_view_wake_marks_pending_at_launch_time() {
    let mut s = Settings::default();
    assert!(apply_view(&mut s, &wake(), T));
    assert_eq!((s.last_launch_at, s.exit_check_pending), (Some(T), true));
}

#[test]
fn apply_view_finished_records_exit_and_clears_pending() {
    for abnormal in [true, false] {
        let mut s = Settings {
            last_exit_abnormal: !abnormal,
            ..pending(Some(T))
        };
        assert!(apply_view(
            &mut s,
            &LaunchView::Finished { abnormal },
            T + 60
        ));
        assert_eq!(
            (s.last_exit_abnormal, s.exit_check_pending),
            (abnormal, false)
        );
        assert_eq!(s.last_launch_at, Some(T));
    }
}

/// 裁定 E：Failed、Cancelled（以及其余进度）不动设置
#[test]
fn apply_view_failed_and_cancelled_change_nothing() {
    let reason = FailReason::GameExitedEarly { code: None };
    let failed = LaunchView::Failed {
        step: Step::Wake,
        reason,
    };
    for view in [failed, LaunchView::Cancelled, LaunchView::Running] {
        let mut s = Settings {
            last_exit_abnormal: true,
            ..pending(Some(T))
        };
        let before = s.clone();
        assert!(!apply_view(&mut s, &view, T + 60), "{view:?}");
        assert_eq!(s, before, "{view:?}");
    }
}

#[test]
fn settle_newer_dump_is_abnormal() {
    let tmp = TempDir::new().unwrap();
    dump(tmp.path(), unix(T + 30));
    let mut s = pending(Some(T));
    assert!(settle(&mut s, false, Some(tmp.path())));
    assert_eq!((s.last_exit_abnormal, s.exit_check_pending), (true, false));
}

#[test]
fn settle_while_still_running_changes_nothing() {
    let tmp = TempDir::new().unwrap();
    dump(tmp.path(), unix(T + 30));
    let mut s = pending(Some(T));
    assert!(!settle(&mut s, true, Some(tmp.path())));
    assert_eq!(s, pending(Some(T)));
}

/// 没有启动时间或为负数：无从比较，即使有转储也按正常处理
#[test]
fn settle_without_valid_launch_time_is_normal() {
    let tmp = TempDir::new().unwrap();
    dump(tmp.path(), SystemTime::now());
    for at in [None, Some(-1)] {
        let mut s = Settings {
            last_exit_abnormal: true,
            ..pending(at)
        };
        assert!(settle(&mut s, false, Some(tmp.path())), "{at:?}");
        assert_eq!(
            (s.last_exit_abnormal, s.exit_check_pending),
            (false, false),
            "{at:?}"
        );
    }
}

/// 写入方（on_view 经 unix_now_secs）与读回方（settle）都按 Unix 秒：
/// 唤起之后写的转储算这次的，唤起前一小时的不算
#[test]
fn exit_check_round_trip_in_unix_seconds() {
    let tmp = TempDir::new().unwrap();
    let (fresh, stale) = (tmp.path().join("fresh"), tmp.path().join("stale"));
    let mut launched = Settings::default();
    assert!(apply_view(&mut launched, &wake(), unix_now_secs()));
    let at = launched.last_launch_at.unwrap();
    dump(&fresh, SystemTime::now());
    dump(&stale, unix(at - 3600));
    let mut s = launched.clone();
    assert!(settle(&mut s, false, Some(&fresh)) && s.last_exit_abnormal);
    let mut s = launched;
    assert!(settle(&mut s, false, Some(&stale)) && !s.last_exit_abnormal);
}
