//! 防闪退：MMV 移植的黑魂敌人（火焰恶魔 c6220、屠龙铠甲 c8300、Outrider Knight c1310）的骨架 / 行为数据里有超出骨架范围的骨骼号。
//! Havok 的骨骼链函数（rva 0x1949930：骨骼 A 是否是骨骼 B 的祖先，并输出 B→A 的链条）沿父节点往上走时越界读到垃圾，链条不终止，
//! 数组涨到上亿项后扩容失败、写空指针崩溃（崩在 0x1949A2F；它返回真后，调用方 0x161DA90 再从 A 往上走，崩在 0x161DB4F）。
//! 钩住 0x1949930：先自己沿 B 的父链走到根，骨骼号越界或步数超过骨骼数（成环）就按原函数「A > B」分支的做法返回假并清空输出，
//! 调用方随即跳过这次骨骼链计算（这一帧该处 IK 不生效）。数据正常时原样调用原函数，结果不变。

use std::ffi::c_void;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

use log::{info, warn};

use crate::game::memory::read;
use crate::game::module::GameModule;
use crate::hook;

const BONE_CHAIN_RVA: usize = 0x1949930;
/// 函数开头 15 字节（三条保存寄存器的 mov），校验游戏版本
const BONE_CHAIN_PROLOGUE: [u8; 15] = [0x48, 0x89, 0x5C, 0x24, 0x10, 0x48, 0x89, 0x74, 0x24, 0x18, 0x4C, 0x89, 0x64, 0x24, 0x20];
/// hkaSkeleton：父节点数组数据指针（i16）、骨骼数（原函数按它分配链条缓冲）
const SKELETON_PARENTS: usize = 0x20;
const SKELETON_BONE_COUNT: usize = 0x38;
/// 输出链条 hkArray 的元素数
const ARRAY_SIZE: usize = 8;

/// (&结果, 骨架, 祖先骨骼, 起点骨骼, &输出链条) -> &结果
type BoneChainFn = unsafe extern "C" fn(*mut u8, usize, u32, u32, usize) -> *mut u8;

static TRAMPOLINE: AtomicUsize = AtomicUsize::new(0);
static REJECTED: AtomicU32 = AtomicU32::new(0);

/// 校验函数开头，返回其地址；版本不对时不装
pub fn locate(module: &GameModule) -> Option<usize> {
    let target = module.va(BONE_CHAIN_RVA);
    if read::<[u8; 15]>(target) != Some(BONE_CHAIN_PROLOGUE) {
        warn!("havok guard disabled: bone chain function mismatch (wrong game build?)");
        return None;
    }
    Some(target)
}

/// 需在叠加层安装之后调用（叠加层关闭时这里自行初始化 MinHook）
pub fn install(target: usize) {
    if !hook::ensure_initialized() {
        warn!("havok guard disabled: MinHook unavailable");
        return;
    }
    match hook::create(target, detour as *mut c_void, &TRAMPOLINE) {
        Ok(h) if hook::enable(std::slice::from_ref(&h)) => info!("havok guard installed: invalid bone chains are skipped"),
        Ok(_) => warn!("havok guard: enabling hook failed"),
        Err(e) => warn!("havok guard unavailable: {e}"),
    }
}

/// 骨骼号按 16 位有符号数用（原函数 movzx r8w / r9w）
unsafe extern "C" fn detour(result: *mut u8, skeleton: usize, ancestor: u32, bone: u32, chain: usize) -> *mut u8 {
    let (ancestor16, bone16) = (ancestor as u16 as i16, bone as u16 as i16);
    if !unsafe { skeleton_chain_ok(skeleton, ancestor16, bone16) } {
        let n = REJECTED.fetch_add(1, Ordering::Relaxed) + 1;
        // 同一个敌人可能每帧都触发，只在第 1、2、4、8… 次记日志
        if n.is_power_of_two() {
            info!("havok guard: skipped invalid bone chain #{n} (skeleton {skeleton:#x}, bone {bone16}, ancestor {ancestor16})");
        }
        unsafe {
            result.write(0);
            ((chain + ARRAY_SIZE) as *mut i32).write(0);
        }
        return result;
    }
    let original: BoneChainFn = unsafe { std::mem::transmute(TRAMPOLINE.load(Ordering::Acquire)) };
    unsafe { original(result, skeleton, ancestor, bone, chain) }
}

/// 只读原函数本来就要读的骨架字段与父节点数组（下标都在骨骼数以内）
unsafe fn skeleton_chain_ok(skeleton: usize, ancestor: i16, bone: i16) -> bool {
    // 起点为负时原函数不进循环；骨架为空交给原函数（原样行为）
    if bone < 0 || skeleton == 0 {
        return true;
    }
    let count = unsafe { ((skeleton + SKELETON_BONE_COUNT) as *const i32).read() };
    let parents = unsafe { ((skeleton + SKELETON_PARENTS) as *const *const i16).read() };
    if parents.is_null() {
        return false;
    }
    chain_ok(count, ancestor, bone, |i| unsafe { parents.add(i).read() })
}

/// 从 bone 沿父节点走到根（遇到 ≤ -1 结束）：骨骼号都在 [0, count) 内、步数不超过 count 才算正常。
/// ancestor 越界时原函数本来也找不到它、返回假，这里同样判为不正常，结果一致
fn chain_ok(count: i32, ancestor: i16, bone: i16, parent: impl Fn(usize) -> i16) -> bool {
    if i32::from(ancestor) >= count {
        return false;
    }
    let mut current = i32::from(bone);
    let mut steps = 0;
    while current > -1 {
        if current >= count || steps >= count {
            return false;
        }
        current = i32::from(parent(current as usize));
        steps += 1;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn walk(parents: &[i16], ancestor: i16, bone: i16) -> bool {
        chain_ok(parents.len() as i32, ancestor, bone, |i| parents[i])
    }

    #[test]
    fn accepts_valid_chains() {
        // 0 为根，1→0，2→1，3→1
        let p = [-1, 0, 1, 1];
        assert!(walk(&p, 0, 3));
        assert!(walk(&p, 2, 3));
        assert!(walk(&p, 0, -1));
    }

    #[test]
    fn rejects_out_of_range_and_cycles() {
        assert!(!walk(&[-1, 0, 7], 0, 2));
        assert!(!walk(&[-1, 0], 0, 0x4B0E));
        assert!(!walk(&[-1, 2, 1], 0, 2));
        assert!(!walk(&[-1, 0], 5, 1));
        assert!(!walk(&[], 0, 0));
    }
}
