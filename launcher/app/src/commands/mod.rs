//! 前端接口：命令（参数用 camelCase）和事件名，与前端 api\tauri.ts 一一对应；
//! 这里只做转发，状态在 state、窗口在 window

use serde::Serialize;
use tauri::{AppHandle, Emitter};

pub mod first_run;
pub mod game_settings;
pub mod launch;
pub mod options;
pub mod report;
pub mod saves;
pub mod wiki;

/// 启动进度，负载是 LaunchView
pub const LAUNCH_VIEW: &str = "launch-view";
/// 首页状态，负载是 HomeState
pub const HOME_STATE: &str = "home-state";
/// 前端该不该跑动画，负载 { "visible": bool }
pub const WINDOW_VISIBILITY: &str = "window-visibility";
/// 启动进行中关窗口，交前端确认；无负载
pub const CLOSE_REQUESTED: &str = "close-requested";
/// 深夜与联机的设置变了（写回完成或磁盘被手改），负载是 GameSettings
pub const GAME_SETTINGS: &str = "game-settings";
/// 配置写回失败，负载是原因
pub const CONFIG_WRITE_FAILED: &str = "config-write-failed";
/// 托盘点了「整理问题记录」；无负载
pub const OPEN_REPORT: &str = "open-report";

/// 发事件给前端；失败只记日志
pub fn emit<P: Serialize + Clone>(app: &AppHandle, event: &str, payload: P) {
    if let Err(e) = app.emit(event, payload) {
        log::warn!("发送事件 {event} 失败：{e}");
    }
}
