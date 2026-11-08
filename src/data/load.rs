//! 从游戏内存构建 GameData：找到参数仓库 → 等它装载完所需的 6 张表 → 复制 → 提取 → 组装。
//! 同一次读取顺带取武器与遗物重铸相关的表（见 weapon_data.rs、relic_data.rs，缺失时只停用对应功能），
//! 并把参数仓库交给调用方（改写内存中的表用）。
//! 在初始化线程里轮询（游戏启动早期参数还没加载），复制出的表在提取后立即释放。

use std::time::{Duration, Instant};

use log::{info, warn};
use nr_stat_alloc_shared::extract::{Tables, extract};
use nr_stat_alloc_shared::param::layout::{antique_stand, chara_init, correct_graph, hero, hero_status, sp_effect};
use nr_stat_alloc_shared::passive_tiers::PassiveTiers;

use crate::game::addresses::Addresses;
use crate::game::module::GameModule;
use crate::game::params::ParamRepository;
use crate::game::singleton::find_instance;

use super::GameData;
use super::relic_data::{self, RELIC_TABLES, RelicData};
use super::weapon_data::{self, EXTRA_TABLES, WeaponData};

const POLL: Duration = Duration::from_millis(500);
/// .data 整节扫描较慢，没找到仓库时降低频率
const SCAN_POLL: Duration = Duration::from_secs(3);
const LOG_INTERVAL: Duration = Duration::from_secs(30);
/// 必需表到齐后再等其余表的时长：参数表是陆续装载的，太早读会漏掉后装的表（如 SwordArtsParam），对应功能被停用
const EXTRA_GRACE: Duration = Duration::from_secs(30);

pub struct Loaded {
    pub data: GameData,
    pub weapons: WeaponData,
    pub relics: Option<RelicData>,
    pub repository: ParamRepository,
}

/// relic_custom：重铸时本模组新增的追踪者词条随模板词条出现
pub fn wait_for_game(module: &GameModule, addrs: &Addresses, timeout: Duration, relic_custom: bool) -> Result<Loaded, String> {
    let wanted = [sp_effect::TABLE, hero::TABLE, hero_status::TABLE, chara_init::TABLE, antique_stand::TABLE, correct_graph::TABLE];
    let mut names: Vec<&str> = wanted.iter().map(|t| t.struct_name).collect();
    let required = names.len();
    names.extend(EXTRA_TABLES.iter().chain(&RELIC_TABLES).map(|t| t.struct_name));
    let start = Instant::now();
    let mut last_log = Instant::now();
    let mut last_problem = String::new();
    let mut instance = None;
    let mut required_at: Option<Instant> = None;
    while start.elapsed() < timeout {
        if instance.is_none() {
            instance = find_instance(module, addrs.vt_param_repository);
            if let Some((static_addr, p)) = instance {
                info!("ParamRepositoryImp {p:#x} (static rva {:#x})", static_addr - module.base);
            }
        }
        let problem = match instance.map(|(_, repo)| ParamRepository::open(repo, addrs.vt_param_res_cap)) {
            None => "ParamRepositoryImp not created yet".to_string(),
            Some(Ok(repository)) => {
                let tables = repository.read_tables(&names);
                let missing: Vec<&str> = names.iter().copied().filter(|n| !tables.contains_key(*n)).collect();
                let required_ready = names[..required].iter().all(|n| tables.contains_key(*n));
                let waited = required_ready.then(|| required_at.get_or_insert_with(Instant::now).elapsed());
                if waited.is_some_and(|w| missing.is_empty() || w >= EXTRA_GRACE) {
                    if !missing.is_empty() {
                        warn!("optional param tables still missing after {EXTRA_GRACE:?}: {missing:?}");
                    }
                    let loaded = Tables::load(|t| tables.get(t.struct_name).map(Vec::as_slice));
                    let data = loaded.and_then(|t| extract(&t)).and_then(GameData::build);
                    let weapons = weapon_data::build(&tables, &repository);
                    let relics = relic_data::build(&tables, relic_custom);
                    if let Ok(d) = &data {
                        info!(
                            "game params read from memory ({} res caps): {} vessels, components ready={}, lease {}s, {} weapon passives with upper tiers, starting weapons {}",
                            repository.len(),
                            d.vessels.len(),
                            d.components_ready,
                            d.lease_seconds,
                            weapons.passive_tiers.as_ref().map_or(0, PassiveTiers::len),
                            if weapons.starting.is_some() { "ready" } else { "unavailable" }
                        );
                    }
                    return data.map(|data| Loaded { data, weapons, relics, repository });
                }
                format!("{} of {} tables loaded ({} res caps), missing {missing:?}", tables.len(), names.len(), repository.len())
            }
            Some(Err(e)) => e,
        };
        if problem != last_problem || last_log.elapsed() >= LOG_INTERVAL {
            warn!("waiting for game params: {problem}");
            last_problem = problem;
            last_log = Instant::now();
        }
        std::thread::sleep(if instance.is_some() { POLL } else { SCAN_POLL });
    }
    Err(format!("game params not available after {timeout:?}: {last_problem}"))
}
