//! 方案文件 schemes.toml（DLL 同目录）：按"角色 + 器皿"绑定，vessel 不写即为该角色的默认方案。
//! 角色名可写中文或英文；vessel 写器皿中文名或 ID；level1 没写的项取原版 1 级数值；
//! growth 为 2~15 级的成长权重（不能写感应）；[scheme.levels] 下按等级手动指定该级新增点，没分配完的点按 growth 补齐。
//! weapons = { main, off } 为该角色 + 器皿出征时的初始武器（不写为原版）。
//! 感应减少的点固定从 15 级倒序返还（见 stats::rules），旧版的 arcane_release 写法仍能读取但会被忽略。

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;

use crate::stats::growth::MAX_LEVEL;
use crate::stats::heroes::{HERO_NAMES_ZH, parse_hero};
use crate::stats::rules::{Gains, SchemeSpec, WeaponPick};

mod write;

pub use write::{VesselLookup, remove, save};

pub const FILE_NAME: &str = "schemes.toml";
/// 属性键名，与下面 Level1 / GrowthStats 的字段名一致（写回时使用）
const STAT_KEYS: [&str; 8] = ["vigor", "mind", "endurance", "strength", "dexterity", "intelligence", "faith", "arcane"];

#[derive(Deserialize, Default)]
#[serde(default)]
struct SchemeFile {
    scheme: Vec<SchemeEntry>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SchemeEntry {
    hero: String,
    #[serde(default)]
    vessel: Option<String>,
    /// 已废弃，只为兼容旧文件
    #[serde(default)]
    arcane_release: Option<String>,
    #[serde(default)]
    level1: Level1,
    #[serde(default)]
    growth: GrowthStats,
    #[serde(default)]
    levels: BTreeMap<String, GrowthStats>,
    #[serde(default)]
    weapons: Weapons,
}

/// 初始武器：weapons = { main = "执行者", off = "守护者副手" }
#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields, default)]
struct Weapons {
    main: Option<String>,
    off: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields, default)]
struct Level1 {
    vigor: Option<u32>,
    mind: Option<u32>,
    endurance: Option<u32>,
    strength: Option<u32>,
    dexterity: Option<u32>,
    intelligence: Option<u32>,
    faith: Option<u32>,
    arcane: Option<u32>,
}

/// 只有 7 项：感应写在这里会因 deny_unknown_fields 报错（感应只能在 1 级分配）
#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields, default)]
struct GrowthStats {
    vigor: u32,
    mind: u32,
    endurance: u32,
    strength: u32,
    dexterity: u32,
    intelligence: u32,
    faith: u32,
}

impl GrowthStats {
    fn to_array(&self) -> Gains {
        [self.vigor, self.mind, self.endurance, self.strength, self.dexterity, self.intelligence, self.faith]
    }
}

pub struct LoadedScheme {
    pub hero: usize,
    /// 器皿写法原文，由调用方结合游戏数据解析；None 为该角色默认方案
    pub vessel: Option<String>,
    pub spec: SchemeSpec,
}

/// 文件读不到时返回空列表；格式错误返回 Err；单个方案的问题记入第二个返回值，不影响其它方案
pub fn load(dir: &Path) -> Result<(Vec<LoadedScheme>, Vec<String>), String> {
    let path = dir.join(FILE_NAME);
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(_) => return Ok((Vec::new(), vec![format!("{FILE_NAME} not found, no schemes active")])),
    };
    let file: SchemeFile = toml::from_str(&text).map_err(|e| format!("{FILE_NAME} parse error: {e}"))?;

    let mut out: Vec<LoadedScheme> = Vec::new();
    let mut problems = Vec::new();
    for (n, entry) in file.scheme.into_iter().enumerate() {
        if entry.arcane_release.is_some() {
            problems.push(format!("scheme #{}: arcane_release is obsolete and ignored (lowered arcane is now refunded from Lv15 downward)", n + 1));
        }
        match convert(entry) {
            Ok(s) => {
                if let Some(old) = out.iter_mut().find(|o| o.hero == s.hero && o.vessel == s.vessel) {
                    let target = s.vessel.as_deref().unwrap_or("默认");
                    problems.push(format!("scheme #{}: {} / {target} defined twice, the later one wins", n + 1, HERO_NAMES_ZH[s.hero]));
                    *old = s;
                } else {
                    out.push(s);
                }
            }
            Err(e) => problems.push(format!("scheme #{}: {e}", n + 1)),
        }
    }
    Ok((out, problems))
}

