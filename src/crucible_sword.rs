//! 执行者8（熔炉妖刀）按角色选动作。动作文件所有人都一样，每台电脑只要按「这个角色身上有没有锻体标记」选，队友也能看到；
//! 标记是会同步的效果。不能改内存里的武器行：那只在本机生效，而且同一局的另一个执行者也会被换掉。
//! - 招式表（行为图偏移类型 13 / 16）：rva 0x66CC70，角色虚函数 +0x270，(角色, 手) -> 招式表。
//!   执行者拔刀（0x3D4C10：hero 8 + 状态 609）时读妖刀行的 heroMovesetOverrideID，原值 278，带标记时换成 293（tools\tiered_moveset.ps1）；
//!   熔炉形态踏地期间（CRUCIBLE_ACTIVE）不管拔没拔刀都换成 293，绝招当帧就能播踏地（tools\tiered_crucible.lua，踏地完就摘）
//! - 战技类型（偏移类型 18，纳刀架势）：rva 0x48F6E0，选片段 a(600 + 类型)_0400xx。它只看真正装备的武器，不管拔出的妖刀，
//!   手上刀的战技不在架势片段表里就落到表头的旋转刺轮（a839）。带标记且拔刀时返回月隐的类型
//! 这两个函数可能在行为图的工作线程上调用，这里只读内存。

use std::ffi::c_void;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock, PoisonError};
use std::time::Instant;

use log::{info, warn};
use nr_stat_alloc_shared::tiered_beast::CRUCIBLE_ACTIVE;
use nr_stat_alloc_shared::tiered_params::EXECUTOR_CRUCIBLE;

use crate::game::addresses::Addresses;
use crate::game::memory::{read, read_ptr};
use crate::game::module::GameModule;
use crate::game::{sp_effect, world};
use crate::hook;

const MOVESET_RVA: usize = 0x66CC70;
/// 函数开头 15 字节（保存寄存器、开栈），校验游戏版本
const MOVESET_PROLOGUE: [u8; 15] = [0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x74, 0x24, 0x10, 0x57, 0x48, 0x83, 0xEC, 0x50];
const ART_TYPE_RVA: usize = 0x48F6E0;
/// 函数开头 14 字节（保存寄存器、建栈帧）
const ART_TYPE_PROLOGUE: [u8; 14] = [0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x7C, 0x24, 0x20, 0x55, 0x48, 0x8B, 0xEC];
/// 战技类型函数的参数是角色模块，这个字段指回角色
const MODULE_OWNER: usize = 0x8;
const CURSED_SWORD_MOVESET: u32 = 278;
const CRUCIBLE_MOVESET: u32 = 293;
/// 拔出妖刀（HKS IsDemonSwordMode 认的效果）
const CURSED_SWORD_DRAWN: i32 = 707_060;
/// 名刀月隐的战技类型（a778）
const MOONVEIL_ART_TYPE: i32 = 178;

/// 标记只在拿球时变化，选动作却很频繁，查效果链表要逐个读内存，所以按角色缓存这么久
const SEEN_MS: u64 = 500;
/// 一局最多 6 人，留点余量
const SEEN_SLOTS: usize = 8;

type MovesetFn = unsafe extern "C" fn(usize, u32) -> u32;
/// (角色模块, &输出类型) -> &输出类型
type ArtTypeFn = unsafe extern "C" fn(usize, *mut i32) -> *mut i32;

static MOVESET_TRAMPOLINE: AtomicUsize = AtomicUsize::new(0);
static ART_TYPE_TRAMPOLINE: AtomicUsize = AtomicUsize::new(0);
static ADDRS: OnceLock<Addresses> = OnceLock::new();
static START: OnceLock<Instant> = OnceLock::new();
static SEEN: Mutex<[Seen; SEEN_SLOTS]> = Mutex::new([Seen::EMPTY; SEEN_SLOTS]);
/// 已记过「带标记」日志的角色。钩子对敌人也调用，缓存位会被挤掉，不能靠缓存里的上一次结果判断要不要记
static LOGGED: Mutex<[usize; SEEN_SLOTS]> = Mutex::new([0; SEEN_SLOTS]);

