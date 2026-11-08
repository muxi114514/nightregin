//! 首领掉落球第三个选项加入锻造石【３】（配置 [boss_loot]）：启动时改写本机内存里的 ItemLotParam_enemy，
//! 不改 regulation（私服按 regulation 哈希分匹配池），只影响自己的客户端。规则见 shared::boss_loot。
//! map 与 enemy 两张掉落表类型名相同，逐张按内容处理（只有 enemy 里有"局内词条 / 徘徊赐福"行）。

use log::{info, warn};
use nr_stat_alloc_shared::boss_loot::{LotEdit, plan};
use nr_stat_alloc_shared::param::ParamFile;
use nr_stat_alloc_shared::param::layout::item_lot;

use crate::config::BossLootConfig;
use crate::game::memory::{read_bytes, write};
use crate::game::params::ParamRepository;

pub fn apply(cfg: &BossLootConfig, repository: &ParamRepository) {
    if cfg.smithing_stone_percent == 0 {
        return;
    }
    let t = item_lot::TABLE;
    let (mut rows, mut failed) = (0, 0);
    for (address, size) in repository.table_locations(t.struct_name) {
        let Some(bytes) = read_bytes(address, size as usize) else { continue };
        let table = match ParamFile::parse(&bytes, t.row_size) {
            Ok(table) => table,
            Err(e) => {
                warn!("boss loot: {} at {address:#x}: {e}", t.struct_name);
                continue;
            }
        };
        for (_, edits) in plan(&table, cfg.smithing_stone_percent) {
            rows += 1;
            failed += edits.iter().filter(|e| !apply_edit(address, e)).count();
        }
    }
    if failed > 0 {
        warn!("boss loot: {failed} writes failed");
    }
    info!("boss loot: smithing stone [3] added to {rows} third-choice rows at {}%", cfg.smithing_stone_percent);
}

fn apply_edit(table: usize, edit: &LotEdit) -> bool {
    match *edit {
        LotEdit::I32(offset, v) => write(table + offset, v),
        LotEdit::U16(offset, v) => write(table + offset, v),
        LotEdit::U8(offset, v) => write(table + offset, v),
    }
}
