//! 红露滴圣杯瓶（2026-10-31 实机核对）：上限 = PlayerGameData+0xFF 的一个字节（赐福补充按它补）；
//! 当前次数 = 背包里「已装」圣杯瓶条目的数量，喝光时条目换成「空」的编号、数量 1，快捷栏句柄（PGD+0x4A8）也跟着换。

use nr_stat_alloc_shared::tiered_flasks::{RED_FLASKS, is_filled};

use super::inventory::{self, Entry};
use super::memory::{read, write};

const MAX: usize = 0xFF;
const QUANTITY: usize = 0x8;

pub fn max(game_data: usize) -> Option<u8> {
    read::<u8>(game_data + MAX)
}

pub fn set_max(game_data: usize, value: u8) -> bool {
    write(game_data + MAX, value)
}

/// 背包里的圣杯瓶条目
pub struct Flask {
    pub entry: Entry,
    addr: usize,
    pub quantity: u32,
}

impl Flask {
    pub fn filled(&self) -> bool {
        is_filled(self.entry.item_id)
    }

    /// 只对已装的瓶子有意义；空瓶要换编号，走游戏自己的补充
    pub fn set_quantity(&self, quantity: u32) -> bool {
        write(self.addr + QUANTITY, quantity)
    }
}

/// hint：上次找到的条目编号，先查它，不对再整表找
pub fn find(game_data: usize, hint: Option<u32>) -> Option<Flask> {
    let equip = inventory::equip_game_data(game_data);
    let (entry, addr) = hint
        .and_then(|i| inventory::entry_at(equip, i))
        .filter(|(e, _)| RED_FLASKS.contains(&e.item_id))
        .or_else(|| inventory::find_goods(equip, |id| RED_FLASKS.contains(&id)))?;
    Some(Flask { entry, addr, quantity: read::<u32>(addr + QUANTITY)? })
}