#[derive(Clone, Copy)]
struct Seen {
    chr: usize,
    expires_ms: u64,
    holds: bool,
}

impl Seen {
    const EMPTY: Seen = Seen { chr: 0, expires_ms: 0, holds: false };
}

/// 记录还没过期就返回结果
fn lookup(seen: &[Seen], chr: usize, now_ms: u64) -> Option<bool> {
    seen.iter().find(|s| s.chr == chr && s.expires_ms > now_ms).map(|s| s.holds)
}

/// 写入；没有这个角色的记录时占用最早过期的位置
fn store(seen: &mut [Seen], chr: usize, holds: bool, now_ms: u64) {
    let at = seen.iter().position(|s| s.chr == chr).or_else(|| seen.iter().enumerate().min_by_key(|(_, s)| s.expires_ms).map(|(i, _)| i));
    if let Some(i) = at {
        seen[i] = Seen { chr, expires_ms: now_ms + SEEN_MS, holds };
    }
}

/// 要不要记日志：带标记的角色第一次见到记 true，记过的角色标记没了记 false，没标记的（敌人）不记
fn note_marker(logged: &mut [usize], chr: usize, holds: bool) -> Option<bool> {
    let at = logged.iter().position(|&c| c == chr);
    match (holds, at) {
        (true, None) => {
            // 满了就顶掉第一位（最多 6 个玩家，基本用不到）
            let free = logged.iter().position(|&c| c == 0).unwrap_or(0);
            logged[free] = chr;
            Some(true)
        }
        (false, Some(i)) => {
            logged[i] = 0;
            Some(false)
        }
        _ => None,
    }
}

fn seen_slots() -> MutexGuard<'static, [Seen; SEEN_SLOTS]> {
    SEEN.lock().unwrap_or_else(PoisonError::into_inner)
}

fn holds_marker(chr: usize) -> bool {
    let now_ms = START.get_or_init(Instant::now).elapsed().as_millis() as u64;
    if let Some(holds) = lookup(&*seen_slots(), chr, now_ms) {
        return holds;
    }
    let holds = sp_effect::has_all(chr, [EXECUTOR_CRUCIBLE]);
    store(&mut *seen_slots(), chr, holds, now_ms);
    // 联机排查用：队友那边要能看到这里 teammate 的 marker=true，才说明标记同步过去了
    let changed = note_marker(&mut *LOGGED.lock().unwrap_or_else(PoisonError::into_inner), chr, holds);
    if let Some(holds) = changed {
        let local = ADDRS.get().and_then(world::main_player) == Some(chr);
        info!("crucible sword: {} chr {chr:#x} marker={holds}", if local { "local" } else { "teammate" });
    }
    holds
}

pub struct Targets {
    moveset: usize,
    art_type: usize,
}

/// 校验函数开头；版本不对时不装（熔炉妖刀放原来的动作，其余不受影响）
pub fn locate(module: &GameModule) -> Option<Targets> {
    let targets = Targets { moveset: module.va(MOVESET_RVA), art_type: module.va(ART_TYPE_RVA) };
    if read::<[u8; 15]>(targets.moveset) != Some(MOVESET_PROLOGUE) || read::<[u8; 14]>(targets.art_type) != Some(ART_TYPE_PROLOGUE) {
        warn!("crucible sword disabled: function mismatch (wrong game build?)");
        return None;
    }
    Some(targets)
}

