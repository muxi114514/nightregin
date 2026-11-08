//! 游戏数据：原版成长表、器皿、三维曲线、皮肤（CharaInitParam）-> 角色、整表替换 / 属性改变类 SpEffect。
//! 启动时从游戏内存中已加载的参数表提取（见 load.rs），所以总与实际生效的 regulation 一致，不依赖外部数据文件。

mod inherit_data;
mod load;
mod relic_data;
pub mod relic_names;
mod relic_text;
mod vessel_names;
mod weapon_data;
mod weapon_names;
mod weapon_text;

pub use inherit_data::InheritData;
pub use load::wait_for_game;
pub use relic_data::RelicData;
pub use weapon_data::StartingData;
pub use weapon_names::weapon_name;

use std::collections::{HashMap, HashSet};

use nr_stat_alloc_shared::extract::{CurveRecord, Extracted, UNIVERSAL_VESSEL};

use crate::stats::components::{RESTORE_FP, RESTORE_HP};
use crate::stats::curve::{Curve, VitalCurves};
use crate::stats::growth::HeroGrowth;
use crate::stats::heroes::from_hero_id;
use vessel_names::VESSEL_NAMES;

/// 组件不在 regulation 中时使用的时长（只用于续期间隔检查）
const DEFAULT_LEASE_SECONDS: f32 = 6.0;

/// 器皿（AntiqueStandParam）。hero 为 None 表示所有角色通用的圣杯
#[derive(Debug, Clone)]
pub struct Vessel {
    pub id: u32,
    pub hero: Option<usize>,
    /// 中文名（schemes.toml 里写的是它）
    pub name: String,
    pub name_en: String,
}

impl Vessel {
    /// 当前界面语言下的名字
    pub fn display_name(&self) -> &str {
        crate::lang::pick(self.name.as_str(), self.name_en.as_str())
    }
}

pub struct GameData {
    pub growth: Vec<HeroGrowth>,
    pub vessels: Vec<Vessel>,
    pub vital_curves: Option<VitalCurves>,
    chara_hero: HashMap<u32, usize>,
    replacement: HashSet<i32>,
    modifier: HashSet<i32>,
    pub restore_hp: i32,
    pub restore_fp: i32,
    pub lease_seconds: f32,
    /// regulation 中有本模组的组件（没有时只能编辑方案，不能生效）
    pub components_ready: bool,
}

impl GameData {
    pub fn build(raw: Extracted) -> Result<Self, String> {
        let growth = raw
            .heroes
            .into_iter()
            .map(|h| HeroGrowth::new(h.levels, h.stats).map_err(|e| format!("hero {}: {e}", h.hero_id)))
            .collect::<Result<Vec<_>, _>>()?;
        let chara_hero = raw.chara_hero.into_iter().filter_map(|(id, hero)| Some((id, from_hero_id(hero)?))).collect();
        let vessels = raw
            .vessels
            .into_iter()
            .map(|v| {
                let names = VESSEL_NAMES.iter().find(|n| n.0 == v.id);
                Vessel {
                    id: v.id,
                    hero: (v.hero_type != UNIVERSAL_VESSEL).then(|| from_hero_id(v.hero_type as u32)).flatten(),
                    name: names.map_or(format!("器皿 {}", v.id), |n| n.1.to_string()),
                    name_en: names.map_or(format!("Vessel {}", v.id), |n| n.2.to_string()),
                }
            })
            .collect();
        Ok(Self {
            growth,
            vessels,
            vital_curves: raw.curves.map(vital_curves).transpose()?,
            chara_hero,
            replacement: raw.replacement.into_iter().collect(),
            modifier: raw.modifier.into_iter().collect(),
            restore_hp: RESTORE_HP,
            restore_fp: RESTORE_FP,
            lease_seconds: raw.lease_seconds.unwrap_or(DEFAULT_LEASE_SECONDS),
            components_ready: raw.components_ready,
        })
    }

    /// PlayerGameData 中的当前皮肤 ID -> 角色序号
    pub fn hero_of_chara(&self, chara_init_id: u32) -> Option<usize> {
        self.chara_hero.get(&chara_init_id).copied()
    }

    /// 角色身上第一个"整表替换成长"类效果（天秤交易、More Map 重塑等）
    pub fn active_replacement(&self, speffects: &[i32]) -> Option<i32> {
        speffects.iter().copied().find(|id| self.replacement.contains(id))
    }

