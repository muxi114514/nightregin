//! 局内武器词条继承（游戏线程）：来源武器的三个词条槽整把覆盖到目标武器上，来源随后消失。
//! 步骤：重新读取并核对两把武器 → 档位规则（shared passive_inherit）→ 写目标的词条槽 → 原 ID 重设一次让游戏按新词条重新装备
//! → 原生移除来源（装备着会先卸下）→ 核对它已不在背包；移除失败时把目标恢复原样。

mod entry;
mod task;
mod view;

pub use entry::{REQUEST_FLAG, block_queue, on_flag};
pub use entry::request as request_panel;
pub use task::InheritTask;
pub use view::{InheritOutcome, InheritRequest, InheritView, InheritWeapon, WeaponPick};

use std::sync::Arc;

use log::{error, info};
use nr_stat_alloc_shared::passive_inherit::InheritError;

use crate::data::{InheritData, weapon_name};
use crate::game::equip_ops::EquipOps;
use crate::game::gaitem::{Gaitems, WeaponIns};
use crate::game::inventory::{self, Entry};
use crate::tr;

pub struct PassiveInherit {
    data: Arc<InheritData>,
    gaitems: Gaitems,
    ops: EquipOps,
}

impl PassiveInherit {
    pub fn new(data: Arc<InheritData>, gaitems: Gaitems, ops: EquipOps) -> Self {
        Self { data, gaitems, ops }
    }

    /// 6 个武器槽里能参与继承的武器（空手、箭矢不列出）
    pub fn view(&self, equip: usize) -> InheritView {
        InheritView { slots: std::array::from_fn(|slot| self.weapon_in_slot(equip, slot)) }
    }

    /// 返回结果说明（成功 / 失败都已写日志）
    pub fn execute(&self, equip: usize, req: &InheritRequest) -> Result<String, String> {
        let result = self.run(equip, req);
        match &result {
            Ok(text) => info!("passive inherit: {text}"),
            Err(text) => error!("passive inherit failed: {text}"),
        }
        result
    }

    fn run(&self, equip: usize, req: &InheritRequest) -> Result<String, String> {
        let (source, target) = (req.source, req.target);
        if source.handle == target.handle {
            return Err(tr!("请选择两把不同的武器", "Select two different weapons"));
        }
        let stale = || tr!("武器已变化，请重新选择", "The weapons changed; please select again");
        let (from, _) = self.resolve(equip, source).ok_or_else(stale)?;
        let (to, to_ins) = self.resolve(equip, target).ok_or_else(stale)?;
        self.data.rarities.check(from.item_id, to.item_id).map_err(describe)?;
        let (passives, before) = (source.passives, target.passives);
        if !to_ins.set_passive_slots(passives) {
            return Err(tr!("写入武器失败，未改动", "Writing the weapon failed; nothing changed"));
        }
        self.ops.refresh(equip, to.index, to.item_id);
        let left = self.ops.remove_one(equip, from.index);
        if inventory::entry(equip, from.index).is_some_and(|e| e.handle == source.handle) {
            // 来源还在：把目标恢复原样（重新装备会换实例，按条目编号重新找）
            let restored = self.weapon_at(equip, to.index).is_some_and(|w| w.set_passive_slots(before));
            if restored {
                self.ops.refresh(equip, to.index, to.item_id);
            }
            error!("passive inherit: weapon {} (index {}) still present after remove (left {left}), target restored: {restored}", from.item_id, from.index);
            return Err(tr!("移除来源武器失败，目标武器已恢复原样", "Removing the source weapon failed; the target was restored"));
        }
        let after = self.weapon_at(equip, to.index).and_then(|w| w.passive_slots());
        info!(
            "passive inherit: {} (index {}, handle {:#x}) {passives:?} -> {} (index {}) {before:?}, now {after:?}, source left {left}",
            from.item_id, from.index, source.handle, to.item_id, to.index
        );
        let (from_name, to_name) = (weapon_name(from.item_id), weapon_name(to.item_id));
        Ok(tr!("词条已继承：{from_name} → {to_name}", "Passives inherited: {from_name} → {to_name}"))
    }

    fn weapon_in_slot(&self, equip: usize, slot: usize) -> Option<InheritWeapon> {
        let entry = inventory::entry(equip, inventory::equipped_index(equip, slot)?)?;
        self.data.rarities.tier(entry.item_id)?;
        let passives = self.gaitems.weapon(entry.handle)?.passive_slots()?;
        Some(InheritWeapon { pick: WeaponPick { index: entry.index, handle: entry.handle, passives }, item_id: entry.item_id })
    }

    /// 条目编号、句柄、词条槽都对得上的武器
    fn resolve(&self, equip: usize, pick: WeaponPick) -> Option<(Entry, WeaponIns)> {
        let entry = inventory::entry(equip, pick.index).filter(|e| e.handle == pick.handle)?;
        let ins = self.gaitems.weapon(pick.handle)?;
        (ins.passive_slots()? == pick.passives).then_some((entry, ins))
    }

    fn weapon_at(&self, equip: usize, index: u32) -> Option<WeaponIns> {
        self.gaitems.weapon(inventory::entry(equip, index)?.handle)
    }
}

/// 档位名：0 白、1 蓝、2 紫 / 红 / 金
pub fn tier_name(tier: u8) -> String {
    match tier {
        0 => tr!("白", "white"),
        1 => tr!("蓝", "blue"),
        _ => tr!("紫 / 红 / 金", "purple / red / gold"),
    }
}

fn describe(e: InheritError) -> String {
    match e {
        InheritError::NotInheritable(id) => {
            let name = weapon_name(id);
            tr!("{name} 不是能带词条的武器（空手或箭矢）", "{name} cannot carry passives (unarmed or ammo)")
        }
        InheritError::LowerTier { source, target } => {
            let (s, t) = (tier_name(source), tier_name(target));
            tr!("来源武器（{s}）的档位低于目标武器（{t}），不能继承", "The source ({s}) is a lower tier than the target ({t}); cannot inherit")
        }
    }
}