fn convert(entry: SchemeEntry) -> Result<LoadedScheme, String> {
    let hero = parse_hero(&entry.hero).ok_or(format!("unknown hero \"{}\"", entry.hero))?;
    let l = &entry.level1;
    let mut overrides = BTreeMap::new();
    for (key, gains) in &entry.levels {
        let level: u32 = key.trim().parse().map_err(|_| format!("level key \"{key}\" is not a number"))?;
        if !(2..=MAX_LEVEL).contains(&level) {
            return Err(format!("level {level} out of range 2~{MAX_LEVEL}"));
        }
        overrides.insert(level, gains.to_array());
    }
    Ok(LoadedScheme {
        hero,
        vessel: entry.vessel.map(|v| v.trim().to_string()).filter(|v| !v.is_empty()),
        spec: SchemeSpec {
            level1: [l.vigor, l.mind, l.endurance, l.strength, l.dexterity, l.intelligence, l.faith, l.arcane],
            weights: entry.growth.to_array(),
            overrides,
            weapons: WeaponPick { main: trimmed(entry.weapons.main), off: trimmed(entry.weapons.off) },
        },
    })
}

/// 空串视为没写
fn trimmed(s: Option<String>) -> Option<String> {
    s.map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_weapons() {
        let text = "[[scheme]]\nhero = \"女爵\"\nvessel = \"女爵杯盅\"\nweapons = { main = \"追踪者\" }\n";
        let file: SchemeFile = toml::from_str(text).unwrap();
        let s = convert(file.scheme.into_iter().next().unwrap()).unwrap();
        assert_eq!(s.spec.weapons, WeaponPick { main: Some("追踪者".into()), off: None });
        assert!(toml::from_str::<SchemeFile>("[[scheme]]\nhero = \"女爵\"\nweapons = { left = \"x\" }\n").is_err());
    }

    #[test]
    fn parses_full_entry() {
        let text = r#"
            [[scheme]]
            hero = "执行者"
            vessel = "执行者杯盅"
            arcane_release = "spread"
            level1 = { vigor = 10, arcane = 20 }
            growth = { vigor = 3, dexterity = 2 }
            [scheme.levels]
            5 = { vigor = 4, dexterity = 6 }
        "#;
        let file: SchemeFile = toml::from_str(text).unwrap();
        let s = convert(file.scheme.into_iter().next().unwrap()).unwrap();
        assert_eq!(s.hero, 7);
        assert_eq!(s.vessel.as_deref(), Some("执行者杯盅"));
        assert_eq!(s.spec.level1[0], Some(10));
        assert_eq!(s.spec.level1[1], None);
        assert_eq!(s.spec.weights, [3, 0, 0, 0, 2, 0, 0]);
        assert_eq!(s.spec.overrides[&5], [4, 0, 0, 0, 6, 0, 0]);
    }

    #[test]
    fn shipped_example_is_valid() {
        use crate::stats::rules::{Limits, build};
        use crate::stats::vanilla::embedded;
        let file: SchemeFile = toml::from_str(include_str!("../../config/schemes.toml")).unwrap();
        let growth = embedded();
        for entry in file.scheme {
            let s = convert(entry).unwrap();
            build(&s.spec, &growth[s.hero], &[], Limits { min: 1, cap: 70 }).unwrap();
        }
    }

    #[test]
    fn rejects_arcane_in_growth() {
        let text = "[[scheme]]\nhero = \"Scholar\"\ngrowth = { arcane = 3 }\n";
        assert!(toml::from_str::<SchemeFile>(text).is_err());
    }
}
