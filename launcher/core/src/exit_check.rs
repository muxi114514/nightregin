//! 游戏退出情况的记账：启动进度写进设置，启动器重开后补查上次没查到的退出

use std::path::Path;
use std::time::{Duration, SystemTime};

use crate::launch::{LaunchView, Step, abnormal_since};
use crate::settings::Settings;

/// 进度对设置的记账（裁定 E）：Wake 记下启动时间并置待查；只有 Finished 写退出结果、清待查。
/// 返回是否改了设置
pub fn apply_view(s: &mut Settings, view: &LaunchView, now_secs: i64) -> bool {
    match view {
        LaunchView::Step { step: Step::Wake } => {
            s.last_launch_at = Some(now_secs);
            s.exit_check_pending = true;
            true
        }
        LaunchView::Finished { abnormal } => {
            s.last_exit_abnormal = *abnormal;
            s.exit_check_pending = false;
            true
        }
        _ => false,
    }
}

/// 当前时间的 Unix 秒：last_launch_at 的单位，补查时按秒读回
pub fn unix_now_secs() -> i64 {
    chrono::Utc::now().timestamp()
}

/// 补查的判定：没有待查标记、或上次的启动还在运行时不动设置，返回 false；
/// 否则按 last_launch_at（Unix 秒）之后 dumps 里有没有新的真崩溃转储（退出时的已知崩溃不算）写回
/// last_exit_abnormal，清掉待查标记。
/// 没有启动时间或为负数就无从比较，按正常处理
pub fn settle(s: &mut Settings, still_running: bool, dumps: Option<&Path>) -> bool {
    if !s.exit_check_pending || still_running {
        return false;
    }
    let since = s.last_launch_at.and_then(unix_time);
    s.last_exit_abnormal = since.is_some_and(|since| abnormal_since(dumps, since));
    s.exit_check_pending = false;
    true
}

/// Unix 秒转 SystemTime；负数为 None
fn unix_time(secs: i64) -> Option<SystemTime> {
    let secs = u64::try_from(secs).ok()?;
    SystemTime::UNIX_EPOCH.checked_add(Duration::from_secs(secs))
}
