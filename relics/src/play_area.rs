//! 夜晚首领（LotResultPlayAreaParam）：每个地图模式一行，bossId1 / 2 = 第一 / 二夜首领，extraBossId1 / 2 = 额外首领，
//! 值是首领场地的地图号（4984 = m49_84）。停用某个首领：凡是用到它的格子换成同一夜的另一个首领，
//! 候选按行 ID 轮流取（不让某一个吃下全部份额），跳过该行已有的首领（两夜不重复）。强化（modifier）列不动。
//! 表里没有被停用的首领时什么都不改（如原版独立版）。重复运行结果不变。

use std::collections::HashSet;

use nr_stat_alloc_patcher::param_write;
use nr_stat_alloc_shared::param::{ParamFile, field};

use crate::layout::play_area::BOSS_COLUMNS;

pub struct BossSwap {
    pub boss: i16,
    pub replacements: &'static [i16],
}

#[derive(Default)]
pub struct Report {
    pub replaced: usize,
}

pub fn apply(file: &ParamFile, swaps: &[BossSwap], report: &mut Report) -> Result<Vec<u8>, String> {
    let mut rows = param_write::rows_of(file);
    // 候选必须是表里本来就有的首领，防止写错号
    let present: HashSet<i16> = rows.iter().flat_map(|r| bosses(&r.data)).collect();
    for row in rows.iter_mut() {
        for &col in &BOSS_COLUMNS {
            let Some(swap) = swaps.iter().find(|s| boss_at(&row.data, col) == Some(s.boss)) else { continue };
            let taken = bosses(&row.data);
            let n = swap.replacements.len();
            let pick = (0..n)
                .map(|k| swap.replacements[(row.id as usize + k) % n])
                .find(|r| present.contains(r) && !taken.contains(r) && swaps.iter().all(|s| s.boss != *r))
                .ok_or(format!("row {}: no replacement for boss {}", row.id, swap.boss))?;
            row.data[col..col + 2].copy_from_slice(&pick.to_le_bytes());
            report.replaced += 1;
        }
    }
    param_write::write_ordered(file, &rows)
}

/// 输出自检：被停用的首领一格都不剩
pub fn verify(file: &ParamFile, swaps: &[BossSwap]) -> Result<(), String> {
    match file.rows().find(|r| bosses(r.data).iter().any(|b| swaps.iter().any(|s| s.boss == *b))) {
        Some(r) => Err(format!("self-check: play area {} still uses a disabled night boss", r.id)),
        None => Ok(()),
    }
}

fn boss_at(row: &[u8], col: usize) -> Option<i16> {
    field::u16(row, col).map(|v| v as i16)
}

fn bosses(row: &[u8]) -> Vec<i16> {
    BOSS_COLUMNS.iter().filter_map(|&c| boss_at(row, c)).filter(|&b| b > 0).collect()
}
