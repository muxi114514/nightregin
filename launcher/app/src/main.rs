//! 启动器入口：单实例预检 → 找整合包 → 装日志 → 查 WebView2 → 读设置并补查上次退出 → 建 Tauri 应用。
//! `--smoke`：跳过前面几步，窗口始终隐藏，前端就绪即以 0 退出，30 秒没就绪以 2 退出（build.ps1 冒烟用）
#![windows_subsystem = "windows"]

mod commands;
mod launch_thread;
mod panic_dialog;
mod shortcut;
mod startup;
mod state;
mod tray;
mod webview;
mod wiki_protocol;
mod window;

use std::sync::Arc;
use std::time::Duration;

use launcher_core::config::service::ConfigEvent;
use launcher_core::logging;
use launcher_core::os::WinOs;
use tauri::plugin::TauriPlugin;
use tauri::{AppHandle, Context, Manager, Wry};

use crate::commands::{CONFIG_WRITE_FAILED, GAME_SETTINGS, HOME_STATE};
use crate::state::AppState;

/// 冒烟模式下等待前端就绪的时限，超时以 2 退出
const SMOKE_TIMEOUT: Duration = Duration::from_secs(30);
const SMOKE_TIMEOUT_EXIT_CODE: i32 = 2;

fn main() {
    // args() 遇到不是合法 Unicode 的参数会 panic
    let smoke = std::env::args_os().skip(1).any(|arg| arg == "--smoke");
    // 只展开一次：每展开一次就把前端资源再嵌进 exe 一份
    let context = tauri::generate_context!();
    if smoke {
        spawn_smoke_timeout();
        run(context, AppState::smoke(startup::exe_dir()));
        return;
    }
    // 第 0 步：已有启动器在运行就跳过 1~4 步（不轮换日志、不改设置），交给单实例插件转交后退出
    if startup::instance_running(&context.config().identifier) {
        forward_to_running(context);
        return;
    }
    let title = context.package_info().name.clone();
    let Some(pack) = startup::discover_pack(&title) else {
        std::process::exit(1);
    };
    let logs = pack.logs_dir();
    if let Err(e) = logging::init(&logs) {
        // 日志装不上也照常运行，只用对话框提示一次
        startup::show_error(&title, &startup::log_unavailable_text(&logs, &e));
    }
    log::info!(
        "启动器 {} 启动，整合包：{}",
        context.package_info().version,
        pack.root.display()
    );
    if !startup::ensure_webview2(&pack, &title) {
        std::process::exit(1);
    }
    let os = Arc::new(WinOs);
    panic_dialog::install(
        pack.clone(),
        title.clone(),
        context.package_info().version.to_string(),
    );
    let (store, settings) = startup::load_settings(&pack, &*os);
    run(context, AppState::new(pack, os, store, settings));
}

/// 第 5 步。单实例插件必须第一个注册；冒烟不注册它：要验证的是自己的窗口，不能转交给正在运行的启动器
fn run(context: Context<Wry>, state: AppState) {
    let smoke = state.is_smoke();
    let mut builder = tauri::Builder::default();
    if !smoke {
        builder = builder.plugin(single_instance());
    }
    let result = builder
        .plugin(tauri_plugin_dialog::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            commands::launch::ui_ready,
            commands::launch::get_home,
            commands::launch::start_game,
            commands::launch::cancel_launch,
            commands::launch::end_processes,
            commands::launch::focus_game,
            commands::launch::choose_game_dir,
            commands::launch::get_launch_view,
            commands::launch::minimize_window,
            commands::launch::close_app,
            commands::game_settings::get_game_settings,
            commands::game_settings::set_game_setting,
            commands::game_settings::restore_config_default,
            commands::game_settings::fix_everdark,
            commands::game_settings::align_suffix,
            commands::saves::get_saves,
            commands::saves::backup_now,
            commands::saves::restore_backup,
            commands::saves::open_save_folder,
            commands::saves::set_save_account,
            commands::saves::copy_from_vanilla,
            commands::options::get_options,
            commands::options::set_option,
            commands::options::get_about,
            commands::options::create_shortcut,
            commands::report::ack_open_report,
            commands::report::preview_report,
            commands::report::create_report,
            commands::report::reveal_path,
            commands::first_run::get_first_run,
            commands::first_run::finish_first_run,
            commands::wiki::get_wiki,
            commands::wiki::get_wiki_odds,
        ])
        .register_asynchronous_uri_scheme_protocol(wiki_protocol::SCHEME, wiki_protocol::handle)
        .on_window_event(window::on_window_event)
        .setup(move |app| {
            setup(app.handle(), smoke);
            Ok(())
        })
        .run(context);
    if let Err(e) = result {
        log::error!("启动器运行失败：{e}");
        std::process::exit(1);
    }
}

