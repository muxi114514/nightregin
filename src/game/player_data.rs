//! PlayerGameData 定位与等级 / 属性读取。
//! PlayerIns 内的指针偏移靠 RTTI 运行时发现（实测 0x5A0）；属性为基础值（不含遗物加成），可在配置中覆盖偏移。
//! 实测布局：+0x10 血量 当前/上限/上限，+0x1C 专注，+0x28 精力，+0x34 起 8 项基础属性（比 ER 少两个浮点字段，整体前移 8），
//! +0x68 等级，+0x1B8 起 8 项最终属性（基础 + 遗物 + SpEffect 加成，即属性界面显示值）。

use log::info;

use crate::config::OffsetOverrides;

use super::addresses::Addresses;
use super::memory::{looks_like_pointer, read, read_ptr};
use super::rtti::has_vtable;

pub const DEFAULT_STATS_OFFSET: usize = 0x34;
pub const DEFAULT_LEVEL_OFFSET: usize = 0x68;
const VITALS_OFFSET: usize = 0x10;
/// 当前皮肤在 CharaInitParam 中的 ID，查表得到角色（heroId）
const CHARA_INIT_OFFSET: usize = 0xE8;
const EFFECTIVE_STATS_OFFSET: usize = 0x1B8;
const PLAYER_INS_SCAN_LEN: usize = 0x2000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerSnapshot {
    pub level: u32,
    /// 基础属性：生命 精神 耐力 力量 敏捷 智力 信仰 感应
    pub stats: [u32; 8],
    /// 最终属性（含遗物与 SpEffect 加成）
    pub effective: [u32; 8],
    /// 血量 / 专注 / 精力，各为 [当前, 上限, 上限]（第三个值实测与上限同步变化）
    pub vitals: [[u32; 3]; 3],
    pub chara_init: u32,
}

impl PlayerSnapshot {
    pub fn hp(&self) -> (u32, u32) {
        (self.vitals[0][0], self.vitals[0][1])
    }

    pub fn fp(&self) -> (u32, u32) {
        (self.vitals[1][0], self.vitals[1][1])
    }
}

impl PlayerSnapshot {
    pub fn plausible(&self) -> bool {
        (1..=99).contains(&self.level) && self.stats.iter().all(|s| (1..=150).contains(s))
    }
}

pub struct PlayerDataLocator {
    game_data_offset: Option<usize>,
    stats_offset: usize,
    level_offset: usize,
}

impl PlayerDataLocator {
    pub fn new(overrides: &OffsetOverrides) -> Self {
        Self {
            game_data_offset: overrides.player_game_data,
            stats_offset: overrides.stats.unwrap_or(DEFAULT_STATS_OFFSET),
            level_offset: overrides.level.unwrap_or(DEFAULT_LEVEL_OFFSET),
        }
    }

    pub fn game_data_offset(&self) -> Option<usize> {
        self.game_data_offset
    }

    /// PlayerIns -> PlayerGameData；首次调用时扫描并缓存偏移
    pub fn game_data(&mut self, addrs: &Addresses, player: usize) -> Option<usize> {
        if let Some(off) = self.game_data_offset {
            let p = read_ptr(player + off)?;
            if has_vtable(p, addrs.vt_player_game_data) {
                return Some(p);
            }
        }
        for off in (0..PLAYER_INS_SCAN_LEN).step_by(8) {
            let Some(value) = read_ptr(player + off) else { continue };
            if looks_like_pointer(value) && read_ptr(value) == Some(addrs.vt_player_game_data) {
                if self.game_data_offset != Some(off) {
                    info!("PlayerIns -> PlayerGameData offset discovered: {off:#x}");
                }
                self.game_data_offset = Some(off);
                return Some(value);
            }
        }
        None
    }

    pub fn snapshot(&self, game_data: usize) -> Option<PlayerSnapshot> {
        Some(PlayerSnapshot {
            stats: read::<[u32; 8]>(game_data + self.stats_offset)?,
            level: read::<u32>(game_data + self.level_offset)?,
            effective: read::<[u32; 8]>(game_data + EFFECTIVE_STATS_OFFSET)?,
            vitals: read::<[[u32; 3]; 3]>(game_data + VITALS_OFFSET)?,
            chara_init: read::<u32>(game_data + CHARA_INIT_OFFSET)?,
        })
    }

    /// 只用已缓存的偏移读取（诊断其它角色用，避免对非玩家对象做扫描而污染缓存）
    pub fn game_data_cached(&self, addrs: &Addresses, player: usize) -> Option<usize> {
        let p = read_ptr(player + self.game_data_offset?)?;
        has_vtable(p, addrs.vt_player_game_data).then_some(p)
    }
}
