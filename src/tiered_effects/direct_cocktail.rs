//! 隐士1（shared::tiered_mixcraft）：持有标记时把右手三把武器的属性写进混合魔法的属性槽（game::mixcraft），技艺因此直接放魔法。
//! 放魔法时引擎会清空三槽：这时开始冷却，冷却期间槽留空（脚本让技艺没有反应，HUD 上也看得出），结束后再填回去；
//! 这一发同时算吸收了三个属性痕，依次挂吸收效果（回专注、隐士3 叠层、遗物联动都走原版那条路）。

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use log::{info, warn};
use nr_stat_alloc_shared::param::ParamFile;
use nr_stat_alloc_shared::param::layout::equip_weapon;
use nr_stat_alloc_shared::tiered_mixcraft::{DIRECT_COCKTAIL, WeaponElements, absorb_effect, slots};

use crate::game::addresses::Addresses;
use crate::game::memory::read_bytes;
use crate::game::params::ParamRepository;
use crate::game::{inventory, mixcraft};

use super::Frame;

const COOLDOWN: Duration = Duration::from_secs(8);
/// 吸收效果只持续 0.3 秒，隔开才能一个一个生效（同属性时也算三次）
const ABSORB_GAP: Duration = Duration::from_millis(350);
/// 右手 1~3 的装备槽（0 左手 1、1 右手 1、2 左手 2…）
const RIGHT_HAND_SLOTS: [usize; 3] = [1, 3, 5];

pub struct DirectCocktail {
    weapons: WeaponElements,
    module: Option<usize>,
    cycle: Cycle,
}

impl DirectCocktail {
    /// 启动时读武器表；表缺失或结构不符时为 None
    pub fn locate(repository: &ParamRepository) -> Option<Self> {
        let t = equip_weapon::TABLE;
        let (address, size) = repository.table_locations(t.struct_name).into_iter().next()?;
        let bytes = read_bytes(address, size as usize)?;
        let table = ParamFile::parse(&bytes, t.row_size).map_err(|e| warn!("{}: {e}", t.struct_name)).ok()?;
        let weapons = WeaponElements::build(&table);
        info!("direct cocktail: {} weapons with elements", weapons.len());
        (!weapons.is_empty()).then_some(Self { weapons, module: None, cycle: Cycle::default() })
    }

    /// apply：施加效果（游戏主线程）
    pub fn tick(&mut self, frame: &Frame, addrs: &Addresses, player: usize, game_data: usize, now: Instant, apply: impl FnMut(i32)) {
        if !frame.ids.contains(&DIRECT_COCKTAIL) {
            return;
        }
        let Some(module) = mixcraft::module(addrs, player) else { return };
        if self.module != Some(module) {
            // 换了角色实例，新模块的空槽不算放过魔法
            self.module = Some(module);
            self.cycle.filled = None;
        }
        let Some(current) = mixcraft::slots(module) else { return };
        if let Some(write) = self.cycle.step(now, current, self.wanted(game_data)) {
            if mixcraft::set_slots(module, write) {
                info!("direct cocktail: slots {write:?}");
            } else {
                warn!("direct cocktail: cannot write slots");
            }
        }
        self.cycle.absorb(now, apply);
    }

    /// 回到圆桌：忘掉记录
    pub fn reset(&mut self) {
        self.module = None;
        self.cycle = Cycle::default();
    }

    fn wanted(&self, game_data: usize) -> [u8; 3] {
        let equip = inventory::equip_game_data(game_data);
        slots(RIGHT_HAND_SLOTS.map(|slot| {
            inventory::equipped_index(equip, slot).and_then(|i| inventory::entry(equip, i)).map_or(0, |e| self.weapons.of(e.item_id))
        }))
    }
}

/// 填槽 → 放魔法（被引擎清空）→ 冷却 → 再填
#[derive(Default)]
struct Cycle {
    /// 上次确认填好的槽
    filled: Option<[u8; 3]>,
    ready_at: Option<Instant>,
    absorbs: VecDeque<i32>,
    next_absorb: Option<Instant>,
}

impl Cycle {
    /// current：引擎里现在的槽，wanted：按武器算出的槽；返回要写进去的槽
    fn step(&mut self, now: Instant, current: [u8; 3], wanted: [u8; 3]) -> Option<[u8; 3]> {
        if current == [0; 3]
            && let Some(cast) = self.filled.take()
        {
            info!("direct cocktail: cast {cast:?}, cooling down");
            self.ready_at = Some(now + COOLDOWN);
            self.absorbs.extend(cast.into_iter().filter_map(absorb_effect));
            self.next_absorb = Some(now);
        }
        if self.ready_at.is_some_and(|t| now < t) {
            return None;
        }
        self.ready_at = None;
        self.filled = Some(wanted);
        (current != wanted).then_some(wanted)
    }

    /// 到时间就挂下一个吸收效果
    fn absorb(&mut self, now: Instant, mut apply: impl FnMut(i32)) {
        if self.next_absorb.is_none_or(|t| now < t) {
            return;
        }
        if let Some(effect) = self.absorbs.pop_front() {
            apply(effect);
        }
        self.next_absorb = (!self.absorbs.is_empty()).then_some(now + ABSORB_GAP);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIRE: [u8; 3] = [2, 2, 2];
    const EMPTY: [u8; 3] = [0; 3];

    fn absorbed(cycle: &mut Cycle, now: Instant) -> Vec<i32> {
        let mut out = Vec::new();
        cycle.absorb(now, |id| out.push(id));
        out
    }

    #[test]
    fn fills_empty_slots_and_follows_weapon_changes() {
        let (mut c, t) = (Cycle::default(), Instant::now());
        assert_eq!(c.step(t, EMPTY, FIRE), Some(FIRE));
        assert_eq!(c.step(t, FIRE, FIRE), None);
        assert_eq!(c.step(t, FIRE, [1, 1, 3]), Some([1, 1, 3]));
    }

    #[test]
    fn overwrites_residues_absorbed_before_the_buff() {
        assert_eq!(Cycle::default().step(Instant::now(), [3, 0, 0], FIRE), Some(FIRE));
    }

    #[test]
    fn empty_slots_after_filling_start_the_cooldown() {
        let (mut c, t) = (Cycle::default(), Instant::now());
        c.step(t, EMPTY, FIRE);
        c.step(t, FIRE, FIRE);
        assert_eq!(c.step(t, EMPTY, FIRE), None);
        assert_eq!(c.step(t + COOLDOWN - Duration::from_millis(1), EMPTY, FIRE), None);
        assert_eq!(c.step(t + COOLDOWN, EMPTY, FIRE), Some(FIRE));
    }

    #[test]
    fn empty_slots_before_filling_are_not_a_cast() {
        let (mut c, t) = (Cycle::default(), Instant::now());
        assert_eq!(c.step(t, EMPTY, FIRE), Some(FIRE));
        assert!(c.ready_at.is_none() && c.absorbs.is_empty());
    }

    #[test]
    fn each_slot_of_a_cast_counts_as_one_absorption_spaced_apart() {
        let (mut c, t) = (Cycle::default(), Instant::now());
        c.step(t, EMPTY, [2, 2, 4]);
        c.step(t, EMPTY, [2, 2, 4]);
        assert_eq!(absorbed(&mut c, t), [706_000]);
        assert_eq!(absorbed(&mut c, t), []);
        assert_eq!(absorbed(&mut c, t + ABSORB_GAP), [706_000]);
        assert_eq!(absorbed(&mut c, t + ABSORB_GAP * 2), [706_003]);
        assert_eq!(absorbed(&mut c, t + ABSORB_GAP * 3), []);
    }
}
