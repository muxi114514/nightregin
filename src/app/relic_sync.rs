//! 遗物重铸与叠加层同步：遗物装配界面打开时定期发布当前角色的遗物与货币，执行面板发来的重铸请求。

use std::time::{Duration, Instant};

use log::{error, info};

use crate::game::world;
use crate::relic_reroll::{RerollOutcome, RerollRequest};
use crate::tr;

use super::App;

/// 发布间隔：换遗物后面板最多 0.25 秒内跟上，每次只读 6 个实例
const PUBLISH_INTERVAL: Duration = Duration::from_millis(250);

impl App {
    /// 遗物装配界面打开时定期读取，有变化才发布
    pub(super) fn sync_relics(&mut self) {
        if !self.relic_menu_open || self.reroll.is_none() || self.last_relic_publish.elapsed() < PUBLISH_INTERVAL {
            return;
        }
        self.last_relic_publish = Instant::now();
        let Some((gd, hero)) = self.current_hero() else { return };
        let Some(view) = self.reroll.as_ref().and_then(|r| r.view(gd, hero)) else { return };
        if self.last_relic_view.as_ref() != Some(&view) {
            self.last_relic_view = Some(view.clone());
            self.bridge.publish_relics(view);
        }
    }

    pub(super) fn run_reroll(&mut self, req: &RerollRequest) {
        let current = self.current_hero();
        let result = match (current, self.reroll.as_mut()) {
            (Some((gd, hero)), Some(r)) => r.execute(gd, hero, req),
            _ => Err(tr!("重铸不可用（读不到角色或遗物数据）", "Reroll unavailable (hero or relic data not readable)")),
        };
        let outcome = match result {
            Ok((text, new_effects)) => RerollOutcome { ok: true, text, new_effects },
            Err(text) => RerollOutcome { ok: false, text, new_effects: Vec::new() },
        };
        if outcome.ok {
            info!("ui: {}", outcome.text);
        } else {
            error!("ui: {}", outcome.text);
        }
        self.bridge.post_reroll_outcome(outcome);
        // 下一帧立即重新发布，面板马上看到新词条与余额
        self.last_relic_view = None;
        self.last_relic_publish = Instant::now().checked_sub(PUBLISH_INTERVAL).unwrap_or_else(Instant::now);
    }

    /// 当前 PlayerGameData 与角色序号
    fn current_hero(&mut self) -> Option<(usize, usize)> {
        let player = world::main_player(&self.addrs)?;
        let gd = self.locator.game_data(&self.addrs, player)?;
        let snapshot = self.locator.snapshot(gd)?;
        let hero = self.data.as_ref()?.hero_of_chara(snapshot.chara_init)?;
        Some((gd, hero))
    }
}
