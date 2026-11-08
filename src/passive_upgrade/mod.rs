//! 锻造石强化武器时同步升级词条（配置 [weapon_passive]）。
//! 钩住 EquipGameData::change_item_id：强化提交按背包条目编号把武器 ID +1（强化等级），武器已装备时随后重新装备。
//! 在原函数执行前把武器实例里的词条换成上位档：界面直接读词条槽，重新装备时游戏按新词条重建缓存、换掉被动效果（实机验证）。
//! 只处理强化提交发起的调用：另一个调用点（0x20E170）是角色初始武器随等级自动升级，不属于锻造石强化。
//! 不钩强化提交本身：整合包的 RestoredLegendaryStones 已钩它并校验返回地址，叠加钩子会让它的金色强化失效。
//! 钩子跑在游戏主线程（菜单），状态在安装前一次性建好，之后只读。

mod dump;

use std::ffi::c_void;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use log::{error, info, warn};
use nr_stat_alloc_shared::passive_tiers::PassiveTiers;

use crate::config::WeaponPassiveConfig;
use crate::game::addresses::GaitemAddresses;
use crate::game::gaitem::{Gaitems, PASSIVE_COUNT, WeaponIns};
use crate::game::inventory;
use crate::hook;

/// 一次强化最多提升的等级（普通武器 +0 → +3）
const MAX_STEPS: u32 = 3;
const ITEM_ID_MASK: u32 = 0x0FFF_FFFF;

type ChangeItemIdFn = unsafe extern "C" fn(usize, u32, u32) -> usize;

static STATE: OnceLock<State> = OnceLock::new();
static TRAMPOLINE: AtomicUsize = AtomicUsize::new(0);
/// 处理中 panic 后本次会话停用（原函数照常执行）
static DISABLED: AtomicBool = AtomicBool::new(false);

struct State {
    tiers: PassiveTiers,
    gaitems: Gaitems,
    dump: bool,
    reinforce_return: usize,
}

/// 需在叠加层安装之后调用（叠加层关闭时这里自行初始化 MinHook）
pub fn install(cfg: &WeaponPassiveConfig, tiers: Option<PassiveTiers>, addrs: &GaitemAddresses) {
    if !cfg.enabled {
        return;
    }
    let Some(tiers) = tiers.filter(|t| !t.is_empty()) else {
        warn!("weapon passive upgrade disabled: AttachEffectParam unavailable");
        return;
    };
    let tier_count = tiers.len();
    let state = State { tiers, gaitems: Gaitems::new(addrs), dump: cfg.dump, reinforce_return: addrs.reinforce_return };
    if !hook::ensure_initialized() {
        warn!("weapon passive upgrade disabled: MinHook unavailable");
        return;
    }
    if STATE.set(state).is_err() {
        return;
    }
    match hook::create(addrs.change_item_id, entry as *mut c_void, &TRAMPOLINE) {
        Ok(h) if hook::enable(std::slice::from_ref(&h)) => {
            info!("weapon passive upgrade installed: {tier_count} passives with upper tiers, dump={}", cfg.dump);
        }
        Ok(_) => warn!("weapon passive upgrade: enabling hook failed"),
        Err(e) => warn!("weapon passive upgrade unavailable: {e}"),
    }
}

/// 其他功能自己提高强化等级时（三档锻体升紫），在调 change_item_id 之前调用：词条按提升的等级升档，同锻造台强化
pub fn upgrade_passives(equip_game_data: usize, index: u32, new_id: u32) {
    if let Some(state) = STATE.get()
        && !DISABLED.load(Ordering::Relaxed)
        && catch_unwind(AssertUnwindSafe(|| state.on_reinforce(equip_game_data, index, new_id))).is_err()
    {
        DISABLED.store(true, Ordering::Relaxed);
        error!("weapon passive upgrade panicked; disabled for this session");
    }
}

/// 入口桩：把返回地址（[rsp]）放进 r9 作为第 4 个参数，栈不动，detour 直接返回到游戏。
/// change_item_id 只用 rcx / edx / r8d，r9 在其中先写后用（反汇编确认），借用它不影响原函数
#[unsafe(naked)]
unsafe extern "C" fn entry() {
    std::arch::naked_asm!("mov r9, [rsp]", "jmp {detour}", detour = sym detour);
}

/// this：EquipGameData；index：背包条目编号（不是实例句柄）
unsafe extern "C" fn detour(this: usize, index: u32, new_id: u32, caller: usize) -> usize {
    if let Some(state) = STATE.get()
        && caller == state.reinforce_return
        && !DISABLED.load(Ordering::Relaxed)
        && catch_unwind(AssertUnwindSafe(|| state.on_reinforce(this, index, new_id))).is_err()
    {
        DISABLED.store(true, Ordering::Relaxed);
        error!("weapon passive upgrade panicked; disabled for this session");
    }
    let original: ChangeItemIdFn = unsafe { std::mem::transmute(TRAMPOLINE.load(Ordering::Acquire)) };
    unsafe { original(this, index, new_id) }
}

impl State {
    fn on_reinforce(&self, equip_game_data: usize, index: u32, new_id: u32) {
        let Some(handle) = inventory::gaitem_handle(equip_game_data, index) else {
            warn!("reinforce: inventory index {index:#x} has no item");
            return;
        };
        let Some(weapon) = self.gaitems.weapon(handle) else {
            warn!("reinforce: inventory index {index:#x} handle {handle:#x} is not a weapon instance");
            return;
        };
        let Some(old_id) = weapon.item_id() else { return };
        let new_id = new_id & ITEM_ID_MASK;
        let Some(steps) = reinforce_steps(old_id, new_id) else {
            info!("reinforce {old_id} -> {new_id}: not a level increase, passives unchanged");
            return;
        };
        let before = if self.dump { weapon.snapshot() } else { None };
        let changes = self.upgrade_slots(weapon, steps);
        let summary = if changes.is_empty() {
            "no upgradable passives".to_string()
        } else {
            changes.iter().map(|(slot, from, to)| format!("slot {slot} {from}->{to}")).collect::<Vec<_>>().join(", ")
        };
        info!("reinforce {old_id} -> {new_id} (+{steps}, handle {handle:#x}): {summary}");
        if let Some(bytes) = before {
            info!("weapon {:#x} before: {}", weapon.address(), dump::annotate(&bytes, &self.tiers));
        }
    }

    /// 返回 (槽位, 原词条, 新词条)；空槽与没有上位档的词条不动
    fn upgrade_slots(&self, weapon: WeaponIns, steps: u32) -> Vec<(usize, u32, u32)> {
        (0..PASSIVE_COUNT)
            .filter_map(|slot| {
                let id = weapon.passive(slot)?;
                let up = self.tiers.upgrade(id, steps);
                (up != id && weapon.set_passive(slot, up)).then_some((slot, id, up))
            })
            .collect()
    }
}

/// 同一把武器的强化等级提升量：物品 ID 只有末位（强化等级）变化
fn reinforce_steps(old: u32, new: u32) -> Option<u32> {
    let steps = new.checked_sub(old)?;
    (old / 100 == new / 100 && (1..=MAX_STEPS).contains(&steps)).then_some(steps)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_reinforce_level_increase() {
        assert_eq!(reinforce_steps(9040000, 9040001), Some(1));
        assert_eq!(reinforce_steps(1000500, 1000502), Some(2));
        assert_eq!(reinforce_steps(1000503, 1000502), None);
        assert_eq!(reinforce_steps(1000500, 1000500), None);
        assert_eq!(reinforce_steps(1000500, 1000600), None);
        assert_eq!(reinforce_steps(1000099, 1000100), None);
    }
}
