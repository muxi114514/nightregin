//! 锻体「持有卢恩越多，攻击力越高」（regulation 里只是带状态值 shared::rune_attack::STATE 的常驻效果）。
//! 利普拉「恶魔的添翼」的攻击倍率由 exe 里一处代码（rva 0x6C3200 函数内）计算：`mov edx, 2118; call has_state` 判断持有，
//! 再按持有卢恩 `call curve` 查 CalcCorrectGraph 60。状态值与曲线都写死，所以钩这两个函数：
//! - has_state（叶函数，到处在调）：入口裸函数只放行返回到利普拉判断处的那一次，额外查新锻体的状态值，任一持有就放行，
//!   并把两者各自是否持有记到线程局部（紧接着的曲线计算在同一线程）
//! - curve（各种曲线都用它）：入口裸函数只放行第 60 号曲线那一行，按记下的持有情况合成倍率（shared::rune_attack::blend）
//! 其余调用在裸函数里直接跳回原函数，不进 Rust。曲线行地址启动时从内存里的参数表取得，取不到就两个钩子都不装
//! （只装 has_state 会让新锻体拿到利普拉全额）。

use std::cell::Cell;
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use log::{info, warn};
use nr_stat_alloc_shared::param::layout::correct_graph;
use nr_stat_alloc_shared::param::{ParamFile, field};
use nr_stat_alloc_shared::rune_attack::{self, CURVE_ID, STATE, blend};

use crate::game::memory::{read, read_bytes};
use crate::game::module::GameModule;
use crate::game::params::ParamRepository;
use crate::hook;

const HAS_STATE_RVA: usize = 0x508E20;
const CURVE_RVA: usize = 0x6D2670;
/// 利普拉判断：`mov edx, 0x846`（BA 46 08 00 00）后紧跟 `call has_state`
const LIBRA_STATE_CALL_RVA: usize = 0x6C329E;
const LIBRA_STATE_IMM: [u8; 5] = [0xBA, 0x46, 0x08, 0x00, 0x00];
/// 同一函数里随后求利普拉曲线的 `call curve`
const LIBRA_CURVE_CALL_RVA: usize = 0x6C3310;
const CALL: u8 = 0xE8;
const CALL_LEN: usize = 5;

/// (状态容器, 状态值) -> al
type HasStateFn = unsafe extern "C" fn(usize, u32) -> u8;
/// (持有卢恩, 曲线行, 行内成长值, 行内调整值) -> 倍率
type CurveFn = unsafe extern "C" fn(f32, usize, usize, usize) -> f32;

static HAS_STATE_TRAMPOLINE: AtomicUsize = AtomicUsize::new(0);
static CURVE_TRAMPOLINE: AtomicUsize = AtomicUsize::new(0);
/// has_state 返回到利普拉判断处的地址；裸函数按它过滤
static LIBRA_RETURN: AtomicUsize = AtomicUsize::new(0);
/// 第 60 号曲线行在游戏内存中的地址；裸函数按它过滤
static CURVE_ROW: AtomicUsize = AtomicUsize::new(0);
static SEEN: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy)]
struct Holders {
    libra: bool,
    ours: bool,
}

thread_local! {
    static PENDING: Cell<Option<Holders>> = const { Cell::new(None) };
}

/// 校验过的钩子目标（游戏模块在装钩子前会被交给每帧任务，所以先算好）
pub struct Targets {
    has_state: usize,
    curve: usize,
    libra_return: usize,
    curve_row: usize,
}

/// 第 60 号曲线行在游戏内存中的地址
pub fn curve_row(repository: &ParamRepository) -> Option<usize> {
    let t = correct_graph::TABLE;
    let (address, size) = repository.table_locations(t.struct_name).into_iter().next()?;
    let bytes = read_bytes(address, size as usize)?;
    let table = ParamFile::parse(&bytes, t.row_size).ok()?;
    let row = table.rows().find(|r| r.id == CURVE_ID)?;
    let points = field::f32s::<5>(row.data, correct_graph::STAGE_MAX_VAL);
    let values = field::f32s::<5>(row.data, correct_graph::STAGE_MAX_GROW_VAL);
    info!("rune attack: curve {CURVE_ID} at {:#x}, runes {points:?} -> {values:?}", address + row.offset);
    Some(address + row.offset)
}

