//! 道具类锻体：金12 / 学者2 按标记个数把持有上限乘 1 / 2 / 4（直接改内存里 EquipParamGoods 的 maxNum 与学者上限），
//! 金12 另外常驻「使用道具时效果扩散到周围同伴」的三个状态效果。参数表只在本机生效，不影响队友。

use log::{info, warn};
use nr_stat_alloc_shared::param::layout::equip_goods::{self, BAGCRAFT_MAX_NUM, MAX_NUM};
use nr_stat_alloc_shared::param::{ParamFile, field};
use nr_stat_alloc_shared::tiered_items::{GOLD_ITEMS, SPREAD_EFFECTS, is_scaled, multiplier, scaled_bagcraft, scaled_max};

use crate::game::memory::{read_bytes, write};
use crate::game::params::ParamRepository;

use super::Frame;

/// 一行道具：行在游戏内存中的地址与两个上限的原值
struct Limit {
    row: usize,
    max: i16,
    bagcraft: i8,
}

pub struct ItemLimits {
    limits: Vec<Limit>,
    /// 当前写进参数表的倍数
    applied: u8,
}

impl ItemLimits {
    /// 启动时记下要改的行（此时参数表还是原值）；表缺失或结构不符时为 None
    pub fn locate(repository: &ParamRepository) -> Option<Self> {
        let t = equip_goods::TABLE;
        let (address, size) = repository.table_locations(t.struct_name).into_iter().next()?;
        let bytes = read_bytes(address, size as usize)?;
        let table = ParamFile::parse(&bytes, t.row_size).map_err(|e| warn!("{}: {e}", t.struct_name)).ok()?;
        let limits: Vec<Limit> = table
            .rows()
            .filter_map(|r| {
                let bagcraft = field::i8(r.data, BAGCRAFT_MAX_NUM)?;
                is_scaled(r.id, bagcraft).then_some(Limit { row: address + r.offset, max: field::i16(r.data, MAX_NUM)?, bagcraft })
            })
            .collect();
        info!("tiered items: {} goods with scalable capacity", limits.len());
        (!limits.is_empty()).then_some(Self { limits, applied: 1 })
    }

    /// apply：施加效果（游戏主线程）
    pub fn tick(&mut self, frame: &Frame, mut apply: impl FnMut(i32)) {
        if frame.ids.contains(&GOLD_ITEMS) {
            SPREAD_EFFECTS.iter().filter(|id| !frame.ids.contains(id)).for_each(|&id| apply(id));
        }
        // 出击中只升不降：标记偶尔不在列表里时不把上限压回去（多拿的道具会卡在上限之上）
        self.set(multiplier(frame.ids).max(self.applied));
    }

    /// 回到圆桌：还原上限
    pub fn reset(&mut self) {
        self.set(1);
    }

    fn set(&mut self, m: u8) {
        if m == self.applied {
            return;
        }
        let failed = self
            .limits
            .iter()
            .filter(|l| {
                let max = write(l.row + MAX_NUM, scaled_max(l.max, m));
                let bagcraft = write(l.row + BAGCRAFT_MAX_NUM, scaled_bagcraft(l.bagcraft, m));
                !(max && bagcraft)
            })
            .count();
        if failed > 0 {
            warn!("tiered items: {failed} capacity writes failed");
        }
        info!("tiered items: capacity x{} -> x{m}", self.applied);
        self.applied = m;
    }
}
