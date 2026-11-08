//! 永夜王：ChanceSetter 是控制方，nighter 只管深夜层数。
//! 选中的夜王每层写 100、没选的写 0；「整合包默认」从 launcher\defaults 写回出厂几率

use serde::Serialize;

use super::store::ConfigStore;
use super::{ApplyReport, ConfigFile, ConfigWriteError};
use crate::ini::IniDoc;
use crate::nighter::NighterDoc;

/// 网格的格数：下标 0~6 同 nighter 编号，7 是哈尔莫妮亚
pub const LORD_COUNT: u8 = 8;

const SECTION: &str = "everdarkChances";
const ENABLED: &str = "enabled";
const DEPTHS: u8 = 5;
pub(crate) const CS_LORDS: [&str; LORD_COUNT as usize] = [
    "Gladius", "Adel", "Gnoster", "Maris", "Libra", "Fulghor", "Caligo", "Harmonia",
];

/// 现在的永夜几率是哪一种
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LordOdds {
    /// 选中必定、没选不会（无 / 已挑选 / 全部）
    Picked,
    /// 和出厂几率相同
    Factory,
    /// 手改的几率，或永夜几率被关掉了
    Custom,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LordsView {
    /// 必定以永夜形态出现的夜王；odds 不是 Picked 时为空
    pub list: Vec<u8>,
    pub odds: LordOdds,
    /// 有出厂副本，能选「整合包默认」
    pub has_factory: bool,
}

pub fn read_lords(cs: &IniDoc, factory: Option<&IniDoc>) -> LordsView {
    let view = |list, odds| LordsView {
        list,
        odds,
        has_factory: factory.is_some(),
    };
    if factory.is_some_and(|f| same_everdark(cs, f)) {
        return view(Vec::new(), LordOdds::Factory);
    }
    let enabled = cs
        .get(SECTION, ENABLED)
        .is_some_and(|v| v.eq_ignore_ascii_case("true"));
    if !enabled {
        return view(Vec::new(), LordOdds::Custom);
    }
    let mut list = Vec::new();
    for (id, lord) in (0u8..).zip(CS_LORDS) {
        match uniform_odds(cs, lord) {
            Some(true) => list.push(id),
            Some(false) => {}
            None => return view(Vec::new(), LordOdds::Custom),
        }
    }
    view(list, LordOdds::Picked)
}

/// 写 enabled = true，再把每层的几率写成 100 / 0；键不全时什么都不写。nighter 不动
pub fn apply_lords(store: &ConfigStore, list: &[u8]) -> Result<ApplyReport, ConfigWriteError> {
    store.patch(ConfigFile::ChanceSetter, |bytes| {
        let mut doc = IniDoc::parse(bytes);
        set(&mut doc, SECTION, ENABLED, "true")?;
        for depth in depth_sections() {
            for (id, lord) in (0u8..).zip(CS_LORDS) {
                set(
                    &mut doc,
                    &depth,
                    lord,
                    if list.contains(&id) { "100" } else { "0" },
                )?;
            }
        }
        Ok(doc.to_bytes())
    })
}

/// 从 launcher\defaults 写回出厂的开关和几率
pub fn apply_factory(store: &ConfigStore) -> Result<ApplyReport, ConfigWriteError> {
    let path = ConfigFile::ChanceSetter.default_path(store.pack());
    let factory = match std::fs::read(path) {
        Ok(bytes) => IniDoc::parse(&bytes),
        Err(_) => return Err(ConfigWriteError::DefaultsMissing(ConfigFile::ChanceSetter)),
    };
    store.patch(ConfigFile::ChanceSetter, |bytes| {
        let mut doc = IniDoc::parse(bytes);
        copy_key(&mut doc, &factory, SECTION, ENABLED)?;
        for depth in depth_sections() {
            for lord in CS_LORDS {
                copy_key(&mut doc, &factory, &depth, lord)?;
            }
        }
        Ok(doc.to_bytes())
    })
}

/// 永夜王写入要用到的键都在吗；缺了就让这一行显示读取失败，别等写入时才失败
pub fn check_chance_setter(cs: &IniDoc) -> Result<(), String> {
    let missing = |section: &str, key: &str| format!("缺少 [{section}] {key}");
    if cs.get(SECTION, ENABLED).is_none() {
        return Err(missing(SECTION, ENABLED));
    }
    for depth in depth_sections() {
        if let Some(lord) = CS_LORDS.iter().find(|lord| cs.get(&depth, lord).is_none()) {
            return Err(missing(&depth, lord));
        }
    }
    Ok(())
}

/// 控制方之外的 nighter：永夜名单要保持出厂值（顺序不算）
pub fn everdark_ok(lords: &[u8], factory: &[u8]) -> bool {
    let sorted = |list: &[u8]| {
        let mut v = list.to_vec();
        v.sort_unstable();
        v
    };
    sorted(lords) == sorted(factory)
}

/// 把 nighter 的永夜名单改回出厂值
pub fn fix_everdark(store: &ConfigStore) -> Result<ApplyReport, ConfigWriteError> {
    let malformed = |e: crate::profile::ConfigError| {
        ConfigWriteError::Malformed(ConfigFile::Nighter, e.to_string())
    };
    let bytes = std::fs::read(ConfigFile::Nighter.default_path(store.pack()))
        .map_err(|_| ConfigWriteError::DefaultsMissing(ConfigFile::Nighter))?;
    let factory = NighterDoc::parse(&bytes)
        .and_then(|doc| doc.config())
        .map_err(malformed)?;
    store.patch(ConfigFile::Nighter, |b| {
        let mut doc = NighterDoc::parse(b).map_err(malformed)?;
        doc.set_super_lords(&factory.super_lords)
            .map_err(malformed)?;
        Ok(doc.to_bytes())
    })
}

fn set(doc: &mut IniDoc, section: &str, key: &str, value: &str) -> Result<(), ConfigWriteError> {
    doc.set(section, key, value)
        .map(drop)
        .map_err(|e| ConfigWriteError::Malformed(ConfigFile::ChanceSetter, e.to_string()))
}

fn copy_key(
    doc: &mut IniDoc,
    factory: &IniDoc,
    section: &str,
    key: &str,
) -> Result<(), ConfigWriteError> {
    let value = factory.get(section, key).ok_or_else(|| {
        ConfigWriteError::Malformed(
            ConfigFile::ChanceSetter,
            format!("出厂配置缺少 [{section}] {key}"),
        )
    })?;
    set(doc, section, key, &value)
}

fn depth_sections() -> impl Iterator<Item = String> {
    (1..=DEPTHS).map(|d| format!("{SECTION}.depth{d}"))
}

/// 这位夜王每层都是 100 为 Some(true)，都是 0 为 Some(false)，否则 None
fn uniform_odds(cs: &IniDoc, lord: &str) -> Option<bool> {
    let odds = depth_sections()
        .map(|depth| cs.get(&depth, lord)?.parse::<f32>().ok())
        .collect::<Option<Vec<f32>>>()?;
    if odds.iter().all(|v| *v == 100.0) {
        Some(true)
    } else if odds.iter().all(|v| *v == 0.0) {
        Some(false)
    } else {
        None
    }
}

fn same_everdark(a: &IniDoc, b: &IniDoc) -> bool {
    a.get(SECTION, ENABLED) == b.get(SECTION, ENABLED)
        && depth_sections().all(|depth| {
            CS_LORDS
                .iter()
                .all(|lord| a.get(&depth, lord) == b.get(&depth, lord))
        })
}
