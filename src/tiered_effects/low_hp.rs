//! 「血量偏低」两条紫锻体：判定线直接改内存里 SpEffectParam 的 conditionHp（参数表只在本机生效，不影响队友），
//! 锁血在血量高于判定线时按超出的点数施加扣血效果。

use log::{info, warn};
use nr_stat_alloc_shared::param::layout::sp_effect::{CONDITION_HP, TABLE};
use nr_stat_alloc_shared::param::{ParamFile, field};
use nr_stat_alloc_shared::tiered_low_hp::{BASE_THRESHOLD, HP_CAP, cap, drains, is_low_hp_row, threshold};

use crate::game::memory::{read_bytes, write};
use crate::game::params::ParamRepository;

use super::Frame;

/// 扣血后隔几帧再看血量（施加的效果下一帧才扣到血量上，不等会重复扣）
const DRAIN_WAIT_FRAMES: u8 = 3;

pub struct LowHp {
    /// 「血量偏低时」效果行在游戏内存中的地址
    rows: Vec<usize>,
    /// 当前写进参数表的判定线
    applied: f32,
    wait: u8,
}

impl LowHp {
    /// 启动时记下原值为 40 的效果行；表缺失或结构不符时为 None
    pub fn locate(repository: &ParamRepository) -> Option<Self> {
        let (address, size) = repository.table_locations(TABLE.struct_name).into_iter().next()?;
        let bytes = read_bytes(address, size as usize)?;
        let table = ParamFile::parse(&bytes, TABLE.row_size).map_err(|e| warn!("{}: {e}", TABLE.struct_name)).ok()?;
        let rows: Vec<usize> = table
            .rows()
            .filter(|r| field::f32(r.data, CONDITION_HP).is_some_and(|hp| is_low_hp_row(r.id, hp)))
            .map(|r| address + r.offset)
            .collect();
        info!("tiered low hp: {} effects with the low hp threshold", rows.len());
        (!rows.is_empty()).then_some(Self { rows, applied: BASE_THRESHOLD, wait: 0 })
    }

    /// apply：施加效果（游戏主线程）
    pub fn tick(&mut self, frame: &Frame, mut apply: impl FnMut(i32)) {
        // 出击中只升不降：标记偶尔不在列表里时不把判定线压回去（锁血会跟着把血扣多）
        let line = threshold(frame.ids).max(self.applied);
        self.set(line);
        if self.wait > 0 {
            self.wait -= 1;
            return;
        }
        let (hp, max) = frame.hp;
        let limit = cap(max, line);
        if frame.ids.contains(&HP_CAP) && hp > limit {
            drains(hp - limit).for_each(&mut apply);
            self.wait = DRAIN_WAIT_FRAMES;
        }
    }

    /// 回到圆桌：还原判定线
    pub fn reset(&mut self) {
        self.set(BASE_THRESHOLD);
        self.wait = 0;
    }

    fn set(&mut self, line: f32) {
        if line == self.applied {
            return;
        }
        let failed = self.rows.iter().filter(|&&row| !write(row + CONDITION_HP, line)).count();
        if failed > 0 {
            warn!("tiered low hp: {failed} threshold writes failed");
        }
        info!("tiered low hp: threshold {}% -> {line}%", self.applied);
        self.applied = line;
    }
}
