//! 设置页：启动器自己的选项、关于、桌面快捷方式

use launcher_core::report;
use launcher_core::settings::AfterLaunch;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};

use super::report::versions_of;
use super::{HOME_STATE, emit};
use crate::shortcut::create_desktop_shortcut;
use crate::state::AppState;

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(tag = "key", content = "value", rename_all = "camelCase")]
pub enum OptionChange {
    BackupBeforeStart(bool),
    AfterLaunch(AfterLaunch),
    LowEffects(bool),
    DevMode(bool),
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OptionsView {
    backup_before_start: bool,
    after_launch: AfterLaunch,
    low_effects: bool,
    dev_mode: bool,
    game_dir: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AboutView {
    pack_version: Option<String>,
    launcher_version: String,
    me3_version: Option<String>,
    game_version: Option<String>,
    /// 计划 4 接
    update_source: Option<String>,
}

#[tauri::command(async)]
pub fn get_options(state: State<'_, AppState>) -> OptionsView {
    let settings = state.settings();
    OptionsView {
        backup_before_start: settings.backup_before_start,
        after_launch: settings.after_launch,
        low_effects: settings.low_effects,
        dev_mode: settings.dev_mode,
        game_dir: state
            .pack_state()
            .game
            .map(|game| game.root.display().to_string()),
    }
}

#[tauri::command(async)]
pub fn set_option(app: AppHandle, state: State<'_, AppState>, change: OptionChange) {
    state.update_settings(|s| {
        match change {
            OptionChange::BackupBeforeStart(v) => s.backup_before_start = v,
            OptionChange::AfterLaunch(v) => s.after_launch = v,
            OptionChange::LowEffects(v) => s.low_effects = v,
            OptionChange::DevMode(v) => s.dev_mode = v,
        }
        true
    });
    emit(&app, HOME_STATE, state.home());
}

#[tauri::command(async)]
pub fn get_about(app: AppHandle, state: State<'_, AppState>) -> AboutView {
    let v = versions_of(&state.pack_state());
    AboutView {
        pack_version: v.pack,
        launcher_version: app.package_info().version.to_string(),
        me3_version: v.me3,
        game_version: v.game,
        update_source: None,
    }
}

#[tauri::command(async)]
pub fn create_shortcut(app: AppHandle, state: State<'_, AppState>) -> Result<String, String> {
    let desktop = report::desktop_dir().ok_or_else(|| "找不到桌面".to_owned())?;
    let _ = &state;
    create_desktop_shortcut(&desktop, &app.package_info().name).map(|p| p.display().to_string())
}
