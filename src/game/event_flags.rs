//! 写事件旗标的钩子与分发：CSEventFlagMan::set_event_flag 与发给队友的同步队列各钩一次，
//! 词条继承、午夜结算各注册处理函数；任一处理函数吞掉就不调原函数。
//! 队友发来的旗标经 SetNetSyncEventFlag 以「不请求同步」再进 set_event_flag，所以也会经过这里。钩子跑在游戏线程。

use std::ffi::c_void;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use log::{info, warn};

use super::addresses::EventFlagAddresses;
use crate::hook;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlagVerdict {
    Pass,
    /// 不调原函数（自定义旗标、块不存在）
    Swallow,
}

/// (旗标, 值, 请求同步)：本机脚本写入请求同步 ≠ 0，队友同步过来的为 0
pub type FlagHandler = fn(u32, u8, u8) -> FlagVerdict;
/// 返回 true = 不放进发给队友的同步队列
pub type QueueFilter = fn(u32) -> bool;

/// (管理器, &旗标, 值, 调试上下文, 请求同步, 日志)
type SetEventFlagFn = unsafe extern "C" fn(usize, *const u32, u8, usize, u8, u8);
/// (管理器, &旗标, 值)
type QueueNetSyncFn = unsafe extern "C" fn(usize, *const u32, u8);

static HANDLERS: OnceLock<Vec<FlagHandler>> = OnceLock::new();
static FILTERS: OnceLock<Vec<QueueFilter>> = OnceLock::new();
static SET_FLAG_TRAMPOLINE: AtomicUsize = AtomicUsize::new(0);
static QUEUE_TRAMPOLINE: AtomicUsize = AtomicUsize::new(0);
static SEEN: AtomicBool = AtomicBool::new(false);

/// 各钩子是否装上
#[derive(Debug, Clone, Copy, Default)]
pub struct InstallReport {
    pub local: bool,
    pub queue: bool,
}

/// 每个处理函数都调用（观察者都要看到），任一吞掉即吞掉
pub fn dispatch(handlers: &[FlagHandler], id: u32, value: u8, sync: u8) -> FlagVerdict {
    let mut verdict = FlagVerdict::Pass;
    for h in handlers {
        if h(id, value, sync) == FlagVerdict::Swallow {
            verdict = FlagVerdict::Swallow;
        }
    }
    verdict
}

pub fn queue_blocked(filters: &[QueueFilter], id: u32) -> bool {
    filters.iter().any(|f| f(id))
}

/// 需在叠加层之后调用（叠加层关闭时这里自行初始化 MinHook）；只生效一次
pub fn install(addrs: &EventFlagAddresses, handlers: Vec<FlagHandler>, filters: Vec<QueueFilter>) -> InstallReport {
    let mut report = InstallReport::default();
    if HANDLERS.set(handlers).is_err() || FILTERS.set(filters).is_err() {
        warn!("event flag hooks already installed");
        return report;
    }
    if !hook::ensure_initialized() {
        warn!("event flag hooks disabled: MinHook unavailable");
        return report;
    }
    let mut hooks = Vec::new();
    if let Some(at) = addrs.set_event_flag {
        match hook::create(at, set_flag_detour as *mut c_void, &SET_FLAG_TRAMPOLINE) {
            Ok(h) => hooks.push((h, true)),
            Err(e) => warn!("set_event_flag not hooked: {e}"),
        }
    }
    let queue_needed = FILTERS.get().is_some_and(|f| !f.is_empty());
    if let (Some(at), true) = (addrs.queue_net_sync, queue_needed) {
        match hook::create(at, queue_detour as *mut c_void, &QUEUE_TRAMPOLINE) {
            Ok(h) => hooks.push((h, false)),
            Err(e) => warn!("net sync queue not hooked: {e}"),
        }
    }
    let (list, kinds): (Vec<_>, Vec<_>) = hooks.into_iter().unzip();
    if !list.is_empty() && hook::enable(&list) {
        report.local = kinds.contains(&true);
        report.queue = kinds.contains(&false);
    } else if !list.is_empty() {
        warn!("event flag hooks: enabling failed");
    }
    info!("event flag hooks: local {}, net sync queue {}, {} handlers", report.local, report.queue, HANDLERS.get().map_or(0, Vec::len));
    report
}

fn flag_id(flag: *const u32) -> u32 {
    if flag.is_null() { 0 } else { unsafe { flag.read() } }
}

unsafe extern "C" fn set_flag_detour(man: usize, flag: *const u32, value: u8, ctx: usize, sync: u8, log: u8) {
    let id = flag_id(flag);
    if !SEEN.swap(true, Ordering::Relaxed) {
        info!("set_event_flag hook active (first flag {id})");
    }
    if dispatch(HANDLERS.get().map_or(&[], Vec::as_slice), id, value, sync) == FlagVerdict::Swallow {
        return;
    }
    let original: SetEventFlagFn = unsafe { std::mem::transmute(SET_FLAG_TRAMPOLINE.load(Ordering::Acquire)) };
    unsafe { original(man, flag, value, ctx, sync, log) }
}

unsafe extern "C" fn queue_detour(man: usize, flag: *const u32, value: u8) {
    if queue_blocked(FILTERS.get().map_or(&[], Vec::as_slice), flag_id(flag)) {
        return;
    }
    let original: QueueNetSyncFn = unsafe { std::mem::transmute(QUEUE_TRAMPOLINE.load(Ordering::Acquire)) };
    unsafe { original(man, flag, value) }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    static CALLS: AtomicUsize = AtomicUsize::new(0);

    fn count_pass(_: u32, _: u8, _: u8) -> FlagVerdict {
        CALLS.fetch_add(1, Ordering::SeqCst);
        FlagVerdict::Pass
    }

    fn swallow_7(id: u32, _: u8, _: u8) -> FlagVerdict {
        if id == 7 { FlagVerdict::Swallow } else { FlagVerdict::Pass }
    }

    fn block_9(id: u32) -> bool {
        id == 9
    }

    #[test]
    fn dispatch_calls_every_handler() {
        let before = CALLS.load(Ordering::SeqCst);
        // 吞掉的处理函数排在前面，后面的观察者照样收到
        let verdict = dispatch(&[swallow_7, count_pass, count_pass], 7, 1, 0);
        assert_eq!(verdict, FlagVerdict::Swallow);
        assert_eq!(CALLS.load(Ordering::SeqCst) - before, 2);
    }

    #[test]
    fn any_swallow_swallows() {
        assert_eq!(dispatch(&[count_pass, swallow_7], 7, 1, 1), FlagVerdict::Swallow);
        assert_eq!(dispatch(&[count_pass, swallow_7], 8, 1, 1), FlagVerdict::Pass);
    }

    #[test]
    fn no_handler_passes() {
        assert_eq!(dispatch(&[], 7, 1, 1), FlagVerdict::Pass);
    }

    #[test]
    fn any_filter_blocks_queue() {
        assert!(queue_blocked(&[block_9], 9));
        assert!(!queue_blocked(&[block_9], 10));
        assert!(!queue_blocked(&[], 9));
    }
}
