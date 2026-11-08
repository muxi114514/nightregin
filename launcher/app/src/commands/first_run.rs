//! 首次运行

use launcher_core::firstrun::{FirstRunPlan, first_run_plan};
use launcher_core::saves::nightreign_save_root;
use launcher_core::steam::{read_active_user, steam_id64};
use tauri::State;

use crate::state::AppState;

#[tauri::command(async)]
pub fn get_first_run(state: State<'_, AppState>) -> FirstRunPlan {
    first_run_plan(
        &state.pack_state(),
        nightreign_save_root().as_deref(),
        read_active_user().and_then(steam_id64),
    )
}

/// 记下完成，account 是玩家选的账号（没选时为 None）
#[tauri::command(async)]
pub fn finish_first_run(state: State<'_, AppState>, account: Option<String>) -> Result<(), String> {
    let account = account
        .map(|a| a.parse::<u64>().map_err(|_| format!("账号不对：{a}")))
        .transpose()?;
    state.update_settings(|s| {
        s.first_run_done = true;
        if account.is_some() {
            s.save_account = account;
        }
        true
    });
    Ok(())
}
