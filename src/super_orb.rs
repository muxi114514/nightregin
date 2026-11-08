//! 超级系列（目前只有阎魔刀，EquipParamWeapon 稀有度 6）的掉落球用专属颜色。包里的 FXR 6104 / 6105 是原版
//! 6000（分组球）/ 572（单件太刀）的超集：外部值 4000 多一档 6 → 状态 9（血红），其余外部值不变（2026-11 反汇编核对）。
//! - 分组球：rva 0x579FF0 (链头) -> u8 外部值，沿 +0x10 按稀有度（+0xCA 低 4 位）0/1/2/3/9 的优先级选色，6 不在表里，
//!   所以阎魔刀和金护符同组时是金球。钩子在组里有稀有度 6 时返回 6；两个调用方都只把结果存成外部值，不查表。
//! - 单件掉落：rva 0x57C6C0 (对象, &输出) -> &输出，写入特效号（太刀 572）。稀有度 6 时换成 6105，普通太刀不动。
//! 特效号不存在时球不显示，所以只在 GameSystemCommonParam 第 0 行的分组球特效号为 6104（包里带了这些特效）时装。
//! 钩子在游戏线程上调用：安全读、不加锁、不写日志。

use std::ffi::c_void;
use std::sync::atomic::{AtomicUsize, Ordering};

use log::{info, warn};
use nr_stat_alloc_shared::param::layout::game_system_common;
use nr_stat_alloc_shared::param::{ParamFile, field};

use crate::game::memory::{read, read_bytes, read_ptr};
use crate::game::module::GameModule;
use crate::game::params::ParamRepository;
use crate::hook;

const GROUP_COLOR_RVA: usize = 0x579FF0;
const SINGLE_FXR_RVA: usize = 0x57C6C0;
/// 校验的代码：两个函数开头，以及分组函数里读稀有度字节、取同组下一个的指令（确认 +0xCA / +0x10 没变）
const SIGNATURES: [(usize, &[u8]); 4] = [
    // sub rsp, 8; mov r9, rcx; test rcx, rcx; je
    (GROUP_COLOR_RVA, &[0x48, 0x83, 0xEC, 0x08, 0x4C, 0x8B, 0xC9, 0x48, 0x85, 0xC9, 0x0F, 0x84, 0xE8, 0x00, 0x00, 0x00]),
    // movsx eax, byte ptr [r9 + 0xCA]
    (0x57A010, &[0x41, 0x0F, 0xBE, 0x81, 0xCA, 0x00, 0x00, 0x00]),
    // mov r9, qword ptr [r9 + 0x10]
    (0x57A07E, &[0x4D, 0x8B, 0x49, 0x10]),
    // 保存寄存器、开栈、取物品 ID、out 先写默认特效号 524
    (SINGLE_FXR_RVA, &[0x48, 0x89, 0x5C, 0x24, 0x18, 0x57, 0x48, 0x83, 0xEC, 0x50, 0x8B, 0x59, 0x28, 0x48, 0x8B, 0xFA, 0x8B, 0xC3, 0xC7, 0x02, 0x0C, 0x02, 0x00, 0x00]),
];
/// 掉落物对象：+0x10 同组下一个；+0xCA 低 4 位是稀有度，高位是标志与分组位
const NEXT: usize = 0x10;
const RARITY_BYTE: usize = 0xCA;
const RARITY_MASK: u8 = 0x0F;
/// 超级系列的稀有度，也直接用作外部值（FXR 6104 / 6105 里 6 → 状态 9）
const SUPER_RARITY: u8 = 6;
/// 包里的分组主特效（6000 的超集），装钩子的标志
const SUPER_GROUP_FXR: i32 = 6104;
const KATANA_FXR: i32 = 572;
/// 包里的单件太刀特效（572 的超集）
const SUPER_KATANA_FXR: i32 = 6105;
/// 一组通常 3 个；坏链（成环）时最多走这么多
const MAX_CHAIN: usize = 16;

/// 链头 -> 外部值（只用 al）
type GroupColorFn = unsafe extern "C" fn(usize) -> u8;
/// (对象, &特效号) -> &特效号
type SingleFxrFn = unsafe extern "C" fn(usize, *mut i32) -> *mut i32;

static GROUP_COLOR_TRAMPOLINE: AtomicUsize = AtomicUsize::new(0);
static SINGLE_FXR_TRAMPOLINE: AtomicUsize = AtomicUsize::new(0);