    pub fn vessel(&self, id: u32) -> Option<&Vessel> {
        self.vessels.iter().find(|v| v.id == id)
    }

    /// 方案文件里的器皿写法（中文名、英文名或 AntiqueStandParam ID）-> 器皿 ID；必须是该角色专属或通用器皿
    pub fn resolve_vessel(&self, hero: usize, key: &str) -> Result<u32, String> {
        let key = key.trim();
        let found = match key.parse::<u32>() {
            Ok(id) => self.vessel(id),
            Err(_) => self.vessels.iter().find(|v| v.name == key || v.name_en == key),
        };
        match found {
            Some(v) if v.hero.is_none() || v.hero == Some(hero) => Ok(v.id),
            Some(v) => Err(format!("vessel \"{}\" belongs to another hero", v.name)),
            None => Err(format!("unknown vessel \"{key}\"")),
        }
    }

    /// 角色身上的"属性改变"遗物效果（只用于诊断，规则上照常叠加）
    pub fn active_modifiers(&self, speffects: &[i32]) -> Vec<i32> {
        speffects.iter().copied().filter(|id| self.modifier.contains(id)).collect()
    }
}

/// 血量 / 专注值 / 精力三条曲线
fn vital_curves(c: [CurveRecord; 3]) -> Result<VitalCurves, String> {
    let curve = |r: &CurveRecord| Curve::new(r.stage_max_val, r.stage_max_grow_val, r.adj_pt);
    Ok(VitalCurves([curve(&c[0])?, curve(&c[1])?, curve(&c[2])?]))
}

impl crate::scheme_file::VesselLookup for GameData {
    fn resolve(&self, hero: usize, key: &str) -> Option<u32> {
        self.resolve_vessel(hero, key).ok()
    }

    fn name(&self, id: u32) -> Option<&str> {
        self.vessel(id).map(|v| v.name.as_str())
    }
}

#[cfg(test)]
mod tests {
    use nr_stat_alloc_shared::extract::{HeroRecord, VesselRecord};

    use super::*;

    fn sample() -> GameData {
        let hero = |hero_id| HeroRecord { hero_id, levels: vec![1, 15], stats: vec![[7, 2, 3, 5, 8, 2, 2, 28], [40, 20, 30, 50, 60, 10, 10, 28]] };
        let vessel = |id, hero_type| VesselRecord { id, hero_type, goods_id: 0 };
        GameData::build(Extracted {
            heroes: (1..=10).map(hero).collect(),
            vessels: vec![vessel(1000, 1), vessel(8001, 8), vessel(19000, 11), vessel(99999, 8)],
            curves: None,
            chara_hero: vec![(50710, 8), (50050, 1), (1, 99)],
            replacement: vec![46260],
            modifier: vec![],
            components_ready: true,
            lease_seconds: Some(6.0),
        })
        .unwrap()
    }

    #[test]
    fn maps_heroes_and_vessels() {
        let d = sample();
        assert_eq!(d.hero_of_chara(50710), Some(7));
        assert_eq!(d.hero_of_chara(50050), Some(0));
        assert_eq!(d.hero_of_chara(1), None);
        assert!(d.active_replacement(&[1, 46260]).is_some());
        assert_eq!(d.growth[7].stats_at(1), [7, 2, 3, 5, 8, 2, 2, 28]);
        assert_eq!(d.vessel(1000).map(|v| v.name.as_str()), Some("追踪者器皿"));
        assert_eq!(d.resolve_vessel(7, "执行者杯盅"), Ok(8001));
        assert_eq!(d.resolve_vessel(7, "黄金树圣杯"), Ok(19000));
        assert_eq!(d.resolve_vessel(7, "8001"), Ok(8001));
        assert!(d.resolve_vessel(7, "追踪者器皿").is_err());
        // 表里新增、内置名字表没有的器皿
        assert_eq!(d.resolve_vessel(7, "器皿 99999"), Ok(99999));
        assert_eq!(d.vessel(99999).map(|v| v.name_en.as_str()), Some("Vessel 99999"));
    }

    #[test]
    fn name_table_covers_unique_ids() {
        let mut ids: Vec<u32> = VESSEL_NAMES.iter().map(|n| n.0).collect();
        ids.dedup();
        assert_eq!(ids.len(), VESSEL_NAMES.len());
    }
}