/// 建隐藏的主窗口、关浏览器功能；正常模式再起 3 秒兜底显示、建托盘。
/// 出错不交给 Tauri（setup 返回 Err 会 panic），自己提示后退出
fn setup(app: &AppHandle, smoke: bool) {
    let window = match window::build_main(app) {
        Ok(window) => window,
        Err(e) => {
            log::error!("创建主窗口失败：{e}");
            if !smoke {
                startup::show_error(
                    &app.package_info().name,
                    &format!("无法创建启动器窗口：{e}"),
                );
            }
            app.exit(1);
            return;
        }
    };
    webview::setup(&window);
    if smoke {
        return;
    }
    window::spawn_reveal_fallback(app);
    spawn_config_writer(app);
    if let Err(e) = tray::create(app) {
        log::error!("创建托盘图标失败：{e}");
    }
}

/// 后台写配置：写完告诉前端刷新；线程建不起来时改动在开始游戏和退出前照样写回
fn spawn_config_writer(app: &AppHandle) {
    let handle = app.clone();
    let spawned = std::thread::Builder::new()
        .name("config-writer".into())
        .spawn(move || {
            let state = handle.state::<AppState>();
            // 记下现在的内容，之后才分得出玩家手改
            state.config().view();
            state.config().run_worker(&|event| match event {
                ConfigEvent::Applied { external_change } => {
                    if external_change {
                        log::info!("写回时发现配置被外部改过，已在最新内容上改");
                    }
                    commands::emit(&handle, GAME_SETTINGS, state.game_settings());
                    commands::emit(&handle, HOME_STATE, state.home());
                }
                ConfigEvent::WriteFailed(message) => {
                    commands::emit(&handle, CONFIG_WRITE_FAILED, message);
                    // 写不进去的改动可能已丢弃，界面回到磁盘上的值
                    commands::emit(&handle, GAME_SETTINGS, state.game_settings());
                }
            });
        });
    if let Err(e) = spawned {
        log::error!("无法创建配置写入线程：{e}");
    }
}

/// 已有启动器在运行：插件把参数转交给它并退出本进程。
/// 走到 setup 说明没转交成（那个实例恰好退出，或以管理员身份运行），直接退出
fn forward_to_running(context: Context<Wry>) {
    let result = tauri::Builder::default()
        .plugin(single_instance())
        .setup(|app| {
            app.handle().exit(0);
            Ok(())
        })
        .run(context);
    if result.is_err() {
        std::process::exit(1);
    }
}

/// 再启动一次时，已在运行的实例把主窗口还原、显示并聚焦
fn single_instance() -> TauriPlugin<Wry> {
    tauri_plugin_single_instance::init(|app, _args, _cwd| window::show_main(app))
}

/// 前端起不来时不让构建脚本卡住；线程建不起来时由 build.ps1 的 60 秒上限兜底
fn spawn_smoke_timeout() {
    let _ = std::thread::Builder::new()
        .name("smoke-timeout".into())
        .spawn(|| {
            std::thread::sleep(SMOKE_TIMEOUT);
            std::process::exit(SMOKE_TIMEOUT_EXIT_CODE);
        });
}