pub struct Targets {
    group_color: usize,
    single_fxr: usize,
}

/// GameSystemCommonParam 第 0 行的分组球特效号（启动时从内存里的参数表读）
pub fn group_orb_fxr(repository: &ParamRepository) -> Option<i32> {
    let t = game_system_common::TABLE;
    let (address, size) = repository.table_locations(t.struct_name).into_iter().next()?;
    let bytes = read_bytes(address, size as usize)?;
    let table = ParamFile::parse(&bytes, t.row_size).ok()?;
    field::i32(table.row(game_system_common::ROW)?, game_system_common::GROUP_ORB_FXR)
}

/// 包里带了超级球特效（分组球特效号为 6104）且代码对得上才返回目标
pub fn locate(module: &GameModule, group_orb_fxr: Option<i32>) -> Option<Targets> {
    match group_orb_fxr {
        Some(SUPER_GROUP_FXR) => {}
        Some(fxr) => {
            info!("super orb not installed: group orb FXR is {fxr}, not {SUPER_GROUP_FXR} (package without super orb effects)");
            return None;
        }
        None => {
            warn!("super orb disabled: group orb FXR unreadable (GameSystemCommonParam row {})", game_system_common::ROW);
            return None;
        }
    }
    if !SIGNATURES.iter().all(|&(rva, code)| read_bytes(module.va(rva), code.len()).as_deref() == Some(code)) {
        warn!("super orb disabled: drop orb code mismatch (wrong game build?)");
        return None;
    }
    Some(Targets { group_color: module.va(GROUP_COLOR_RVA), single_fxr: module.va(SINGLE_FXR_RVA) })
}

/// 需在叠加层安装之后调用（叠加层关闭时这里自行初始化 MinHook）
pub fn install(t: Targets) {
    if !hook::ensure_initialized() {
        warn!("super orb disabled: MinHook unavailable");
        return;
    }
    let group = hook::create(t.group_color, group_color_detour as *mut c_void, &GROUP_COLOR_TRAMPOLINE);
    let single = hook::create(t.single_fxr, single_fxr_detour as *mut c_void, &SINGLE_FXR_TRAMPOLINE);
    let hooks = match (group, single) {
        (Ok(g), Ok(s)) => [g, s],
        (g, s) => {
            warn!("super orb unavailable: {:?} {:?}", g.err(), s.err());
            return;
        }
    };
    if hook::enable(&hooks) {
        info!("super orb installed: groups with rarity {SUPER_RARITY} use external value {SUPER_RARITY}, rarity {SUPER_RARITY} katana orb FXR {KATANA_FXR} -> {SUPER_KATANA_FXR}");
    } else {
        warn!("super orb: enabling hooks failed");
    }
}

unsafe extern "C" fn group_color_detour(head: usize) -> u8 {
    let original: GroupColorFn = unsafe { std::mem::transmute(GROUP_COLOR_TRAMPOLINE.load(Ordering::Acquire)) };
    let color = unsafe { original(head) };
    let mut rarities = [0u8; MAX_CHAIN];
    read_chain(head, &mut rarities).map_or(color, |r| group_color(r, color))
}

unsafe extern "C" fn single_fxr_detour(item: usize, out: *mut i32) -> *mut i32 {
    let original: SingleFxrFn = unsafe { std::mem::transmute(SINGLE_FXR_TRAMPOLINE.load(Ordering::Acquire)) };
    let result = unsafe { original(item, out) };
    // 原函数每个分支都写 *out 并返回 out；两个调用方传的都是掉落物对象，单件颜色同样取 +0xCA 低 4 位
    let rarity = read::<u8>(item.wrapping_add(RARITY_BYTE));
    if let (Some(rarity), Some(fxr)) = (rarity, read::<i32>(result as usize)) {
        let swapped = single_fxr(rarity, fxr);
        if swapped != fxr {
            unsafe { result.write_unaligned(swapped) };
        }
    }
    result
}

/// 按原函数的走法（+0x10 下一个）读出组内各对象的稀有度字节；任一读取失败返回 None
fn read_chain(head: usize, out: &mut [u8; MAX_CHAIN]) -> Option<&[u8]> {
    let mut node = head;
    let mut count = 0;
    for slot in out.iter_mut() {
        if node == 0 {
            break;
        }
        *slot = read::<u8>(node.wrapping_add(RARITY_BYTE))?;
        node = read_ptr(node.wrapping_add(NEXT))?;
        count += 1;
    }
    out.get(..count)
}

