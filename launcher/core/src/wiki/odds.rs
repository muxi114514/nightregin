//! 打开图鉴时现算的几率：夜王和地图取 ChanceSetter，当前层数取 nighter。
//! 手改或缺键时跳过那一项，前端回退到 wiki.json 的文字

use std::collections::BTreeMap;

use serde::Serialize;

use crate::config::everdark::CS_LORDS;
use crate::ini::{IniDoc, read_ini};
use crate::nighter::{NighterConfig, read_nighter};
use crate::paths::PackLayout;

const DEPTHS: usize = 5;
const ENABLED: &str = "enabled";
const NIGHTLORD_SECTION: &str = "nightlordChances";
const EVERDARK_SECTION: &str = "everdarkChances";
const MAP_SECTION: &str = "mapChances";
const NIGHTLORDS: [&str; 10] = [
    "Gladius",
    "Adel",
    "Gnoster",
    "Maris",
    "Libra",
    "Fulghor",
    "Caligo",
    "Heolstor",
    "Harmonia",
    "Straghess",
];
const MAPS: [&str; 6] = [
    "Standard",
    "TheCrater",
    "Mountaintop",
    "RottedWoods",
    "ShroudedCity",
    "GreatHollow",
];

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WikiOdds {
    /// 强制深夜时的层数；没强制或手改出界时 None
    pub level: Option<u8>,
    pub nightlords: Option<BTreeMap<String, [f32; DEPTHS]>>,
    pub maps: Option<BTreeMap<String, f32>>,
    pub everdark: Option<BTreeMap<String, [f32; DEPTHS]>>,
}

pub fn odds_from(cs: Option<&IniDoc>, nighter: Option<&NighterConfig>) -> WikiOdds {
    WikiOdds {
        level: nighter
            .filter(|n| n.force_enable && (1..=DEPTHS as u8).contains(&n.level))
            .map(|n| n.level),
        nightlords: cs.and_then(|c| by_depth(c, NIGHTLORD_SECTION, &NIGHTLORDS)),
        maps: cs.and_then(maps),
        everdark: cs.and_then(|c| by_depth(c, EVERDARK_SECTION, &CS_LORDS)),
    }
}

/// 文件读不了就当没有，几率行回退到 wiki.json 的文字
pub fn read_wiki_odds(pack: &PackLayout) -> WikiOdds {
    let cs = read_ini(&pack.chance_setter_ini())
        .map_err(|e| log::warn!("图鉴读 ChanceSetter 失败：{e}"))
        .ok();
    let nighter = read_nighter(&pack.nighter_json())
        .map_err(|e| log::warn!("图鉴读 nighter 失败：{e}"))
        .ok();
    odds_from(cs.as_ref(), nighter.as_ref())
}

fn enabled(cs: &IniDoc, section: &str) -> bool {
    cs.get(section, ENABLED)
        .is_some_and(|v| v.eq_ignore_ascii_case("true"))
}

fn by_depth(
    cs: &IniDoc,
    section: &str,
    keys: &[&str],
) -> Option<BTreeMap<String, [f32; DEPTHS]>> {
    enabled(cs, section).then(|| {
        keys.iter()
            .filter_map(|key| Some(((*key).to_owned(), row(cs, section, key)?)))
            .collect()
    })
}

/// 五层里有一层读不出来，这一位整个跳过
fn row(cs: &IniDoc, section: &str, key: &str) -> Option<[f32; DEPTHS]> {
    let mut values = [0.0; DEPTHS];
    for (i, slot) in values.iter_mut().enumerate() {
        *slot = number(&cs.get(&format!("{section}.depth{}", i + 1), key)?)?;
    }
    Some(values)
}

fn maps(cs: &IniDoc) -> Option<BTreeMap<String, f32>> {
    enabled(cs, MAP_SECTION).then(|| {
        MAPS.iter()
            .filter_map(|key| {
                let value = number(&cs.get(MAP_SECTION, key)?)?;
                Some(((*key).to_owned(), value))
            })
            .collect()
    })
}

fn number(text: &str) -> Option<f32> {
    text.trim().parse::<f32>().ok().filter(|v| v.is_finite())
}
