//! 午夜的遗物检查与拦准备：钩设置准备状态的函数（0xDFA0A0），午夜开着且带遗物时不让「准备」通过并通知界面；
//! 取消准备沿用游戏自己的调用（同一函数传 0，准备管理对象取自静态 0x3C21F48）。钩子跑在游戏线程，状态用原子量。

use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use log::{info, warn};

use crate::game::addresses::ReadyAddresses;
use crate::game::memory::read_ptr;
use crate::game::equip_data::{self, VESSEL_RELIC_SLOTS};
use crate::game::relic::{self, EQUIPPED_SLOTS};
use crate::hook;

/// (准备管理对象, 1 = 准备 / 0 = 取消)
type SetReadyFn = unsafe extern "C" fn(usize, u8) -> usize;
/// 「1人出击」回调（函数对象）
type SoloSortieFn = unsafe extern "C" fn(usize) -> usize;

static TRAMPOLINE: AtomicUsize = AtomicUsize::new(0);
static SOLO_TRAMPOLINE: AtomicUsize = AtomicUsize::new(0);
static OWNER_STATIC: AtomicUsize = AtomicUsize::new(0);
static INSTALLED: AtomicBool = AtomicBool::new(false);
static BLOCK: AtomicBool = AtomicBool::new(false);
static LOCAL_READY: AtomicBool = AtomicBool::new(false);
/// 按过「1人出击」（单人不经过准备状态）
static SOLO_DEPARTING: AtomicBool = AtomicBool::new(false);
static BLOCKED: AtomicBool = AtomicBool::new(false);

pub fn any_equipped(slots: [Option<u32>; EQUIPPED_SLOTS]) -> bool {
    slots.iter().any(Option::is_some)
}

/// 遗物句柄高 4 位为 0xC（0 为空）
fn any_relic(loadouts: &[[u32; VESSEL_RELIC_SLOTS]]) -> bool {
    loadouts.iter().flatten().any(|&h| h >> 28 == 0xC && h != u32::MAX)
}

/// 遗物检查结果：带遗物的角色、读不到的 (角色, 器皿)
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RelicReport {
    pub current: Option<bool>,
    pub with_relics: Vec<u32>,
    pub unknown: Vec<(u32, u32)>,
    /// 读到的角色数（一个都没读到算未知）
    pub read: usize,
}

impl RelicReport {
    /// 有人带着就是 true；全部读到且都没带是 false；其余未知（不当成「没带」）
    pub fn verdict(&self) -> Option<bool> {
        if self.current == Some(true) || !self.with_relics.is_empty() {
            return Some(true);
        }
        (self.current == Some(false) && self.unknown.is_empty() && self.read > 0).then_some(false)
    }
}

fn summarize(current: Option<bool>, heroes: &[(u32, u32, Option<[u32; VESSEL_RELIC_SLOTS]>)]) -> RelicReport {
    let mut r = RelicReport { current, ..Default::default() };
    for &(hero, vessel, relics) in heroes {
        match relics {
            Some(l) => {
                r.read += 1;
                if any_relic(&[l]) {
                    r.with_relics.push(hero);
                }
            }
            None => r.unknown.push((hero, vessel)),
        }
    }
    r
}

/// 所有角色的遗物情况（当前角色 + 每个角色当前器皿；选角界面能换人，所以要全查）
pub fn relic_report(game_data: usize) -> RelicReport {
    summarize(relic::equipped(game_data).map(any_equipped), &equip_data::hero_loadouts(game_data))
}

/// 单人出击是否放行
fn solo_allowed(block: bool) -> bool {
    !block
}

