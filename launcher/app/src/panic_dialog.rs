//! 启动器自己出错后的原生对话框：问要不要整理问题记录。
//! 不碰 AppState（panic 时它的锁可能正被持有），需要的路径在启动时填好

use std::sync::OnceLock;
use std::time::SystemTime;

use chrono::Local;
use launcher_core::logging;
use launcher_core::paths::PackLayout;
use launcher_core::preflight::PackState;
use launcher_core::report;
use launcher_core::settings::Settings;
use launcher_core::steam::locate_game_via_steam;

use crate::commands::report::{human_size, inputs_for};
use crate::startup;

struct Context {
    pack: PackLayout,
    title: String,
    launcher_version: String,
}

static CONTEXT: OnceLock<Context> = OnceLock::new();

pub fn install(pack: PackLayout, title: String, launcher_version: String) {
    let context = Context {
        pack,
        title,
        launcher_version,
    };
    if CONTEXT.set(context).is_err() {
        log::warn!("问题记录对话框已经装过");
    }
    logging::set_panic_callback(Box::new(|| offer("启动器出错了。")));
}

/// reason 是对话框第一句
pub fn offer(reason: &str) {
    let Some(ctx) = CONTEXT.get() else {
        return;
    };
    let state = PackState::load(
        ctx.pack.clone(),
        Settings::default(),
        &locate_game_via_steam,
    );
    let inputs = inputs_for(&state, &ctx.launcher_version);
    let now = SystemTime::now();
    let preview = report::preview(&inputs, now);
    let dumps = if preview.dumps.is_empty() {
        String::new()
    } else {
        format!(
            "\n\n会带上 {} 个崩溃转储（{}），崩溃转储可能包含账号名等信息。",
            preview.dumps.len(),
            human_size(preview.dump_bytes)
        )
    };
    let text = format!("{reason}\n要把问题记录整理到桌面吗？{dumps}");
    if !startup::confirm(&ctx.title, &text) {
        return;
    }
    match report::create(&inputs, now, Local::now().naive_local()) {
        Ok(path) => {
            startup::show_info(&ctx.title, &format!("问题记录已放到：\n{}", path.display()))
        }
        Err(e) => startup::show_error(&ctx.title, &e.to_string()),
    }
}
