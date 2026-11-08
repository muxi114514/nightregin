//! 诊断输出：F9 转储当前状态（含所有 PlayerIns 的组件，用于确认联机同步），F8 校准 PlayerGameData 偏移。

pub mod sound_trace;

use log::{info, warn};

use crate::data::GameData;
use crate::game::addresses::Addresses;
use crate::game::memory::read;
use crate::game::module::GameModule;
use crate::game::player_data::PlayerDataLocator;
use crate::game::rtti::object_type_name;
use crate::game::{equip_data, menu, sp_effect, world};
use crate::stats::components::{compose, is_component};
use crate::stats::heroes::HERO_NAMES_ZH;
use crate::stats::vanilla;

const WCM_SCAN_LEN: usize = 0x18000;
const CHILD_SCAN_LEN: usize = 0x100;
const CALIBRATE_LEN: usize = 0x200;

pub struct DiagContext<'a> {
    pub module: &'a GameModule,
    pub addrs: &'a Addresses,
    pub data: Option<&'a GameData>,
    pub desired: &'a [i32],
    pub status: &'a str,
}

pub fn dump(ctx: &DiagContext, locator: &mut PlayerDataLocator) {
    info!("===== DUMP =====");
    info!("status: {}", ctx.status);
    info!("relic equip menu open: {}", menu::relic_equip_open(ctx.addrs));
    let Some(wcm) = world::world_chr_man(ctx.addrs) else {
        warn!("WorldChrMan not ready");
        return;
    };
    info!("WorldChrMan {wcm:#x}");
    let Some(player) = world::main_player(ctx.addrs) else {
        warn!("main player not available (loading / menu?)");
        return;
    };
    info!("main player {player:#x} type {:?}", object_type_name(ctx.module, player));
    let speffects = sp_effect::list_ids(player).unwrap_or_default();

    match locator.game_data(ctx.addrs, player).and_then(|gd| locator.snapshot(gd).map(|s| (gd, s))) {
        Some((gd, s)) => {
            info!("PlayerGameData {gd:#x} (PlayerIns+{:#x})", locator.game_data_offset().unwrap_or(0));
            let hero = ctx.data.and_then(|d| d.hero_of_chara(s.chara_init));
            info!("chara init {} -> hero {}", s.chara_init, hero.map_or("unknown", |h| HERO_NAMES_ZH[h]));
            if let Some(h) = hero {
                let vessel = equip_data::equipped_vessel(gd, h as u32 + 1);
                let name = vessel.and_then(|v| ctx.data.and_then(|d| d.vessel(v))).map_or("?", |v| v.name.as_str());
                info!("equipped vessel {vessel:?} {name}");
            }
            info!("level {} base stats {:?} plausible={}", s.level, s.stats, s.plausible());
            let bonus: Vec<i64> = s.effective.iter().zip(s.stats).map(|(e, b)| *e as i64 - b as i64).collect();
            info!("effective stats {:?} (bonus from relics + components {:?})", s.effective, bonus);
            let ((hp, hp_max), (fp, fp_max)) = (s.hp(), s.fp());
            info!("HP {hp}/{hp_max}  FP {fp}/{fp_max}  Stamina {}/{}", s.vitals[2][0], s.vitals[2][1]);

            let embedded;
            let growths = match ctx.data {
                Some(d) => &d.growth,
                None => {
                    embedded = vanilla::embedded();
                    &embedded
                }
            };
            if let Some(h) = hero {
                let base = growths[h].stats_at(s.level);
                let diff: Vec<i64> = s.stats.iter().zip(base).map(|(a, b)| *a as i64 - b as i64).collect();
                info!("vanilla {} Lv{} {:?}, memory base - vanilla {:?} (non-zero = stat-change relic)", HERO_NAMES_ZH[h], s.level, base, diff);
            } else if let Some((h, dist)) = vanilla::identify(growths, s.level, &s.stats) {
                info!("closest vanilla hero by stats: {} (distance {dist})", HERO_NAMES_ZH[h]);
            }
            if let Some(d) = ctx.data {
                info!("active respec {:?}, stat-change relics {:?}", d.active_replacement(&speffects), d.active_modifiers(&speffects));
            }
        }
        None => warn!("PlayerGameData not found in PlayerIns"),
    }

    let ours: Vec<i32> = speffects.iter().copied().filter(|&i| is_component(i)).collect();
    info!("speffects: {} total, components {:?} -> delta {:?}", speffects.len(), ours, compose(&ours));
    info!("desired components {:?} -> delta {:?}", ctx.desired, compose(ctx.desired));
    info!("first ids: {:?}", speffects.iter().take(64).collect::<Vec<_>>());

    // 联机时队友也在这个列表里：main=false 且 components 非 0，说明对方的组件同步到了本机
    let players = world::scan_players(ctx.addrs, WCM_SCAN_LEN, CHILD_SCAN_LEN);
    info!("PlayerIns found: {}", players.len());
    for p in players {
        let ours: Vec<i32> = sp_effect::list_ids(p).unwrap_or_default().into_iter().filter(|&i| is_component(i)).collect();
        let data = locator.game_data_cached(ctx.addrs, p).and_then(|gd| locator.snapshot(gd));
        match data.filter(|s| s.plausible()) {
            Some(s) => info!(
                "  {p:#x} main={} lvl={} base {:?} effective {:?} components {:?}",
                p == player, s.level, s.stats, s.effective, compose(&ours)
            ),
            None => info!("  {p:#x} main={} (no player data) components {:?}", p == player, compose(&ours)),
        }
    }
}

/// 两次校准之间保存的数据
#[derive(Default)]
pub struct CalibrationState {
    game_data: Option<Vec<u32>>,
}

/// F8：输出 PlayerGameData 前 0x200 字节（u32），并与上一次校准对比，方便定位字段
pub fn calibrate(addrs: &Addresses, locator: &mut PlayerDataLocator, state: &mut CalibrationState) {
    calibrate_game_data(addrs, locator, &mut state.game_data);
}

fn calibrate_game_data(addrs: &Addresses, locator: &mut PlayerDataLocator, prev: &mut Option<Vec<u32>>) {
    info!("===== CALIBRATE: PlayerGameData =====");
    let Some(player) = world::main_player(addrs) else {
        warn!("main player not available");
        return;
    };
    let Some(gd) = locator.game_data(addrs, player) else {
        warn!("PlayerGameData not found");
        return;
    };
    let values: Vec<u32> = (0..CALIBRATE_LEN / 4).map(|i| read::<u32>(gd + i * 4).unwrap_or(0xDEAD_BEEF)).collect();
    for (row, chunk) in values.chunks(8).enumerate() {
        info!("  +{:03x}: {:?}", row * 32, chunk);
    }
    if let Some(old) = prev.as_ref() {
        for (i, (a, b)) in old.iter().zip(&values).enumerate() {
            if a != b {
                info!("  changed +{:03x}: {} -> {}", i * 4, a, b);
            }
        }
    }
    *prev = Some(values);
}
