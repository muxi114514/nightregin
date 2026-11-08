//! 与叠加层同步：发布面板编辑用的方案快照，执行面板发来的保存 / 删除命令。
//! 写文件与重载方案都在游戏线程完成，重载后下一次对账（≤0.5 秒）新方案就生效。

use log::{error, info};

use crate::bridge::{LiveStats, SchemeCommand, SchemeSnapshot, UiCommand};
use crate::game::player_data::PlayerSnapshot;
use crate::scheme_file;
use crate::stats::heroes::hero_name;
use crate::{lang, tr};
use crate::stats::rules;

use super::App;

/// 快照的发布依据：角色、器皿、等级、方案簿版本
pub(super) type SnapshotKey = (usize, Option<u32>, u32, u64);

impl App {
    /// 角色 / 器皿 / 等级或方案簿变化时发布新快照
    pub(super) fn publish_snapshot(&mut self, hero: usize, vessel: Option<u32>, level: u32) {
        let key = (hero, vessel, level, self.book_generation);
        if self.last_snapshot == Some(key) {
            return;
        }
        self.last_snapshot = Some(key);
        self.bridge.publish_snapshot(SchemeSnapshot {
            hero,
            vessel,
            level,
            vessel_spec: vessel.and_then(|v| self.book.spec(hero, Some(v))).cloned(),
            default_spec: self.book.spec(hero, None).cloned(),
        });
    }

    /// 实时属性 / 三维上限变化时发布（目标属性 = 原版 + 当前生效的差值）
    pub(super) fn publish_live(&mut self, hero: usize, s: &PlayerSnapshot) {
        let Some(data) = self.data.as_ref() else { return };
        let vanilla = data.growth[hero].stats_at(s.level);
        let delta = self.status.delta();
        let live = LiveStats {
            hero,
            level: s.level,
            effective: s.effective,
            target: std::array::from_fn(|i| (vanilla[i] as i32 + delta[i]).max(1) as u32),
            max: [s.vitals[0][1], s.vitals[1][1], s.vitals[2][1]],
        };
        if self.last_live != Some(live) {
            self.last_live = Some(live);
            self.bridge.publish_live(live);
        }
    }

    pub(super) fn process_ui_commands(&mut self) {
        for cmd in self.bridge.take_commands() {
            let cmd = match cmd {
                UiCommand::Scheme(c) => c,
                UiCommand::Reroll(req) => {
                    self.run_reroll(&req);
                    continue;
                }
            };
            let (text, ok) = match self.run_command(&cmd) {
                Ok(t) => (t, true),
                Err(e) => (e, false),
            };
            if ok {
                info!("ui: {text}");
            } else {
                error!("ui: {text}");
            }
            self.bridge.post_message(text, ok);
        }
    }

    fn run_command(&mut self, cmd: &SchemeCommand) -> Result<String, String> {
        let data = self.data.clone().ok_or_else(|| tr!("未能读取游戏参数表，无法保存方案", "Game params could not be read; cannot save schemes"))?;
        let label = |hero: usize, vessel: Option<u32>| {
            let target = match vessel {
                Some(id) => data.vessel(id).map_or_else(|| id.to_string(), |v| v.display_name().to_string()),
                None => lang::pick("角色默认方案", "hero default scheme").to_string(),
            };
            format!("{} · {target}", hero_name(hero))
        };
        let text = match cmd {
            SchemeCommand::Save { hero, vessel, spec } => {
                // 面板已逐步校验，这里再校验一次，保证写进文件的方案一定能加载
                rules::build(spec, &data.growth[*hero], &[], self.limits()).map_err(|e| tr!("方案不合规，未保存：{e}", "Scheme is invalid and was not saved: {e}"))?;
                scheme_file::save(&self.dir, &*data, (*hero, *vessel), spec)?;
                let l = label(*hero, *vessel);
                tr!("已保存：{l}", "Saved: {l}")
            }
            SchemeCommand::Delete { hero, vessel } => {
                if scheme_file::remove(&self.dir, &*data, (*hero, *vessel))? {
                    let l = label(*hero, *vessel);
                    tr!("已删除：{l}", "Deleted: {l}")
                } else {
                    let l = label(*hero, *vessel);
                    tr!("{l} 还没有保存过方案", "{l} has no saved scheme")
                }
            }
        };
        self.reload_book();
        Ok(text)
    }
}
