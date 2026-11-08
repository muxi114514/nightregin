//! 按方案（角色 + 器皿）改写内存中的 CharaInitParam 皮肤行：每次对账时查各角色当前器皿的生效方案，
//! 武器选择变了才写；没有方案、方案没写武器或不合规时写回原值。不改 regulation，只影响自己的客户端。
//! 出征开局会按这张表重新发放初始武器，圆桌里切换角色也会（实机验证）；读档直接进圆桌时显示的是存档里的装备。
//! 有变化时顺带重算出击遗物的角色开关（relic_gates.rs），让改战技 / 改魔法 / 附加类遗物跟着武器走。
//! 只在游戏主线程（每帧任务）调用。

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use log::{info, warn};
use nr_stat_alloc_shared::loadouts::FieldEdit;

use crate::data::StartingData;
use crate::game::memory::write;
use crate::stats::heroes::{HERO_COUNT, HERO_NAMES_ZH};
use crate::stats::rules::WeaponPick;

use super::{pick, relic_gates};

pub struct LoadoutApplier {
    data: Arc<StartingData>,
    /// 角色序号 -> 已处理过的选择（不合规的也记下，避免重复刷日志）
    applied: HashMap<usize, WeaponPick>,
    /// 实际换了武器的角色（heroId）-> 带的武器
    carried: BTreeMap<u32, Vec<u32>>,
}

impl LoadoutApplier {
    pub fn new(data: Arc<StartingData>) -> Self {
        Self { data, applied: HashMap::new(), carried: BTreeMap::new() }
    }

    /// pick_of(角色序号) 返回该角色当前生效方案里的武器选择（None 为原版）
    pub fn sync(&mut self, mut pick_of: impl FnMut(usize) -> Option<WeaponPick>) {
        let mut changed = false;
        for hero in 0..HERO_COUNT {
            let pick = pick_of(hero).unwrap_or_default();
            if self.applied.get(&hero) == Some(&pick) {
                continue;
            }
            let name = HERO_NAMES_ZH[hero];
            let hero_id = hero as u32 + 1;
            let planned = if pick.is_vanilla() {
                None
            } else {
                pick::validate(&self.data, hero, &pick)
                    .and_then(|(main, off)| {
                        info!("starting weapons: {name} -> main {main}, off-hand {off:?}");
                        Ok((self.data.loadouts.plan(hero_id, main, off, &self.data.weapons)?, (main, off)))
                    })
                    .map_err(|e| warn!("starting weapons: {name} {pick:?}: {e}; using vanilla"))
                    .ok()
            };
            let edits = match planned {
                Some((edits, (main, off))) => {
                    self.carried.insert(hero_id, std::iter::once(main).chain(off).collect());
                    edits
                }
                None => {
                    self.carried.remove(&hero_id);
                    self.data.loadouts.restore(hero_id)
                }
            };
            self.write(name, &edits);
            self.applied.insert(hero, pick);
            changed = true;
        }
        if changed {
            relic_gates::apply(&self.data, &self.carried);
        }
    }

    fn write(&self, name: &str, edits: &[FieldEdit]) {
        let failed = edits.iter().filter(|e| !write(self.data.chara_init_address + e.offset, e.value)).count();
        if failed > 0 {
            warn!("starting weapons: {name}: {failed} of {} writes failed", edits.len());
        }
    }
}
