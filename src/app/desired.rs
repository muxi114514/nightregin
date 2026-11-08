//! 决定本地角色此刻应挂的属性差值：差值 = 方案目标 - 原版该级属性。
//! 方案按"角色 + 当前器皿"选取（没有器皿专属方案时用角色默认方案）；
//! 以原版表（而非内存里的基础值）为准，所以原版"属性改变"遗物仍叠加在方案之上；
//! 检测到整表替换（天秤交易、More Map 重塑）时本局暂停方案。

use crate::config::Mode;
use crate::data::GameData;
use crate::game::player_data::PlayerSnapshot;
use crate::stats::STAT_COUNT;
use crate::stats::growth::Stats;
use crate::stats::heroes::HERO_NAMES_ZH;

use super::book::SchemeBook;

pub type Delta = [i32; STAT_COUNT];

#[derive(Debug, Clone, PartialEq)]
pub enum Status {
    Disabled,
    Test(Delta),
    NoData,
    /// regulation 里没有本模组的组件，方案无法生效
    NoComponents,
    NoPlayerData,
    UnknownChara(u32),
    NoScheme { hero: usize, vessel: Option<u32> },
    Suspended { hero: usize, by: i32 },
    Active { hero: usize, vessel: Option<u32>, bound: bool, level: u32, target: Stats, delta: Delta },
}

impl Status {
    pub fn delta(&self) -> Delta {
        match self {
            Status::Test(d) | Status::Active { delta: d, .. } => *d,
            _ => [0; STAT_COUNT],
        }
    }

    pub fn describe(&self, data: Option<&GameData>) -> String {
        let vessel_name = |v: &Option<u32>| match (v, data) {
            (Some(id), Some(d)) => d.vessel(*id).map_or(format!("vessel {id}"), |x| x.name.clone()),
            (Some(id), None) => format!("vessel {id}"),
            (None, _) => "unknown vessel".into(),
        };
        match self {
            Status::Disabled => "disabled (F10)".into(),
            Status::Test(d) => format!("test delta {d:?}"),
            Status::NoData => "game params not readable".into(),
            Status::NoComponents => "regulation lacks NRStatAlloc components".into(),
            Status::NoPlayerData => "player data not readable".into(),
            Status::UnknownChara(id) => format!("unknown CharaInitParam {id}, using vanilla"),
            Status::NoScheme { hero, vessel } => format!("{} / {}: no scheme, vanilla growth", HERO_NAMES_ZH[*hero], vessel_name(vessel)),
            Status::Suspended { hero, by } => format!("{}: suspended by full-table respec SpEffect {by}", HERO_NAMES_ZH[*hero]),
            Status::Active { hero, vessel, bound, level, target, delta } => format!(
                "{} / {} ({}) Lv{level}: target {target:?} delta {delta:?}",
                HERO_NAMES_ZH[*hero],
                vessel_name(vessel),
                if *bound { "vessel scheme" } else { "default scheme" }
            ),
        }
    }
}

pub struct Inputs<'a> {
    pub mode: Mode,
    pub enabled: bool,
    pub test: Delta,
    pub data: Option<&'a GameData>,
    pub book: &'a SchemeBook,
    pub snapshot: Option<&'a PlayerSnapshot>,
    /// 当前装备的器皿（AntiqueStandParam ID），读不到为 None
    pub vessel: Option<u32>,
    pub speffects: &'a [i32],
}

pub fn decide(i: &Inputs) -> Status {
    if !i.enabled {
        return Status::Disabled;
    }
    if i.data.is_some_and(|d| !d.components_ready) {
        return Status::NoComponents;
    }
    if i.mode == Mode::Test {
        return Status::Test(i.test);
    }
    let Some(data) = i.data else { return Status::NoData };
    let Some(snap) = i.snapshot.filter(|s| s.plausible()) else { return Status::NoPlayerData };
    let Some(hero) = data.hero_of_chara(snap.chara_init) else { return Status::UnknownChara(snap.chara_init) };
    if let Some(by) = data.active_replacement(i.speffects) {
        return Status::Suspended { hero, by };
    }
    let Some((alloc, bound)) = i.book.get(hero, i.vessel) else { return Status::NoScheme { hero, vessel: i.vessel } };

    let level = snap.level;
    let target = alloc.at(level);
    let vanilla = data.growth[hero].stats_at(level);
    let delta = std::array::from_fn(|k| target[k] as i32 - vanilla[k] as i32);
    Status::Active { hero, vessel: i.vessel, bound, level, target, delta }
}
