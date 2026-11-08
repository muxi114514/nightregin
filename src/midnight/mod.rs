//! 午夜模式（深夜栏上的开关）：结算积分与等级、联机同步、文本替换、拦准备。规则在 shared::midnight。

pub mod bridge;
pub mod curse;
pub mod lobby;
pub mod menu;
pub mod mode;
pub mod net;
pub mod progress;
pub mod ready;
pub mod result;
pub mod task;
pub mod texts;

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use log::{info, warn};

use crate::config::{MidnightConfig, OffsetOverrides};
use crate::game::addresses::{self, Addresses, MidnightMenuAddresses, ReadyAddresses};
use crate::game::event_flags::FlagHandler;
use crate::game::module::GameModule;
use crate::game::msg;
use crate::game::steam::SteamApi;

pub use bridge::MidnightBridge;
pub use task::MidnightTask;

/// 测试里会改全局开关的用例串行执行
#[cfg(test)]
pub(crate) fn test_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

#[cfg(test)]
mod tests {
    #[test]
    fn shutdown_releases_hook_state() {
        let _g = super::test_lock();
        super::ready::set_block(true);
        super::texts::set_state(true, true);
        super::shutdown();
        assert!(!super::ready::block_set());
        assert!(super::texts::provide(super::texts::MENU_CATEGORY, 131150).is_none());
        assert!(super::texts::provide(super::texts::MENU_CATEGORY, 132501).is_none());
    }
}

/// 叠加层装好后才能装的钩子用到的地址
pub struct MidnightNative {
    menu: MidnightMenuAddresses,
    ready: ReadyAddresses,
    msg_lookup: Option<usize>,
}

/// 旗标分发器要注册的处理函数：击败旗标观察
pub fn flag_handlers() -> [FlagHandler; 1] {
    [result::on_flag]
}

/// engine：诅咒的内存改写；diag_curse：强制诅咒编号（0 为随机）
pub fn setup(cfg: &MidnightConfig, dir: &Path, module: &GameModule, addrs: &Addresses, offsets: &OffsetOverrides, engine: curse::CurseEngine, diag_curse: u8) -> (MidnightTask, Arc<MidnightBridge>, MidnightNative) {
    let menu = addresses::resolve_midnight_menu(module);
    let native = MidnightNative { ready: addresses::resolve_ready(module), msg_lookup: addresses::resolve_msg_lookup(module), menu: menu.clone() };
    let bridge = Arc::new(MidnightBridge::default());
    let link = new_link(Duration::from_secs(cfg.resend_seconds.max(1)));
    let tracker = result::ResultTracker::new(progress::ProgressStore::new(dir), cfg.scoring(), result::global_boss_flags());
    info!("midnight ready: {} points, panel hook {}, {} sortie dialog(s)", tracker.progress().points, menu.panel_loader.is_some(), menu.matching_dialogs.len());
    let parts = task::TaskParts { addrs: addrs.clone(), menu_addrs: menu, bridge: Arc::clone(&bridge), link, tracker, engine, diag_curse };
    (MidnightTask::new(cfg, offsets, parts), bridge, native)
}

fn new_link(resend: Duration) -> Box<dyn net::MidnightLink + Send> {
    match SteamApi::load() {
        Some(api) => Box::new(lobby::LobbyLink::new(api, resend)),
        None => {
            warn!("midnight: Steam API unavailable, co-op sync disabled");
            Box::new(net::NullLink)
        }
    }
}

/// 每帧任务出错停用午夜时调用：放开拦准备、恢复原文本（钩子还在，但不再改任何东西）
pub fn shutdown() {
    ready::set_block(false);
    texts::set_state(false, false);
}

/// 需在叠加层之后调用
pub fn install_hooks(native: &MidnightNative) {
    if let Some(at) = native.msg_lookup {
        msg::install(at, texts::provide);
    }
    menu::install(&native.menu);
    ready::install(&native.ready);
}
