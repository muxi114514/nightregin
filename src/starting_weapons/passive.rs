//! 出征时给初始武器随机 1 条效力 1 的通用词条（之后不随等级升档）。
//! 每 0.5 秒扫一次本地玩家背包：找出当前皮肤的初始主手 / 副手、且词条槽全空的武器实例，抽一条写进第一个槽，
//! 再用原 ID 调一次 change_item_id 让游戏重新装备，被动效果随之生效（界面直接读词条槽）。
//! 只在出征地图生效，圆桌保持原样；每个实例只处理一次，回到圆桌后清空记录。只在游戏主线程（每帧任务）运行。

use std::collections::HashSet;
use std::sync::Arc;
use std::time::{Duration, Instant};

use log::{info, warn};
use nr_stat_alloc_shared::param::layout::chara_init;

use crate::config::OffsetOverrides;
use crate::data::StartingData;
use crate::game::addresses::{Addresses, GaitemAddresses};
use crate::game::gaitem::{Gaitems, PASSIVE_COUNT};
use crate::game::inventory;
use crate::game::memory::read;
use crate::game::player_data::PlayerDataLocator;
use crate::game::world;
use crate::rng::XorShift;

const SCAN_INTERVAL: Duration = Duration::from_millis(500);
/// 当前皮肤（CharaInitParam ID）
const PGD_CHARA_INIT: usize = 0xE8;

type ChangeItemIdFn = unsafe extern "C" fn(usize, u32, u32) -> usize;

pub struct StartingPassives {
    addrs: Addresses,
    locator: PlayerDataLocator,
    data: Arc<StartingData>,
    gaitems: Gaitems,
    change_item_id: ChangeItemIdFn,
    rng: XorShift,
    done: HashSet<u32>,
    last_scan: Instant,
}

impl StartingPassives {
    pub fn new(addrs: Addresses, offsets: &OffsetOverrides, data: Arc<StartingData>, gaitem: &GaitemAddresses) -> Self {
        Self {
            addrs,
            locator: PlayerDataLocator::new(offsets),
            data,
            gaitems: Gaitems::new(gaitem),
            change_item_id: unsafe { std::mem::transmute::<usize, ChangeItemIdFn>(gaitem.change_item_id) },
            rng: XorShift::from_time(),
            done: HashSet::new(),
            last_scan: Instant::now(),
        }
    }

    pub fn tick(&mut self) {
        if self.last_scan.elapsed() < SCAN_INTERVAL {
            return;
        }
        self.last_scan = Instant::now();
        let Some(player) = world::main_player(&self.addrs) else { return };
        if !world::current_map(player).is_some_and(world::is_expedition) {
            self.done.clear();
            return;
        }
        let Some(game_data) = self.locator.game_data(&self.addrs, player) else { return };
        let Some((hero_id, starting)) = self.starting_weapons(game_data) else { return };
        let equip = inventory::equip_game_data(game_data);
        for entry in inventory::weapons(equip) {
            if !starting.contains(&(entry.item_id - entry.item_id % 100)) || self.done.contains(&entry.handle) {
                continue;
            }
            let Some(weapon) = self.gaitems.weapon(entry.handle) else { continue };
            self.done.insert(entry.handle);
            if (0..PASSIVE_COUNT).any(|slot| weapon.passive(slot).is_some()) {
                continue;
            }
            let random = self.rng.next();
            let Some(id) = self.data.pool.roll(entry.item_id, &self.data.weapons, hero_id, random) else {
                warn!("starting passive: no passive pool for weapon {}", entry.item_id);
                continue;
            };
            if weapon.set_passive(0, id) {
                // 原 ID 再设一次：游戏会重新装备，按新词条挂被动效果
                unsafe { (self.change_item_id)(equip, entry.index, entry.item_id) };
                info!("starting passive: weapon {} (handle {:#x}) rolled {id}", entry.item_id, entry.handle);
            }
        }
    }

    /// 当前皮肤的 heroId 与初始主手 / 副手本体 ID（读内存中的表，含互换后的结果）
    fn starting_weapons(&self, game_data: usize) -> Option<(u32, [u32; 2])> {
        let skin = read::<u32>(game_data + PGD_CHARA_INIT)?;
        let row = self.data.loadouts.skins.iter().find(|s| s.id == skin)?;
        let at = |field: usize| read::<i32>(self.data.chara_init_address + row.offset + field).map(|id| (id.max(0) as u32) / 100 * 100);
        Some((row.hero_id, [at(chara_init::WEAPON_RIGHT_1)?, at(chara_init::WEAPON_LEFT_1)?]))
    }
}
