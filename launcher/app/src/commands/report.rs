//! 整理问题记录

use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

use chrono::Local;
use launcher_core::gamever::{dotted, read_exe_version};
use launcher_core::preflight::PackState;
use launcher_core::report::sanitize::Identities;
use launcher_core::report::{self, ReportInputs, ReportPreview};
use launcher_core::saves::{account_dirs, nightreign_save_root};
use launcher_core::steam::{read_active_user, steam_id64};
use tauri::{AppHandle, Manager, State};

use super::{HOME_STATE, emit};
use crate::state::AppState;

const UNKNOWN: &str = "未知";

/// 整合包、启动器、游戏、me3 的版本号（取不到时为 None）
pub struct Versions {
    pub pack: Option<String>,
    pub game: Option<String>,
    pub me3: Option<String>,
}

pub fn versions_of(state: &PackState) -> Versions {
    let exe_version = |path: &Path| read_exe_version(path).ok().map(|v| dotted(v.file_version));
    Versions {
        pack: state.manifest.as_ref().and_then(|m| m.version.clone()),
        game: state.game.as_ref().and_then(|g| exe_version(&g.exe())),
        me3: exe_version(&state.pack.me3_exe()),
    }
}

/// 问题记录的全部输入；不碰 AppState，panic 时也能用
pub fn inputs_for(state: &PackState, launcher_version: &str) -> ReportInputs {
    let mut steam_ids: Vec<u64> = nightreign_save_root()
        .as_deref()
        .map_or_else(Vec::new, account_dirs);
    steam_ids.extend(read_active_user().and_then(steam_id64));
    let v = versions_of(state);
    let windows = windows_version::OsVersion::current();
    let versions = format!(
        "整合包：{}\r\n启动器：{launcher_version}\r\n游戏文件版本：{}\r\nme3：{}\r\nWindows：{}.{}.{}\r\n",
        v.pack.as_deref().unwrap_or(UNKNOWN),
        v.game.as_deref().unwrap_or(UNKNOWN),
        v.me3.as_deref().unwrap_or(UNKNOWN),
        windows.major,
        windows.minor,
        windows.build,
    );
    ReportInputs {
        pack: state.pack.clone(),
        game: state.game.clone(),
        me3_log_dir: report::me3_log_dir(),
        desktop: report::desktop_dir().unwrap_or_else(|| state.pack.launcher_dir()),
        identities: Identities::from_system(steam_ids),
        versions,
    }
}

/// 1.5 MB、820 KB
pub fn human_size(bytes: u64) -> String {
    const MB: f64 = 1024.0 * 1024.0;
    let value = bytes as f64;
    if value >= MB {
        format!("{:.1} MB", value / MB)
    } else {
        format!("{:.0} KB", value / 1024.0)
    }
}

fn inputs(app: &AppHandle, state: &AppState) -> ReportInputs {
    inputs_for(&state.pack_state(), &app.package_info().version.to_string())
}

/// 前端收到托盘的「整理问题记录」时回一声
#[tauri::command]
pub fn ack_open_report(state: State<'_, AppState>) {
    state.ack_report();
}

#[tauri::command(async)]
pub fn preview_report(app: AppHandle, state: State<'_, AppState>) -> ReportPreview {
    report::preview(&inputs(&app, &state), SystemTime::now())
}

/// 进行中再调用返回 Err；成功后清掉「上次游戏异常退出」
#[tauri::command(async)]
pub fn create_report(app: AppHandle, state: State<'_, AppState>) -> Result<String, String> {
    let _busy = state.begin_report()?;
    let path = report::create(
        &inputs(&app, &state),
        SystemTime::now(),
        Local::now().naive_local(),
    )
    .map_err(|e| e.to_string())?;
    log::info!("问题记录已生成：{}", path.display());
    let shown = path.display().to_string();
    state.report_done(path);
    if let Some(state) = app.try_state::<AppState>() {
        emit(&app, HOME_STATE, state.home());
    }
    Ok(shown)
}

/// 在资源管理器里选中刚生成的问题记录；只接受 create_report 刚返回的路径
#[tauri::command(async)]
pub fn reveal_path(state: State<'_, AppState>, path: String) -> Result<(), String> {
    let path = PathBuf::from(path);
    if !state.may_reveal(&path) {
        return Err("只能打开刚生成的问题记录".to_owned());
    }
    Command::new("explorer")
        .raw_arg(format!("/select,\"{}\"", path.display()))
        .spawn()
        .map(drop)
        .map_err(|e| format!("打开资源管理器失败：{e}"))
}
