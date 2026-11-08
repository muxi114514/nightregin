//! 出击时改战技 / 改魔法 / 附加类遗物的角色开关（规则见 shared start_relics）：初始武器选择变化后按各角色实际带的武器重算，
//! 写进本机内存的 AttachEffectParam（遗物对该角色是否生效）与 SwordArtsParam（isStartSkill）。只在游戏主线程调用。

use std::collections::BTreeMap;

use log::{info, warn};
use nr_stat_alloc_shared::start_relics::GateTable;

use crate::data::StartingData;
use crate::game::memory::write;

/// carried：换过初始武器的角色（heroId）-> 实际带的武器（本体 ID）
pub fn apply(data: &StartingData, carried: &BTreeMap<u32, Vec<u32>>) {
    let Some(relics) = &data.relics else { return };
    let edits = relics.gates.plan(carried, &data.loadouts.defaults, &data.weapons);
    let base = |t| match t {
        GateTable::AttachEffect => relics.attach_address,
        GateTable::SwordArts => relics.sword_arts_address,
    };
    let failed = edits.iter().filter(|e| !write(base(e.table) + e.offset, e.value)).count();
    if failed > 0 {
        warn!("start-weapon relics: {failed} of {} writes failed", edits.len());
    }
    let changed = edits.iter().filter(|e| e.value != e.original).count();
    info!("start-weapon relics: {changed} gate bytes differ from vanilla ({} heroes with swapped weapons)", carried.len());
}
