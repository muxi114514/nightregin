//! 当前装备的器皿。实测布局（2026-10-25，内存差分 + 指针扫描）：
//! PlayerGameData+0x6F8 -> 装备数据；装备数据+0x18 为 DLMap<heroId, 角色配置> = {分配器, 头节点, 元素数}；
//! 树节点为 MSVC std::map 布局：+0x0 左、+0x8 父、+0x10 右、+0x19 isnil，+0x20 键（heroId），+0x2C 当前器皿 ID。
//! 装备数据+0x0 为另一张 DLMap<器皿 ID, 器皿配置>（2026-11-07 实测 70 个）：节点 +0x1C 键（器皿 ID），+0x24 起 6 个遗物句柄（3 普通 + 3 深夜，0 为空）。
//! 通用器皿（19000 段）不在全局表里：每个角色配置节点 +0x30 起是该角色自己的 DLMap<通用器皿 ID, 器皿配置>（节点格式同上）。
//! 遗物装在器皿里，所以某角色带的遗物 = 他当前器皿那条记录里的 6 个句柄（先查自己的通用器皿表，再查全局表）。

use super::memory::{looks_like_pointer, read, read_ptr};

const PGD_EQUIP_DATA: usize = 0x6F8;
const EQUIP_VESSEL_MAP: usize = 0x0;
const VESSEL_NODE_KEY: usize = 0x1C;
const VESSEL_NODE_RELICS: usize = 0x24;
pub const VESSEL_RELIC_SLOTS: usize = 6;
const EQUIP_HERO_MAP: usize = 0x18;
const MAP_HEAD: usize = 0x8;
const MAP_SIZE: usize = 0x10;
const NODE_LEFT: usize = 0x0;
const NODE_PARENT: usize = 0x8;
const NODE_RIGHT: usize = 0x10;
const NODE_ISNIL: usize = 0x19;
const NODE_KEY: usize = 0x20;
const NODE_VESSEL: usize = 0x2C;
const NODE_UNIVERSAL_MAP: usize = 0x30;
/// 树深度上限，防止读到损坏数据时死循环
const MAX_DEPTH: usize = 64;

/// 全部角色配置节点 (heroId, 节点地址)，中序遍历；读不到为空
pub fn hero_nodes(game_data: usize) -> Vec<(u32, usize)> {
    let Some(equip) = read_ptr(game_data + PGD_EQUIP_DATA).filter(|&p| looks_like_pointer(p)) else { return Vec::new() };
    let map = equip + EQUIP_HERO_MAP;
    let Some(head) = read_ptr(map + MAP_HEAD).filter(|&p| looks_like_pointer(p)) else { return Vec::new() };
    let mut out = Vec::new();
    let mut stack = Vec::new();
    let mut node = read_ptr(head + NODE_PARENT).unwrap_or(head);
    // 显式栈中序遍历，步数上限防止损坏数据死循环
    for _ in 0..MAX_DEPTH * 4 {
        while node != head && looks_like_pointer(node) && read::<u8>(node + NODE_ISNIL) == Some(0) && stack.len() < MAX_DEPTH {
            stack.push(node);
            node = read_ptr(node + NODE_LEFT).unwrap_or(head);
        }
        let Some(n) = stack.pop() else { break };
        if let Some(key) = read::<u32>(n + NODE_KEY) {
            out.push((key, n));
        }
        node = read_ptr(n + NODE_RIGHT).unwrap_or(head);
    }
    out
}

/// 在 DLMap（map = {分配器, 头节点, 元素数}）里按键二叉查找节点；key_off 为键在节点里的偏移
fn find_node(map: usize, key: u32, key_off: usize, max_size: u64) -> Option<usize> {
    let head = read_ptr(map + MAP_HEAD).filter(|&p| looks_like_pointer(p))?;
    let size = read::<u64>(map + MAP_SIZE)?;
    if size == 0 || size > max_size {
        return None;
    }
    let mut node = read_ptr(head + NODE_PARENT)?;
    for _ in 0..MAX_DEPTH {
        if node == head || !looks_like_pointer(node) || read::<u8>(node + NODE_ISNIL)? != 0 {
            return None;
        }
        let k = read::<u32>(node + key_off)?;
        if k == key {
            return Some(node);
        }
        node = read_ptr(node + if key < k { NODE_LEFT } else { NODE_RIGHT })?;
    }
    None
}

/// hero_id 为 1..=10；返回该角色当前装备的器皿 ID（AntiqueStandParam 行号）
pub fn equipped_vessel(game_data: usize, hero_id: u32) -> Option<u32> {
    let equip = read_ptr(game_data + PGD_EQUIP_DATA).filter(|&p| looks_like_pointer(p))?;
    let node = find_node(equip + EQUIP_HERO_MAP, hero_id, NODE_KEY, 64)?;
    read::<u32>(node + NODE_VESSEL)
}

/// 某角色（配置节点）的器皿里装的 6 个遗物句柄（0 为空）：先查该角色的通用器皿表，再查全局器皿表
fn hero_vessel_relics(equip: usize, hero_node: usize, vessel_id: u32) -> Option<[u32; VESSEL_RELIC_SLOTS]> {
    let node = find_node(hero_node + NODE_UNIVERSAL_MAP, vessel_id, VESSEL_NODE_KEY, 64).or_else(|| find_node(equip + EQUIP_VESSEL_MAP, vessel_id, VESSEL_NODE_KEY, 4096))?;
    read::<[u32; VESSEL_RELIC_SLOTS]>(node + VESSEL_NODE_RELICS)
}

/// 每个角色 (heroId, 当前器皿, 器皿里的遗物)；遗物读不到为 None
pub fn hero_loadouts(game_data: usize) -> Vec<(u32, u32, Option<[u32; VESSEL_RELIC_SLOTS]>)> {
    let Some(equip) = read_ptr(game_data + PGD_EQUIP_DATA).filter(|&p| looks_like_pointer(p)) else { return Vec::new() };
    hero_nodes(game_data)
        .into_iter()
        .map(|(hero, node)| {
            let vessel = read::<u32>(node + NODE_VESSEL).unwrap_or(0);
            (hero, vessel, hero_vessel_relics(equip, node, vessel))
        })
        .collect()
}
