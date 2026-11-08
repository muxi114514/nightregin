//! 武器类锻体：金4 按背包里金色武器的把数挂前 k 档增伤，紫15 每拿一次把白 / 蓝武器升到紫色（词条同锻造台强化一起升档）。

use std::time::{Duration, Instant};

use log::info;
use nr_stat_alloc_shared::passive_inherit::Rarities;
use nr_stat_alloc_shared::tiered_weapons::{GOLD, GOLD_WEAPON_DAMAGE, GOLD_WEAPON_ROWS, PURPLE_UPGRADE, purple_id};

use crate::game::inventory;
use crate::passive_upgrade;

use super::Frame;

/// 数金色武器要扫整个背包，隔一段时间一次
const SCAN_INTERVAL: Duration = Duration::from_millis(500);

type ChangeItemIdFn = unsafe extern "C" fn(usize, u32, u32) -> usize;

pub struct Weapons {
    rarities: Rarities,
    change_item_id: ChangeItemIdFn,
    next_scan: Instant,
    upgrades_seen: Option<usize>,
}

impl Weapons {
    pub fn new(rarities: Rarities, change_item_id: usize) -> Self {
        Self {
            rarities,
            change_item_id: unsafe { std::mem::transmute::<usize, ChangeItemIdFn>(change_item_id) },
            next_scan: Instant::now(),
            upgrades_seen: None,
        }
    }

    /// apply / remove：施加 / 移除效果（游戏主线程）
    pub fn tick(&mut self, frame: &Frame, game_data: usize, mut apply: impl FnMut(i32), mut remove: impl FnMut(i32)) {
        let equip = inventory::equip_game_data(game_data);
        self.upgrade(frame, equip);
        let wants = frame.ids.contains(&GOLD_WEAPON_DAMAGE);
        if !wants && !GOLD_WEAPON_ROWS.iter().any(|r| frame.ids.contains(r)) {
            return;
        }
        let now = Instant::now();
        if now < self.next_scan {
            return;
        }
        self.next_scan = now + SCAN_INTERVAL;
        let gold = if wants { self.gold_count(equip) } else { 0 };
        let (add, drop) = plan(gold, frame.ids);
        add.into_iter().for_each(&mut apply);
        drop.into_iter().for_each(&mut remove);
    }

    fn gold_count(&self, equip: usize) -> usize {
        inventory::weapons(equip).iter().filter(|e| self.rarities.current(e.item_id) == Some(GOLD)).count()
    }

    /// 紫15：份数比本局见过的最多还多时升一次（升过的已是紫色，重复执行没有影响）
    fn upgrade(&mut self, frame: &Frame, equip: usize) {
        let count = frame.ids.iter().filter(|&&id| id == PURPLE_UPGRADE).count();
        let Some(seen) = self.upgrades_seen else {
            self.upgrades_seen = Some(count);
            return;
        };
        if count <= seen {
            return;
        }
        self.upgrades_seen = Some(count);
        for e in inventory::weapons(equip) {
            let Some(new_id) = purple_id(&self.rarities, e.item_id) else { continue };
            passive_upgrade::upgrade_passives(equip, e.index, new_id);
            unsafe { (self.change_item_id)(equip, e.index, new_id) };
            info!("purple upgrade: {} -> {new_id}", e.item_id);
        }
    }

    pub fn reset(&mut self) {
        self.upgrades_seen = None;
        self.next_scan = Instant::now();
    }
}

/// 金色武器 gold 把时要挂上 / 移除的档位
fn plan(gold: usize, ids: &[i32]) -> (Vec<i32>, Vec<i32>) {
    let (mut add, mut drop) = (Vec::new(), Vec::new());
    for (k, &row) in GOLD_WEAPON_ROWS.iter().enumerate() {
        match (k < gold, ids.contains(&row)) {
            (true, false) => add.push(row),
            (false, true) => drop.push(row),
            _ => {}
        }
    }
    (add, drop)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_keeps_first_k_rows() {
        let r = GOLD_WEAPON_ROWS;
        assert_eq!(plan(2, &[]), (vec![r[0], r[1]], vec![]));
        assert_eq!(plan(1, &[r[0], r[1], r[2]]), (vec![], vec![r[1], r[2]]));
        assert_eq!(plan(9, &[r[0]]), (r[1..].to_vec(), vec![]));
        assert_eq!(plan(0, &[r[5]]), (vec![], vec![r[5]]));
    }
}
