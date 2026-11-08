//! 托盘：左键单击显示主界面；菜单「显示主界面 / 开始游戏 / 整理问题记录 / 退出」

use std::time::Duration;

use launcher_core::launch::LaunchJob;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

use crate::commands::{self, OPEN_REPORT};
use crate::state::AppState;
use crate::{launch_thread, panic_dialog, state, window};

const TRAY_ID: &str = "main";
const SHOW: &str = "show";
const START: &str = "start";
const REPORT: &str = "report";
const QUIT: &str = "quit";
/// 过这么久界面还没回执，就当界面坏了
const REPORT_ACK_TIMEOUT: Duration = Duration::from_secs(5);

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let menu = Menu::with_items(
        app,
        &[
            &MenuItem::with_id(app, SHOW, "显示主界面", true, None::<&str>)?,
            &MenuItem::with_id(app, START, "开始游戏", true, None::<&str>)?,
            &MenuItem::with_id(app, REPORT, "整理问题记录", true, None::<&str>)?,
            &MenuItem::with_id(app, QUIT, "退出", true, None::<&str>)?,
        ],
    )?;
    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip(&app.package_info().name)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| on_menu(app, event.id().as_ref()))
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                window::show_main(tray.app_handle());
            }
        });
    match app.default_window_icon() {
        Some(icon) => builder = builder.icon(icon.clone()),
        None => log::warn!("没有默认图标，托盘图标会是空白"),
    }
    builder.build(app)?;
    Ok(())
}

/// 托盘建成了没有（没建成时隐入托盘改为最小化）
pub fn exists(app: &AppHandle) -> bool {
    app.tray_by_id(TRAY_ID).is_some()
}

/// 交给界面做（它能列出转储给玩家确认）；界面坏了没有回执时改用原生对话框
fn open_report(app: &AppHandle) {
    window::show_main(app);
    let acks = app.state::<AppState>().report_acks();
    commands::emit(app, OPEN_REPORT, ());
    let handle = app.clone();
    let spawned = std::thread::Builder::new()
        .name("report-watchdog".into())
        .spawn(move || {
            std::thread::sleep(REPORT_ACK_TIMEOUT);
            if handle.state::<AppState>().report_acks() == acks {
                panic_dialog::offer("启动器界面没有响应。");
            }
        });
    if let Err(e) = spawned {
        log::warn!("无法创建线程：{e}");
    }
}

fn on_menu(app: &AppHandle, id: &str) {
    match id {
        SHOW => window::show_main(app),
        // 开始成功才弹出窗口：启动过场和失败提示都在窗口里；已在启动中（例如游戏运行时）什么都不做
        // 首次运行没走完时只显示窗口，让玩家先走完
        START if !app.state::<AppState>().can_start_from_tray() => window::show_main(app),
        START => match launch_thread::start_launch(app, LaunchJob::default()) {
            Ok(()) => window::show_main(app),
            Err(e) => log::info!("托盘「开始游戏」没有执行：{e}"),
        },
        REPORT => open_report(app),
        QUIT => state::quit(app),
        _ => {}
    }
}