/// 组内任一对象稀有度为 6 就用超级球的外部值，否则沿用原函数的结果
fn group_color(rarities: &[u8], original: u8) -> u8 {
    if rarities.iter().any(|&b| b & RARITY_MASK == SUPER_RARITY) { SUPER_RARITY } else { original }
}

/// 稀有度 6 的太刀单件球换成超集特效，其余不变（普通太刀仍用原版 572）
fn single_fxr(rarity: u8, fxr: i32) -> i32 {
    if fxr == KATANA_FXR && rarity & RARITY_MASK == SUPER_RARITY { SUPER_KATANA_FXR } else { fxr }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn super_rarity_takes_over_the_group() {
        // 阎魔刀（6，带分组位 0x80）和两个金色护符同组：原函数选金色 3
        assert_eq!(group_color(&[0x86, 0x83, 0x83], 3), SUPER_RARITY);
        // 带 0x10 标志
        assert_eq!(group_color(&[0x93, 0x96], 3), SUPER_RARITY);
        // 整组都是 6：原函数全按 -1 算，返回 0xFF
        assert_eq!(group_color(&[0x86], 0xFF), SUPER_RARITY);
    }

    #[test]
    fn other_groups_keep_the_original_color() {
        assert_eq!(group_color(&[0x83, 0x81, 0x89], 3), 3);
        assert_eq!(group_color(&[0x84, 0x85, 0x88], 0xFF), 0xFF);
        assert_eq!(group_color(&[], 0xFF), 0xFF);
        // 只看低 4 位，高位凑不出 6
        assert_eq!(group_color(&[0x60, 0xE0], 0), 0);
    }

    #[test]
    fn only_super_katana_orbs_switch_effect() {
        // 稀有度 6（可带 0x10 标志）的太刀
        assert_eq!(single_fxr(0x06, KATANA_FXR), SUPER_KATANA_FXR);
        assert_eq!(single_fxr(0x16, KATANA_FXR), SUPER_KATANA_FXR);
        // 普通太刀不动
        for rarity in [0x00, 0x01, 0x02, 0x03, 0x04, 0x09, 0x13, 0x60] {
            assert_eq!(single_fxr(rarity, KATANA_FXR), KATANA_FXR);
        }
        // 其它特效不动
        for fxr in [501, 524, 526, 551, 571, 573, 583, SUPER_KATANA_FXR] {
            assert_eq!(single_fxr(0x06, fxr), fxr);
        }
    }

    /// 用真实内存搭链表（read 走 ReadProcessMemory，能读本进程）
    fn node(rarity: u8) -> Box<[u8; 0x100]> {
        let mut n = Box::new([0u8; 0x100]);
        n[RARITY_BYTE] = rarity;
        n
    }

    fn link(from: &mut [u8; 0x100], to: usize) {
        from[NEXT..NEXT + 8].copy_from_slice(&to.to_le_bytes());
    }

    #[test]
    fn reads_the_whole_chain() {
        let (mut a, mut b, c) = (node(0x83), node(0x86), node(0x81));
        link(&mut b, c.as_ptr() as usize);
        link(&mut a, b.as_ptr() as usize);
        let mut buf = [0; MAX_CHAIN];
        assert_eq!(read_chain(a.as_ptr() as usize, &mut buf), Some(&[0x83, 0x86, 0x81][..]));
        assert_eq!(read_chain(0, &mut buf), Some(&[][..]));
    }

    #[test]
    fn bad_chains_are_bounded_or_rejected() {
        // 自己指向自己：最多走 MAX_CHAIN 个
        let mut a = node(0x83);
        let looped = a.as_ptr() as usize;
        link(&mut a, looped);
        let mut buf = [0; MAX_CHAIN];
        assert_eq!(read_chain(looped, &mut buf).map(<[u8]>::len), Some(MAX_CHAIN));
        // 下一个指向不可读的地址：放弃，钩子沿用原函数结果
        let mut b = node(0x86);
        link(&mut b, 0x10);
        assert_eq!(read_chain(b.as_ptr() as usize, &mut buf), None);
    }
}
