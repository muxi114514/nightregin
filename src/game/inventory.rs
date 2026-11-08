//! 背包 EquipInventoryData（EquipGameData + 0x1C8）：菜单与强化按条目编号引用物品，这里换算成物品实例句柄。
//! 布局来自 0x1F6D80：编号 < [+0x1C] 时条目在 [+0x40] 数组，否则在 [+0x30] 数组（编号减去 [+0x1C]）；
//! 条目 0x14 字节，+0x00 实例句柄、+0x04 物品 ID、+0x08 数量；编号上限 [+0x70]（2026-10-26 实机核对，数量 2026-10-31 用圣杯瓶核对）。

use super::memory::{looks_like_pointer, read, read_bytes, read_ptr};

const EQUIP_GAME_DATA: usize = 0x1D8;
const INVENTORY: usize = 0x1C8;
/// 条目数上限（实测约 0x99C），防止读到损坏数据时一次读太多
const MAX_ENTRIES: usize = 0x2000;
/// 实例句柄的类别（高 4 位）：武器 8、道具 B
const WEAPON_HANDLE: u32 = 0x8;
const GOODS_HANDLE: u32 = 0xB;
const SPLIT: usize = 0x1C;
const SECOND_ENTRIES: usize = 0x30;
const FIRST_ENTRIES: usize = 0x40;
const MAX_INDEX: usize = 0x70;
const ENTRY_SIZE: usize = 0x14;
/// EquipGameData +0x08：各装备槽对应的背包条目编号（i32 × 0x20，空槽 -1），0~5 为武器槽（0x1F3430 / 0x1F3E90）
const EQUIP_SLOTS: usize = 0x8;
const EQUIP_SLOT_COUNT: usize = 0x20;
pub const WEAPON_SLOT_COUNT: usize = 6;

/// 背包条目
#[derive(Debug, Clone, Copy)]
pub struct Entry {
    pub index: u32,
    pub handle: u32,
    pub item_id: u32,
}

/// PlayerGameData -> EquipGameData（change_item_id 的 this）
pub fn equip_game_data(player_game_data: usize) -> usize {
    player_game_data + EQUIP_GAME_DATA
}

/// equip_game_data：PlayerGameData + 0x1D8（change_item_id 的 this）
pub fn gaitem_handle(equip_game_data: usize, index: u32) -> Option<u32> {
    entry(equip_game_data, index).map(|e| e.handle)
}

/// 条目编号对应的非空条目
pub fn entry(equip_game_data: usize, index: u32) -> Option<Entry> {
    entry_at(equip_game_data, index).map(|(e, _)| e)
}

/// 非空条目与它在内存里的地址（条目 +0x08 为数量）
pub fn entry_at(equip_game_data: usize, index: u32) -> Option<(Entry, usize)> {
    let inv = equip_game_data + INVENTORY;
    if index > read::<u32>(inv + MAX_INDEX)? {
        return None;
    }
    let split = read::<u32>(inv + SPLIT)?;
    let (entries, i) = if index < split { (FIRST_ENTRIES, index) } else { (SECOND_ENTRIES, index - split) };
    let base = read_ptr(inv + entries).filter(|&p| looks_like_pointer(p))?;
    let at = base + i as usize * ENTRY_SIZE;
    let handle = read::<u32>(at).filter(|&h| h != 0 && h != u32::MAX)?;
    Some((Entry { index, handle, item_id: read::<u32>(at + 4)? & 0x0FFF_FFFF }, at))
}

/// 第一个物品编号满足 pred 的道具条目（实例句柄高 4 位为 0xB）与它的地址；两个数组各一次批量读取
pub fn find_goods(equip_game_data: usize, pred: impl Fn(u32) -> bool) -> Option<(Entry, usize)> {
    scan(equip_game_data, |handle, item_id| handle >> 28 == GOODS_HANDLE && pred(item_id)).into_iter().next()
}

/// 第 slot 个装备槽里的背包条目编号（空槽为 None）
pub fn equipped_index(equip_game_data: usize, slot: usize) -> Option<u32> {
    if slot >= EQUIP_SLOT_COUNT {
        return None;
    }
    read::<i32>(equip_game_data + EQUIP_SLOTS + slot * 4).and_then(|i| u32::try_from(i).ok())
}

/// 全部非空条目中的武器（实例句柄高 4 位为 8）
pub fn weapons(equip_game_data: usize) -> Vec<Entry> {
    scan(equip_game_data, |handle, _| handle >> 28 == WEAPON_HANDLE).into_iter().map(|(e, _)| e).collect()
}

/// 全部非空条目中满足 keep(句柄, 物品编号) 的（条目, 地址）；两个数组各一次批量读取
fn scan(equip_game_data: usize, keep: impl Fn(u32, u32) -> bool) -> Vec<(Entry, usize)> {
    let inv = equip_game_data + INVENTORY;
    let (Some(max), Some(split)) = (read::<u32>(inv + MAX_INDEX), read::<u32>(inv + SPLIT)) else { return Vec::new() };
    let total = (max as usize + 1).min(MAX_ENTRIES);
    let split = (split as usize).min(total);
    let mut out = Vec::new();
    for (array, first, count) in [(FIRST_ENTRIES, 0, split), (SECOND_ENTRIES, split, total - split)] {
        let Some(base) = read_ptr(inv + array).filter(|&p| looks_like_pointer(p)) else { continue };
        let Some(bytes) = read_bytes(base, count * ENTRY_SIZE) else { continue };
        for (i, e) in bytes.chunks_exact(ENTRY_SIZE).enumerate() {
            let handle = u32::from_le_bytes(e[0..4].try_into().unwrap());
            let item_id = u32::from_le_bytes(e[4..8].try_into().unwrap()) & 0x0FFF_FFFF;
            if handle != 0 && handle != u32::MAX && keep(handle, item_id) {
                out.push((Entry { index: (first + i) as u32, handle, item_id }, base + i * ENTRY_SIZE));
            }
        }
    }
    out
}
