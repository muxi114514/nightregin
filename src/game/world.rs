//! WorldChrMan 与玩家角色定位。

use std::collections::BTreeSet;

use super::addresses::{Addresses, offsets};
use super::memory::{looks_like_pointer, read, read_ptr};
use super::rtti::has_vtable;

/// 玩家所在地图块编号在 PlayerIns 中的偏移（编号 = 区域<<24 | 块<<16 | 区<<8 | 序号，格式来自 MapId 转字符串函数 0x677440）。
/// 2026-10-26 实机对比：圆桌 0x0A000000（m10_00_00_00），出征 0x3C2B2600（m60_43_38_00）
const PLAYER_MAP_ID: Option<usize> = Some(0x38);
/// 非出征地图的区域号：m09（标题等）、m10（圆桌）
const HUB_AREAS: [u32; 2] = [9, 10];

pub fn current_map(player: usize) -> Option<u32> {
    read::<u32>(player + PLAYER_MAP_ID?).filter(|&m| m != u32::MAX)
}

pub fn is_expedition(map: u32) -> bool {
    !HUB_AREAS.contains(&(map >> 24))
}

pub fn world_chr_man(addrs: &Addresses) -> Option<usize> {
    let wcm = read_ptr(addrs.world_chr_man_static)?;
    (wcm != 0 && has_vtable(wcm, addrs.vt_world_chr_man)).then_some(wcm)
}

/// 本地玩家（每个客户端各自的主角色）；读图 / 菜单过渡期间为 None
pub fn main_player(addrs: &Addresses) -> Option<usize> {
    let wcm = world_chr_man(addrs)?;
    let player = read_ptr(wcm + offsets::WCM_MAIN_PLAYER)?;
    (player != 0 && has_vtable(player, addrs.vt_player_ins)).then_some(player)
}

/// 诊断用：在 WorldChrMan 对象内及其一级子对象中搜索所有 PlayerIns（含联机队友）
pub fn scan_players(addrs: &Addresses, wcm_scan_len: usize, child_scan_len: usize) -> Vec<usize> {
    let Some(wcm) = world_chr_man(addrs) else { return Vec::new() };
    let mut found = BTreeSet::new();
    let is_player = |p: usize| read_ptr(p) == Some(addrs.vt_player_ins);

    for off in (0..wcm_scan_len).step_by(8) {
        let Some(value) = read_ptr(wcm + off) else { continue };
        if !looks_like_pointer(value) {
            continue;
        }
        if is_player(value) {
            found.insert(value);
            continue;
        }
        // ChrSet 等容器把角色指针放在独立数组里，向下多看一层
        for child_off in (0..child_scan_len).step_by(8) {
            let Some(child) = read_ptr(value + child_off) else { break };
            if looks_like_pointer(child) && is_player(child) {
                found.insert(child);
            }
        }
    }
    found.into_iter().filter(|&p| has_vtable(p, addrs.vt_player_ins)).collect()
}