fn should_block(ready: u8, block: bool) -> bool {
    ready != 0 && block
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReadyDecision {
    Block,
    /// 交给原函数；record = 要记下的本机准备状态（别的对象的调用不记）
    Pass { record: Option<bool> },
}

/// expected = 本机准备管理对象（读不到为 None，此时照常记录）
fn decide(owner: usize, expected: Option<usize>, ready: u8, block: bool) -> ReadyDecision {
    if expected.is_some_and(|e| e != owner) {
        return ReadyDecision::Pass { record: None };
    }
    if should_block(ready, block) {
        return ReadyDecision::Block;
    }
    ReadyDecision::Pass { record: Some(ready != 0) }
}

/// 进出出征地图时清掉：出击、匹配重置可能不经过这个函数清准备
pub fn reset_local() {
    LOCAL_READY.store(false, Ordering::Release);
    SOLO_DEPARTING.store(false, Ordering::Release);
}

fn note_solo_departure() {
    SOLO_DEPARTING.store(true, Ordering::Release);
}

/// 本机已经决定出击：按了准备或单人出击
pub fn departing() -> bool {
    local_ready() || SOLO_DEPARTING.load(Ordering::Acquire)
}

/// 需在叠加层之后调用
pub fn install(addrs: &ReadyAddresses) {
    OWNER_STATIC.store(addrs.owner_static, Ordering::Release);
    let Some(at) = addrs.set_ready else {
        warn!("midnight: set_ready not found, relic check falls back to 'not counted'");
        return;
    };
    if !hook::ensure_initialized() {
        warn!("midnight ready hook disabled: MinHook unavailable");
        return;
    }
    match hook::create(at, set_ready_detour as *mut c_void, &TRAMPOLINE) {
        Ok(h) => {
            let ok = hook::enable(&[h]);
            INSTALLED.store(ok, Ordering::Release);
            info!("midnight ready hook installed: {ok}");
        }
        Err(e) => warn!("midnight ready hook unavailable: {e}"),
    }
    // 单人出击不经过 set_ready，另钩按钮回调
    if let Some(solo) = addrs.solo_sortie {
        match hook::create(solo, solo_sortie_detour as *mut c_void, &SOLO_TRAMPOLINE) {
            Ok(h) => info!("midnight solo sortie hook installed: {}", hook::enable(&[h])),
            Err(e) => warn!("midnight solo sortie hook unavailable: {e}"),
        }
    }
}

pub fn blocking_supported() -> bool {
    INSTALLED.load(Ordering::Acquire)
}

/// 午夜开着且带遗物时置位
pub fn set_block(on: bool) {
    BLOCK.store(on, Ordering::Release);
}

#[cfg(test)]
pub fn block_set() -> bool {
    BLOCK.load(Ordering::Acquire)
}

/// 本机最近一次设置的准备状态（钩子装不上时恒为 false）
pub fn local_ready() -> bool {
    LOCAL_READY.load(Ordering::Acquire)
}

/// 自上次调用以来是否拦下过准备（界面据此弹提示）
pub fn take_blocked() -> bool {
    BLOCKED.swap(false, Ordering::AcqRel)
}

/// 取消本机准备（只能在游戏主线程调用）；不可用时返回 false
pub fn cancel_ready() -> bool {
    let at = TRAMPOLINE.load(Ordering::Acquire);
    let Some(owner) = read_ptr(OWNER_STATIC.load(Ordering::Acquire)).filter(|&o| o != 0) else { return false };
    if at == 0 {
        return false;
    }
    let original: SetReadyFn = unsafe { std::mem::transmute(at) };
    unsafe { original(owner, 0) };
    LOCAL_READY.store(false, Ordering::Release);
    info!("midnight: ready cancelled (relics equipped)");
    true
}

unsafe extern "C" fn set_ready_detour(owner: usize, ready: u8) -> usize {
    let expected = read_ptr(OWNER_STATIC.load(Ordering::Acquire)).filter(|&o| o != 0);
    match decide(owner, expected, ready, BLOCK.load(Ordering::Acquire)) {
        ReadyDecision::Block => {
            BLOCKED.store(true, Ordering::Release);
            info!("midnight: ready blocked (relics equipped)");
            return 0;
        }
        ReadyDecision::Pass { record: Some(r) } => {
            info!("midnight: set_ready({ready})");
            LOCAL_READY.store(r, Ordering::Release);
        }
        ReadyDecision::Pass { record: None } => {}
    }
    let original: SetReadyFn = unsafe { std::mem::transmute(TRAMPOLINE.load(Ordering::Acquire)) };
    unsafe { original(owner, ready) }
}

unsafe extern "C" fn solo_sortie_detour(this: usize) -> usize {
    if !solo_allowed(BLOCK.load(Ordering::Acquire)) {
        BLOCKED.store(true, Ordering::Release);
        info!("midnight: solo sortie blocked (relics equipped)");
        return 0;
    }
    info!("midnight: solo sortie");
    note_solo_departure();
    let original: SoloSortieFn = unsafe { std::mem::transmute(SOLO_TRAMPOLINE.load(Ordering::Acquire)) };
    unsafe { original(this) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relic_slots() {
        assert!(!any_equipped([None; 6]));
        assert!(any_equipped([None, None, None, None, None, Some(0xC000_0001)]));
    }

    #[test]
    fn other_owner_never_touches_local_ready() {
        assert_eq!(decide(0x20, Some(0x10), 0, false), ReadyDecision::Pass { record: None });
        assert_eq!(decide(0x20, Some(0x10), 1, true), ReadyDecision::Pass { record: None });
        assert_eq!(decide(0x10, Some(0x10), 1, true), ReadyDecision::Block);
        assert_eq!(decide(0x10, Some(0x10), 1, false), ReadyDecision::Pass { record: Some(true) });
        assert_eq!(decide(0x10, None, 0, false), ReadyDecision::Pass { record: Some(false) });
    }

    #[test]
    fn local_ready_resets() {
        let _g = crate::midnight::test_lock();
        LOCAL_READY.store(true, Ordering::Release);
        reset_local();
        assert!(!local_ready());
    }

    #[test]
    fn departing_follows_ready_and_solo_sortie() {
        let _g = crate::midnight::test_lock();
        reset_local();
        assert!(!departing());
        LOCAL_READY.store(true, Ordering::Release);
        assert!(departing());
        reset_local();
        note_solo_departure();
        assert!(departing());
        reset_local();
        assert!(!departing());
    }

    #[test]
    fn any_hero_with_relics_counts() {
        let empty = [0u32; 6];
        let one = [0, 0, 0, 0, 0, 0xC080_0251];
        assert!(!any_relic(&[empty, empty, empty]));
        assert!(any_relic(&[empty, one, empty]));
        // 不是遗物句柄的值不算（-1 等）
        assert!(!any_relic(&[[u32::MAX, 0, 0, 0, 0, 0x1234]]));
        assert!(!any_relic(&[]));
    }

    #[test]
    fn relic_report_verdicts() {
        let one = Some([0, 0, 0xC080_0251, 0, 0, 0]);
        let none = Some([0u32; 6]);
        let r = summarize(Some(false), &[(1, 1003, none), (7, 7001, one)]);
        assert_eq!((r.verdict(), r.with_relics.clone()), (Some(true), vec![7]));
        let r = summarize(Some(false), &[(1, 1003, none), (2, 19000, none)]);
        assert_eq!(r.verdict(), Some(false));
        // 有人读不到又没发现遗物：未知
        let r = summarize(Some(false), &[(1, 1003, none), (2, 19000, None)]);
        assert_eq!((r.verdict(), r.unknown.clone()), (None, vec![(2, 19000)]));
        // 当前角色带着就是带着
        assert_eq!(summarize(Some(true), &[(2, 19000, None)]).verdict(), Some(true));
    }

    #[test]
    fn solo_sortie_follows_block() {
        assert!(!solo_allowed(true));
        assert!(solo_allowed(false));
    }

    #[test]
    fn only_ready_requests_are_blocked() {
        assert!(should_block(1, true));
        assert!(!should_block(0, true));
        assert!(!should_block(1, false));
    }
}
