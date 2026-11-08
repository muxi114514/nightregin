//! 词条继承的每帧任务（游戏主线程）：锻造台对话脚本的请求或测试热键（[diag] inherit_panel_key）打开继承面板；
//! 面板打开期间定期发布 6 个武器槽里的武器、执行面板发来的继承请求；离开出征地图时关闭面板。

use std::sync::Arc;
use std::time::{Duration, Instant};

use log::{info, warn};

use crate::bridge::UiBridge;
use crate::config::OffsetOverrides;
use crate::game::addresses::Addresses;
use crate::game::inventory;
use crate::game::player_data::PlayerDataLocator;
use crate::game::world;
use crate::hotkey::HotkeyState;

use super::{InheritOutcome, InheritView, PassiveInherit, entry};

/// 面板打开期间重读武器槽的间隔（换武器、拾取后面板随之更新）
const REFRESH_EVERY: Duration = Duration::from_millis(200);

pub struct InheritTask {
    service: PassiveInherit,
    bridge: Arc<UiBridge>,
    addrs: Addresses,
    locator: PlayerDataLocator,
    panel_key: Option<HotkeyState>,
    /// 上次发布的武器槽；None 表示下一帧立即重读并发布
    published: Option<InheritView>,
    last_read: Instant,
}

impl InheritTask {
    pub fn new(service: PassiveInherit, bridge: Arc<UiBridge>, addrs: Addresses, offsets: &OffsetOverrides, panel_key: u32) -> Self {
        let panel_key = (panel_key != 0).then(|| {
            info!("passive inherit panel test key {panel_key:#x}");
            HotkeyState::new(&[panel_key])
        });
        Self { service, bridge, addrs, locator: PlayerDataLocator::new(offsets), panel_key, published: None, last_read: Instant::now() }
    }

    pub fn tick(&mut self) {
        let pressed = self.panel_key.as_mut().is_some_and(|k| !k.poll().is_empty());
        let requested = entry::take_request();
        let open = self.bridge.inherit_open();
        if !pressed && !requested && !open {
            self.published = None;
            return;
        }
        let Some(equip) = self.expedition_equip() else {
            if pressed || requested {
                warn!("passive inherit: not in an expedition (or player data unreadable), ignored");
            }
            self.bridge.close_inherit();
            return;
        };
        if (pressed || requested) && !open {
            info!("passive inherit panel opened by {}", if requested { "request" } else { "hotkey" });
            // 上次关面板前没来得及执行的请求作废
            let _ = self.bridge.take_inherit_request();
            self.bridge.open_inherit();
            self.published = None;
        }
        if let Some(req) = self.bridge.take_inherit_request() {
            self.bridge.post_inherit_outcome(InheritOutcome::from(self.service.execute(equip, &req)));
            self.published = None;
        }
        self.refresh(equip);
    }

    fn refresh(&mut self, equip: usize) {
        if self.published.is_some() && self.last_read.elapsed() < REFRESH_EVERY {
            return;
        }
        self.last_read = Instant::now();
        let view = self.service.view(equip);
        if self.published.as_ref() != Some(&view) {
            self.bridge.publish_inherit_view(view.clone());
            self.published = Some(view);
        }
    }

    /// 出征中本地玩家的 EquipGameData
    fn expedition_equip(&mut self) -> Option<usize> {
        let player = world::main_player(&self.addrs)?;
        if !world::current_map(player).is_some_and(world::is_expedition) {
            return None;
        }
        self.locator.game_data(&self.addrs, player).map(inventory::equip_game_data)
    }
}
