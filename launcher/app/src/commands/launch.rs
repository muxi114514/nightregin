//! 开始游戏、窗口、选游戏目录

use std::path::{Path, PathBuf};
use std::sync::mpsc;

use launcher_core::launch::{LaunchJob, LaunchView};
use launcher_core::paths::{GAME_EXE, GameDir};
use launcher_core::preflight::HomeState;
use serde::Serialize;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;

use crate::launch_thread;
use crate::state::{self, AppState};
use crate::window;

/// 选游戏文件夹的结果：取消不提示，选错要提示
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ChooseDir {
    Cancelled,
    Rejected,
    Chosen { path: String },
}

/// 前端画好首帧。冒烟模式关窗后以 0 退出；否则显示窗口（与 3 秒兜底谁先到谁显示）。
/// 冒烟要在命令里关掉自己的窗口，不能在主线程上同步执行
#[tauri::command(async)]
pub fn ui_ready(app: AppHandle, state: State<'_, AppState>) {
    if state.is_smoke() {
        window::destroy_main(&app);
        app.exit(0);
    } else {
        window::reveal(&app);
    }
}

/// 要读盘，放到线程池执行
#[tauri::command(async)]
pub fn get_home(state: State<'_, AppState>) -> HomeState {
    state.home()
}

#[tauri::command(async)]
pub fn start_game(app: AppHandle, skip_backup: bool, ignore_version: bool) -> Result<(), String> {
    launch_thread::start_launch(
        &app,
        LaunchJob {
            skip_backup,
            ignore_version,
        },
    )
}

/// Esc：还没到唤起才取消，返回 true
#[tauri::command]
pub fn cancel_launch(state: State<'_, AppState>) -> bool {
    state.cancel_launch()
}

/// 「游戏仍在载入」时结束 me3 和游戏进程
#[tauri::command]
pub fn end_processes(state: State<'_, AppState>) {
    state.end_processes();
}

#[tauri::command(async)]
pub fn focus_game(state: State<'_, AppState>) -> bool {
    state.focus_game()
}

/// 选游戏根目录：根目录下要有 Game\nightreign.exe（直接选 Game 文件夹也认），合格才写进设置并返回路径
#[tauri::command(async)]
pub fn choose_game_dir(app: AppHandle, state: State<'_, AppState>) -> ChooseDir {
    let Some(picked) = pick_folder(&app) else {
        return ChooseDir::Cancelled;
    };
    let Some(root) = resolve_game_root(&picked, &|p| p.is_file()) else {
        log::info!("选的目录里没有游戏：{}", picked.display());
        return ChooseDir::Rejected;
    };
    let game = GameDir { root };
    log::info!("手动指定游戏目录：{}", game.root.display());
    let path = game.root.display().to_string();
    state.set_game_dir(game.root);
    ChooseDir::Chosen { path }
}

/// 页面重载后接回启动过场
#[tauri::command]
pub fn get_launch_view(state: State<'_, AppState>) -> LaunchView {
    state.launch_view()
}

#[tauri::command]
pub fn minimize_window(app: AppHandle) {
    window::minimize_main(&app);
}

#[tauri::command(async)]
pub fn close_app(app: AppHandle) {
    state::quit(&app);
}

/// 弹出选文件夹对话框并等它关掉；取消或出错时为 None
fn pick_folder(app: &AppHandle) -> Option<PathBuf> {
    let (tx, rx) = mpsc::channel();
    let mut dialog = app.dialog().file().set_title("选择游戏所在文件夹");
    if let Some(window) = app.get_webview_window(window::MAIN) {
        dialog = dialog.set_parent(&window);
    }
    dialog.pick_folder(move |picked| {
        // 接收端只在这边等，不会提前丢弃
        let _ = tx.send(picked);
    });
    // 对话框没弹出来时回调不会执行，发送端随之丢弃，recv 返回 Err
    let picked = rx.recv().ok().flatten()?;
    picked
        .into_path()
        .inspect_err(|e| log::warn!("选中的文件夹不是本地路径：{e}"))
        .ok()
}

/// 由所选文件夹定出游戏根目录：根目录下有 Game\nightreign.exe 就用它；
/// 玩家常直接选 Game 文件夹（nightreign.exe 就在里面），此时其上级若是合格的根目录就改用上级。
/// is_file 判断某路径是否为已有文件，便于脱离磁盘测试
fn resolve_game_root(selected: &Path, is_file: &dyn Fn(&Path) -> bool) -> Option<PathBuf> {
    let valid = |root: &Path| {
        is_file(
            &GameDir {
                root: root.to_path_buf(),
            }
            .exe(),
        )
    };
    if valid(selected) {
        return Some(selected.to_path_buf());
    }
    let parent = selected.parent()?;
    (is_file(&selected.join(GAME_EXE)) && valid(parent)).then(|| parent.to_path_buf())
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    /// 只认给定文件存在的假磁盘
    fn disk(files: Vec<PathBuf>) -> impl Fn(&Path) -> bool {
        let set: HashSet<PathBuf> = files.into_iter().collect();
        move |p| set.contains(p)
    }

    /// 磁盘上只有 nr\Game\nightreign.exe
    fn nr_disk() -> impl Fn(&Path) -> bool {
        disk(vec![Path::new("nr").join("Game").join("nightreign.exe")])
    }

    #[test]
    fn root_selected_keeps_root() {
        assert_eq!(
            resolve_game_root(Path::new("nr"), &nr_disk()),
            Some(PathBuf::from("nr"))
        );
    }

    #[test]
    fn game_folder_selected_uses_parent() {
        assert_eq!(
            resolve_game_root(&Path::new("nr").join("Game"), &nr_disk()),
            Some(PathBuf::from("nr"))
        );
    }

    #[test]
    fn wrong_folder_is_rejected() {
        assert_eq!(resolve_game_root(Path::new("other"), &nr_disk()), None);
        // 上级是合格的根目录，但所选文件夹里没有 nightreign.exe：不能越级当作根目录
        assert_eq!(
            resolve_game_root(&Path::new("nr").join("saves"), &nr_disk()),
            None
        );
    }
}
