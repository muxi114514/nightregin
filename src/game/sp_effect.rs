//! 角色 SpEffect 的查询与增删。增删必须在游戏主线程（每帧任务）里调用。

use super::addresses::{Addresses, offsets};
use super::memory::{looks_like_pointer, read, read_ptr};

/// 链表遍历上限，防止读到损坏数据时死循环
const MAX_NODES: usize = 2048;

/// 角色当前所有 SpEffect ID；结构不可读时返回 None
pub fn list_ids(chr: usize) -> Option<Vec<i32>> {
    let special_effect = read_ptr(chr + offsets::CHR_SPECIAL_EFFECT)?;
    if !looks_like_pointer(special_effect) {
        return None;
    }
    let mut node = read_ptr(special_effect + offsets::SE_HEAD)?;
    let mut ids = Vec::new();
    while node != 0 {
        if !looks_like_pointer(node) || ids.len() >= MAX_NODES {
            return None;
        }
        ids.push(read::<i32>(node + offsets::SE_ENTRY_ID)?);
        node = read_ptr(node + offsets::SE_ENTRY_NEXT)?;
    }
    Some(ids)
}

/// 角色是否同时带着 ids 里的全部效果（不分配内存，给钩子用；结构不可读时为假）
pub fn has_all<const N: usize>(chr: usize, ids: [i32; N]) -> bool {
    let Some(special_effect) = read_ptr(chr + offsets::CHR_SPECIAL_EFFECT) else { return false };
    if !looks_like_pointer(special_effect) {
        return false;
    }
    let Some(mut node) = read_ptr(special_effect + offsets::SE_HEAD) else { return false };
    let mut found = [false; N];
    let mut steps = 0;
    while node != 0 && steps < MAX_NODES {
        if !looks_like_pointer(node) {
            return false;
        }
        let Some(id) = read::<i32>(node + offsets::SE_ENTRY_ID) else { return false };
        for (hit, want) in found.iter_mut().zip(ids) {
            *hit |= id == want;
        }
        let Some(next) = read_ptr(node + offsets::SE_ENTRY_NEXT) else { return false };
        node = next;
        steps += 1;
    }
    found.iter().all(|hit| *hit)
}

/// sync = true 时同步给联机队友（原生参数 dont_sync = false）
pub fn apply(addrs: &Addresses, chr: usize, id: i32, sync: bool) {
    let f: extern "C" fn(usize, i32, bool) -> u64 = unsafe { std::mem::transmute(addrs.apply_speffect) };
    f(chr, id, !sync);
}

pub fn remove(addrs: &Addresses, chr: usize, id: i32) {
    let f: extern "C" fn(usize, i32) -> u64 = unsafe { std::mem::transmute(addrs.remove_speffect) };
    f(chr, id);
}
