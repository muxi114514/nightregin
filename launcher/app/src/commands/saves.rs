//! 存档页：账号一律用字符串（SteamID64 超过 2^53），文件只按备份名找

use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;

use chrono::Local;
use launcher_core::os::{GAME_EXE, Os};
use launcher_core::preflight::suffix_check;
use launcher_core::savepage::{self, BackupNow, BackupRow, SaveContext, SavesView};
use launcher_core::saves::{self, VanillaCopy, nightreign_save_root};
use launcher_core::steam::{read_active_user, steam_id64};
use launcher_core::suffix::suffix_view;
use tauri::{AppHandle, State};

use super::{HOME_STATE, emit};
use crate::state::AppState;

/// SaveContext 借用的东西
struct SaveEnv {
    root: Option<PathBuf>,
    savefile: String,
    active: Option<u64>,
    remembered: Option<u64>,
    os: Arc<dyn Os>,
}

impl SaveEnv {
    fn new(state: &AppState) -> SaveEnv {
        SaveEnv {
            root: nightreign_save_root(),
            savefile: state.pack_state().savefile().to_owned(),
            active: read_active_user().and_then(steam_id64),
            remembered: state.settings().save_account,
            os: state.os(),
        }
    }

    fn ctx(&self) -> SaveContext<'_> {
        SaveContext {
            root: self.root.as_deref(),
            savefile: &self.savefile,
            active: self.active,
            remembered: self.remembered,
            os: &*self.os,
        }
    }
}

fn account_id(account: &str) -> Result<u64, String> {
    account.parse().map_err(|_| format!("账号不对：{account}"))
}

#[tauri::command(async)]
pub fn get_saves(state: State<'_, AppState>, account: Option<String>) -> Result<SavesView, String> {
    let account = account.as_deref().map(account_id).transpose()?;
    let pack = state.pack_state();
    let suffix = suffix_view(&suffix_check(&pack), pack.chance_setter.as_ref());
    Ok(savepage::saves_view(
        &SaveEnv::new(&state).ctx(),
        account,
        suffix,
    ))
}

#[tauri::command(async)]
pub fn backup_now(
    state: State<'_, AppState>,
    account: String,
    force: bool,
) -> Result<BackupNow, String> {
    let now = Local::now().naive_local();
    savepage::backup_now(
        &SaveEnv::new(&state).ctx(),
        account_id(&account)?,
        force,
        now,
    )
    .map_err(|e| e.to_string())
}

/// 返回这次的恢复前备份；前端用它把那一行排到最上面
#[tauri::command(async)]
pub fn restore_backup(
    app: AppHandle,
    state: State<'_, AppState>,
    account: String,
    name: String,
) -> Result<Option<BackupRow>, String> {
    let _op = state.begin_save_op()?;
    let now = Local::now().naive_local();
    let row = savepage::restore_by_name(
        &SaveEnv::new(&state).ctx(),
        account_id(&account)?,
        &name,
        now,
    )
    .map_err(|e| e.to_string())?;
    emit(&app, HOME_STATE, state.home());
    Ok(row)
}

#[tauri::command(async)]
pub fn open_save_folder(state: State<'_, AppState>, account: String) -> Result<(), String> {
    let env = SaveEnv::new(&state);
    let dir = env
        .ctx()
        .location(account_id(&account)?)
        .map(|save| save.dir)
        .filter(|dir| dir.is_dir())
        .ok_or_else(|| "这个账号还没有存档文件夹".to_owned())?;
    Command::new("explorer")
        .arg(dir)
        .spawn()
        .map(drop)
        .map_err(|e| format!("打开资源管理器失败：{e}"))
}

#[tauri::command(async)]
pub fn set_save_account(state: State<'_, AppState>, account: String) -> Result<(), String> {
    let id = account_id(&account)?;
    state.update_settings(|s| {
        s.save_account = Some(id);
        true
    });
    Ok(())
}

#[tauri::command(async)]
pub fn copy_from_vanilla(
    state: State<'_, AppState>,
    account: String,
) -> Result<VanillaCopy, String> {
    if state.os().find_process(GAME_EXE).is_some() {
        return Err("游戏运行中，请先退出游戏".to_owned());
    }
    let _op = state.begin_save_op()?;
    let env = SaveEnv::new(&state);
    let save = env
        .ctx()
        .location(account_id(&account)?)
        .ok_or_else(|| "找不到存档文件夹".to_owned())?;
    saves::copy_from_vanilla(&save).map_err(|e| format!("复制失败：{e}"))
}
