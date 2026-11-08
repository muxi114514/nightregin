//! 三选一选项刷新（配置 [tiered_buffs] orb_refresh_costs）：界面打开时叠加层显示按钮，点一下按原掉落权重换掉这个球里的锻体选项，武器不动。
//! 每个球的次数关掉重开也算数，球被选走后记录随之清掉。渲染线程只发点击，读写都在游戏线程；规则见 shared::orb_refresh。

use std::collections::HashMap;
use std::sync::Arc;

use log::{info, warn};
use nr_stat_alloc_shared::orb_refresh::{OptionSlot, Pools, goods_id, goods_item, price};
use nr_stat_alloc_shared::param::ParamFile;
use nr_stat_alloc_shared::param::layout::{Table, item_lot, item_table, permanent_buff, sp_effect};

use crate::bridge::{OrbRefreshView, UiBridge};
use crate::data::GameData;
use crate::game::addresses::{Addresses, PrivateItemAddresses};
use crate::game::memory::read_bytes;
use crate::game::params::ParamRepository;
use crate::game::private_item::{ChoiceDialog, ChoiceOption, orb_exists};
use crate::game::runes;
use crate::rng::XorShift;

use super::Frame;

/// 启动时读掉落行、掉落表、锻体与效果表；任一缺失时为 None（按钮不显示）
pub fn locate_pools(repository: &ParamRepository) -> Option<Pools> {
    let bytes_of = |t: Table| -> Vec<Vec<u8>> { repository.table_locations(t.struct_name).into_iter().filter_map(|(at, size)| read_bytes(at, size as usize)).collect() };
    let lot_bytes = bytes_of(item_lot::TABLE);
    let (table_bytes, buff_bytes, effect_bytes) = (bytes_of(item_table::TABLE), bytes_of(permanent_buff::TABLE), bytes_of(sp_effect::TABLE));
    fn parse(bytes: &[u8], t: Table) -> Option<ParamFile<'_>> {
        ParamFile::parse(bytes, t.row_size).map_err(|e| warn!("orb refresh: {}: {e}", t.struct_name)).ok()
    }
    let lots: Vec<ParamFile> = lot_bytes.iter().filter_map(|b| parse(b, item_lot::TABLE)).collect();
    let tables = parse(table_bytes.first()?, item_table::TABLE)?;
    let buffs = parse(buff_bytes.first()?, permanent_buff::TABLE)?;
    let effects = parse(effect_bytes.first()?, sp_effect::TABLE)?;
    let pools = Pools::build(&lots, &tables, &buffs, &effects);
    info!("orb refresh: {} lot tables, {} item tables with buffs", lots.len(), pools.tables_loaded());
    Some(pools)
}

/// 界面当前的选项按能否刷新分开
struct OrbOptions {
    id: u32,
    slots: Vec<(ChoiceOption, OptionSlot)>,
    /// 不刷新的道具选项：新抽的也不能和它们重复
    others: Vec<u32>,
}

pub struct OrbRefresh {
    private: PrivateItemAddresses,
    pools: Pools,
    data: Arc<GameData>,
    costs: Vec<u32>,
    bridge: Arc<UiBridge>,
    /// 球编号 → 已刷新次数
    used: HashMap<u32, u32>,
    showing: bool,
}

impl OrbRefresh {
    pub fn new(private: PrivateItemAddresses, pools: Pools, data: Arc<GameData>, costs: Vec<u32>, bridge: Arc<UiBridge>) -> Self {
        Self { private, pools, data, costs, bridge, used: HashMap::new(), showing: false }
    }

    pub fn tick(&mut self, frame: &Frame, addrs: &Addresses, chara_init: u32, game_data: usize, rng: &mut XorShift) {
        let clicked = self.bridge.take_orb_refresh_click();
        let hero = self.data.hero_of_chara(chara_init).map(|h| h as u8 + 1);
        let Some((dialog, hero)) = ChoiceDialog::showing(addrs, &self.private).zip(hero) else {
            return self.on_closed();
        };
        self.showing = true;
        let Some(orb) = self.options(&dialog, hero) else {
            return self.bridge.set_orb_refresh(None);
        };
        if clicked && self.refresh(&dialog, &orb, frame, hero, game_data, rng) {
            *self.used.entry(orb.id).or_default() += 1;
        }
        let used = self.used.get(&orb.id).copied().unwrap_or(0);
        let cost = price(&self.costs, used);
        self.bridge.set_orb_refresh(Some(OrbRefreshView {
            remaining: (self.costs.len() as u32).saturating_sub(used),
            cost: cost.unwrap_or(0),
            affordable: cost.is_some_and(|c| frame.runes >= c),
        }));
    }

    /// 离开出征：记录全部作废（编号下一局会重复）
    pub fn reset(&mut self) {
        self.used.clear();
        self.showing = false;
        self.bridge.set_orb_refresh(None);
    }

    /// 界面刚关时清掉被选走的球，记录不会越积越多
    fn on_closed(&mut self) {
        if self.showing {
            self.showing = false;
            self.bridge.set_orb_refresh(None);
            let private = &self.private;
            self.used.retain(|&orb, _| orb_exists(private, orb));
        }
    }

    /// 没有可刷新的锻体（全是武器等）或读不到时为 None，不显示按钮
    fn options(&self, dialog: &ChoiceDialog, hero: u8) -> Option<OrbOptions> {
        let mut orb = OrbOptions { id: dialog.orb()?, slots: Vec::new(), others: Vec::new() };
        for option in dialog.options(&self.private)? {
            let Some(goods) = goods_id(option.item) else { continue };
            let slot = OptionSlot { lot: option.lot, buff: goods };
            if self.pools.is_buff(goods) && self.pools.drops(slot, hero) {
                orb.slots.push((option, slot));
            } else {
                orb.others.push(goods);
            }
        }
        (!orb.slots.is_empty()).then_some(orb)
    }

    /// 返回这次刷新是否生效（一个选项都换不了时不收钱也不算次数）
    fn refresh(&self, dialog: &ChoiceDialog, orb: &OrbOptions, frame: &Frame, hero: u8, game_data: usize, rng: &mut XorShift) -> bool {
        let used = self.used.get(&orb.id).copied().unwrap_or(0);
        let Some(cost) = price(&self.costs, used) else { return false };
        let Some(held) = runes::held(game_data).filter(|&r| r >= cost) else { return false };
        let slots: Vec<OptionSlot> = orb.slots.iter().map(|(_, s)| *s).collect();
        let plan = self.pools.plan(&slots, &orb.others, hero, frame.ids, &mut || rng.next());
        if plan.iter().all(Option::is_none) {
            warn!("orb refresh: orb {:#x}: no other buff can be drawn", orb.id);
            return false;
        }
        if cost > 0 && !runes::set(game_data, held - cost) {
            warn!("orb refresh: cannot write runes");
            return false;
        }
        let mut replaced = Vec::with_capacity(plan.len());
        for ((option, slot), new) in orb.slots.iter().zip(&plan) {
            if let Some(new) = *new {
                if !dialog.replace(option, goods_item(new)) {
                    warn!("orb refresh: cannot write option {}", slot.buff);
                }
                replaced.push((slot.buff, new));
            }
        }
        let redrawn = dialog.redraw();
        info!("orb refresh: orb {:#x} refresh {} cost {cost}: {replaced:?} redraw={redrawn}", orb.id, used + 1);
        true
    }
}