/// 核对两处调用确实是利普拉判断与曲线计算（游戏版本不对时不装）
pub fn locate(module: &GameModule, curve_row: Option<usize>) -> Option<Targets> {
    let Some(curve_row) = curve_row else {
        warn!("rune attack disabled: CalcCorrectGraph {CURVE_ID} not found");
        return None;
    };
    let has_state = module.va(HAS_STATE_RVA);
    let curve = module.va(CURVE_RVA);
    let state_call = module.va(LIBRA_STATE_CALL_RVA);
    let imm_ok = read::<[u8; 5]>(state_call - LIBRA_STATE_IMM.len()) == Some(LIBRA_STATE_IMM);
    if !imm_ok || call_target(state_call) != Some(has_state) || call_target(module.va(LIBRA_CURVE_CALL_RVA)) != Some(curve) {
        warn!("rune attack disabled: Libra attack code mismatch (wrong game build?)");
        return None;
    }
    Some(Targets { has_state, curve, libra_return: state_call + CALL_LEN, curve_row })
}

/// 需在叠加层安装之后调用（叠加层关闭时这里自行初始化 MinHook）
pub fn install(t: Targets) {
    if !hook::ensure_initialized() {
        warn!("rune attack disabled: MinHook unavailable");
        return;
    }
    LIBRA_RETURN.store(t.libra_return, Ordering::Release);
    CURVE_ROW.store(t.curve_row, Ordering::Release);
    let hooks = hook::create(t.curve, curve_entry as *mut c_void, &CURVE_TRAMPOLINE)
        .and_then(|c| hook::create(t.has_state, has_state_entry as *mut c_void, &HAS_STATE_TRAMPOLINE).map(|s| [c, s]));
    match hooks {
        Ok(h) if hook::enable(&h) => info!("rune attack installed: state {STATE} adds {}x of Libra's bonus", rune_attack::SHARE),
        Ok(_) => warn!("rune attack: enabling hooks failed"),
        Err(e) => warn!("rune attack unavailable: {e}"),
    }
}

/// `call rel32` 的目标
fn call_target(at: usize) -> Option<usize> {
    let bytes = read::<[u8; CALL_LEN]>(at)?;
    let rel = i32::from_le_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]);
    (bytes[0] == CALL).then(|| at.wrapping_add(CALL_LEN).wrapping_add_signed(rel as isize))
}

/// 返回地址不是利普拉判断处就直接进原函数。rax 在原函数里先写后用，借来比较不影响它
#[unsafe(naked)]
unsafe extern "C" fn has_state_entry() {
    std::arch::naked_asm!(
        "mov rax, qword ptr [rsp]",
        "cmp rax, qword ptr [rip + {ret}]",
        "je {detour}",
        "jmp qword ptr [rip + {tramp}]",
        ret = sym LIBRA_RETURN,
        detour = sym has_state_detour,
        tramp = sym HAS_STATE_TRAMPOLINE,
    );
}

unsafe extern "C" fn has_state_detour(ctx: usize, state: u32) -> u8 {
    let original: HasStateFn = unsafe { std::mem::transmute(HAS_STATE_TRAMPOLINE.load(Ordering::Acquire)) };
    let holders = unsafe { Holders { libra: original(ctx, state) != 0, ours: original(ctx, u32::from(STATE)) != 0 } };
    PENDING.with(|p| p.set(Some(holders)));
    u8::from(holders.libra || holders.ours)
}

/// 不是第 60 号曲线就直接进原函数（比较只改标志位，原函数入口不读它）
#[unsafe(naked)]
unsafe extern "C" fn curve_entry() {
    std::arch::naked_asm!(
        "cmp rdx, qword ptr [rip + {row}]",
        "je {detour}",
        "jmp qword ptr [rip + {tramp}]",
        row = sym CURVE_ROW,
        detour = sym curve_detour,
        tramp = sym CURVE_TRAMPOLINE,
    );
}

unsafe extern "C" fn curve_detour(runes: f32, row: usize, grow: usize, adjust: usize) -> f32 {
    let original: CurveFn = unsafe { std::mem::transmute(CURVE_TRAMPOLINE.load(Ordering::Acquire)) };
    let multiplier = unsafe { original(runes, row, grow, adjust) };
    let Some(h) = PENDING.with(Cell::take) else { return multiplier };
    let blended = blend(multiplier, h.libra, h.ours);
    if h.ours && !SEEN.swap(true, Ordering::Relaxed) {
        info!("rune attack active: {runes} runes, Libra x{multiplier} -> x{blended} (Libra held: {})", h.libra);
    }
    blended
}
