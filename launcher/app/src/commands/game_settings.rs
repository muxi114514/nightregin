//! 深夜与联机：读设置、改动入队、一键改正

use launcher_core::config::ConfigFile;
use launcher_core::config::game::{GameSettings, SettingChange};
use tauri::{AppHandle, Manager, State};

use super::{GAME_SETTINGS, HOME_STATE, emit};
use crate::state::AppState;

/// 窗口拿到焦点时查一次磁盘：玩家切出去手改了配置，界面跟着刷新
pub fn recheck_on_focus(app: &AppHandle) {
    let handle = app.clone();
    let spawned = std::thread::Builder::new()
        .name("config-recheck".into())
        .spawn(move || {
            let Some(state) = handle.try_state::<AppState>() else {
                return;
            };
            if state.config().changed_on_disk() {
                emit(&handle, GAME_SETTINGS, state.game_settings());
                emit(&handle, HOME_STATE, state.home());
            }
        });
    if let Err(e) = spawned {
        log::warn!("无法创建配置检查线程：{e}");
    }
}

#[tauri::command(async)]
pub fn get_game_settings(state: State<'_, AppState>) -> GameSettings {
    state.game_settings()
}

/// 入队，300ms 后写回；取值不合法时 Err
#[tauri::command(async)]
pub fn set_game_setting(state: State<'_, AppState>, change: SettingChange) -> Result<(), String> {
    state.set_game_setting(change)
}

#[tauri::command(async)]
pub fn restore_config_default(
    app: AppHandle,
    state: State<'_, AppState>,
    file: ConfigFile,
) -> Result<GameSettings, String> {
    state
        .config()
        .restore_default(file)
        .map_err(|e| e.to_string())?;
    emit(&app, HOME_STATE, state.home());
    Ok(state.game_settings())
}

#[tauri::command(async)]
pub fn fix_everdark(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    state.config().fix_everdark().map_err(|e| e.to_string())?;
    emit(&app, HOME_STATE, state.home());
    Ok(())
}

#[tauri::command(async)]
pub fn align_suffix(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    state.config().align_suffix().map_err(|e| e.to_string())?;
    emit(&app, HOME_STATE, state.home());
    Ok(())
}