/// 需在叠加层安装之后调用（叠加层关闭时这里自行初始化 MinHook）
pub fn install(targets: Targets, addrs: Addresses) {
    if !hook::ensure_initialized() {
        warn!("crucible sword disabled: MinHook unavailable");
        return;
    }
    let _ = ADDRS.set(addrs);
    let moveset = hook::create(targets.moveset, moveset_detour as *mut c_void, &MOVESET_TRAMPOLINE);
    let art_type = hook::create(targets.art_type, art_type_detour as *mut c_void, &ART_TYPE_TRAMPOLINE);
    let hooks = match (moveset, art_type) {
        (Ok(m), Ok(a)) => [m, a],
        (m, a) => {
            warn!("crucible sword unavailable: {:?} {:?}", m.err(), a.err());
            return;
        }
    };
    if hook::enable(&hooks) {
        info!("crucible sword installed");
    } else {
        warn!("crucible sword: enabling hooks failed");
    }
}

unsafe extern "C" fn moveset_detour(chr: usize, hand: u32) -> u32 {
    let original: MovesetFn = unsafe { std::mem::transmute(MOVESET_TRAMPOLINE.load(Ordering::Acquire)) };
    let moveset = unsafe { original(chr, hand) };
    if !holds_marker(chr) {
        return moveset;
    }
    // 熔炉形态变身中不缓存：按绝招当帧就要选到踏地片段（只查带锻体的角色）
    if moveset == CURSED_SWORD_MOVESET || sp_effect::has_all(chr, [CRUCIBLE_ACTIVE]) { CRUCIBLE_MOVESET } else { moveset }
}

unsafe extern "C" fn art_type_detour(module: usize, out: *mut i32) -> *mut i32 {
    let original: ArtTypeFn = unsafe { std::mem::transmute(ART_TYPE_TRAMPOLINE.load(Ordering::Acquire)) };
    let result = unsafe { original(module, out) };
    let owner = read_ptr(module + MODULE_OWNER).unwrap_or(0);
    if owner != 0 && holds_marker(owner) && sp_effect::has_all(owner, [CURSED_SWORD_DRAWN]) {
        unsafe { out.write(MOONVEIL_ART_TYPE) };
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_expires() {
        let mut seen = [Seen::EMPTY; 2];
        assert_eq!(lookup(&seen, 0x100, 0), None);
        store(&mut seen, 0x100, true, 0);
        assert_eq!(lookup(&seen, 0x100, SEEN_MS - 1), Some(true));
        assert_eq!(lookup(&seen, 0x100, SEEN_MS), None);
        store(&mut seen, 0x100, false, SEEN_MS);
        assert_eq!(lookup(&seen, 0x100, SEEN_MS), Some(false));
    }

    #[test]
    fn marker_logged_once_per_change_and_never_for_enemies() {
        let mut logged = [0usize; 2];
        // 敌人（没标记）一直不记
        assert_eq!(note_marker(&mut logged, 0x900, false), None);
        assert_eq!(note_marker(&mut logged, 0x900, false), None);
        // 带标记的角色只在第一次记，缓存被挤掉后重查也不再记
        assert_eq!(note_marker(&mut logged, 0x100, true), Some(true));
        assert_eq!(note_marker(&mut logged, 0x100, true), None);
        // 标记没了记一次，之后不再记
        assert_eq!(note_marker(&mut logged, 0x100, false), Some(false));
        assert_eq!(note_marker(&mut logged, 0x100, false), None);
        // 位满了顶掉第一位
        assert_eq!(note_marker(&mut logged, 0x200, true), Some(true));
        assert_eq!(note_marker(&mut logged, 0x300, true), Some(true));
        assert_eq!(note_marker(&mut logged, 0x400, true), Some(true));
        assert_eq!(logged, [0x400, 0x300]);
    }

    #[test]
    fn full_cache_replaces_the_oldest() {
        let mut seen = [Seen::EMPTY; 2];
        store(&mut seen, 0x100, true, 0);
        store(&mut seen, 0x200, true, 10);
        store(&mut seen, 0x300, false, 20);
        assert_eq!(lookup(&seen, 0x100, 20), None);
        assert_eq!(lookup(&seen, 0x200, 20), Some(true));
        assert_eq!(lookup(&seen, 0x300, 20), Some(false));
    }
}
