//! 蓝10（shared::tiered_quick_forge）：持有锻体、装备界面开着时，叠加层显示「强化武器」「继承词条」两个按钮（ui::overlay），
//! 点按钮或按热键（quick_forge_key / quick_inherit_key）打开强化界面或词条继承面板。渲染线程只发点击，打开都在游戏线程。

use std::sync::Arc;

use log::{info, warn};
use nr_stat_alloc_shared::tiered_quick_forge::QUICK_FORGE;

use crate::bridge::{QuickForgeAction, UiBridge};
use crate::hotkey::HotkeyState;

use super::Frame;

pub struct QuickForge {
    keys: HotkeyState,
    forge_key: u32,
    inherit_key: u32,
    bridge: Arc<UiBridge>,
    shown: bool,
}

impl QuickForge {
    pub fn new(forge_key: u32, inherit_key: u32, bridge: Arc<UiBridge>) -> Self {
        Self { keys: HotkeyState::new(&[forge_key, inherit_key]), forge_key, inherit_key, bridge, shown: false }
    }

    pub fn check(&mut self, frame: &Frame, in_equip_menu: impl FnOnce() -> bool, open_forge: impl FnOnce() -> bool, open_inherit: impl FnOnce()) {
        // 有锻体才查界面（要读好几次内存）
        let shown = frame.ids.contains(&QUICK_FORGE) && in_equip_menu();
        self.set_shown(shown);
        let keys = self.keys.poll();
        let click = self.bridge.take_quick_forge_click();
        if !shown {
            return;
        }
        match choose(click, keys.contains(&self.forge_key), keys.contains(&self.inherit_key)) {
            Some(QuickForgeAction::Inherit) => {
                open_inherit();
                info!("quick forge: passive inherit requested");
            }
            Some(QuickForgeAction::Reinforce) if open_forge() => info!("quick forge: reinforce menu opened"),
            Some(QuickForgeAction::Reinforce) => warn!("quick forge: reinforce menu not opened"),
            None => {}
        }
    }

    /// 离开出征：按钮收起
    pub fn reset(&mut self) {
        self.set_shown(false);
    }

    fn set_shown(&mut self, shown: bool) {
        if shown != self.shown {
            self.shown = shown;
            self.bridge.set_quick_forge_shown(shown);
        }
    }
}

/// 按钮点击优先，其次热键
fn choose(click: Option<QuickForgeAction>, forge_pressed: bool, inherit_pressed: bool) -> Option<QuickForgeAction> {
    click.or(if inherit_pressed {
        Some(QuickForgeAction::Inherit)
    } else if forge_pressed {
        Some(QuickForgeAction::Reinforce)
    } else {
        None
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn frame(ids: &[i32]) -> Frame<'_> {
        Frame { ids, hp: (500, 1000), runes: 0 }
    }

    /// 点击一次后跑一帧，返回 (打开了强化, 打开了继承, 按钮是否显示)
    fn run(ids: &[i32], click: QuickForgeAction, in_menu: bool) -> (bool, bool, bool) {
        let bridge = Arc::new(UiBridge::default());
        let mut q = QuickForge::new(0, 0, Arc::clone(&bridge));
        bridge.click_quick_forge(click);
        let (forge, inherit) = (Cell::new(false), Cell::new(false));
        q.check(&frame(ids), || in_menu, || { forge.set(true); true }, || inherit.set(true));
        (forge.get(), inherit.get(), bridge.quick_forge_shown())
    }

    #[test]
    fn each_button_opens_its_own_menu() {
        assert_eq!(run(&[QUICK_FORGE], QuickForgeAction::Reinforce, true), (true, false, true));
        assert_eq!(run(&[QUICK_FORGE], QuickForgeAction::Inherit, true), (false, true, true));
    }

    #[test]
    fn needs_the_buff_and_the_equip_menu() {
        assert_eq!(run(&[], QuickForgeAction::Reinforce, true), (false, false, false));
        assert_eq!(run(&[QUICK_FORGE], QuickForgeAction::Inherit, false), (false, false, false));
    }

    #[test]
    fn click_wins_over_keys() {
        assert_eq!(choose(Some(QuickForgeAction::Reinforce), false, true), Some(QuickForgeAction::Reinforce));
        assert_eq!(choose(None, true, false), Some(QuickForgeAction::Reinforce));
        assert_eq!(choose(None, false, true), Some(QuickForgeAction::Inherit));
        assert_eq!(choose(None, false, false), None);
    }

    #[test]
    fn reset_hides_the_buttons() {
        let bridge = Arc::new(UiBridge::default());
        let mut q = QuickForge::new(0, 0, Arc::clone(&bridge));
        q.check(&frame(&[QUICK_FORGE]), || true, || true, || {});
        assert!(bridge.quick_forge_shown());
        q.reset();
        assert!(!bridge.quick_forge_shown());
    }
}
