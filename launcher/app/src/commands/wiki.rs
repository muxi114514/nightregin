//! 图鉴：数据和当前几率

use launcher_core::wiki::odds::WikiOdds;
use launcher_core::wiki::{WikiView, read_wiki};
use tauri::State;

use crate::state::AppState;

#[tauri::command(async)]
pub fn get_wiki(state: State<'_, AppState>) -> WikiView {
    read_wiki(state.pack())
}

#[tauri::command(async)]
pub fn get_wiki_odds(state: State<'_, AppState>) -> WikiOdds {
    state.wiki_odds()
}
