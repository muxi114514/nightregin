//! 深夜与联机页的游戏设置：从模组配置读成每行一个结果，改动按文件合并写回

use std::mem::discriminant;

use serde::{Deserialize, Serialize};

use super::everdark::{
    LORD_COUNT, LordOdds, LordsView, apply_factory, apply_lords, check_chance_setter, read_lords,
};
use super::store::ConfigStore;
use super::{ApplyReport, ConfigFile, ConfigWriteError};
use crate::ini::IniDoc;
use crate::nighter::{NighterConfig, NighterDoc};
use crate::profile::ConfigError;

const SCALING: &str = "SCALING";
const HEALTH: &str = "health_scaling";
const DAMAGE: &str = "damage_scaling";
const POSTURE: &str = "posture_scaling";

/// 一行的值：读失败时带上是哪个文件和原因，只影响用到这个文件的行
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum Field<T> {
    Ok { value: T },
    Failed { file: ConfigFile, message: String },
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameSettings {
    pub force_deep_night: Field<bool>,
    pub deep_night_level: Field<u8>,
    pub unlock_deep_night: Field<bool>,
    pub lords: Field<LordsView>,
    /// 永夜王「已挑选」的组合，存在启动器设置里
    pub lord_picks: Option<Vec<u8>>,
    pub health_scaling: Field<u16>,
    pub damage_scaling: Field<u16>,
    pub posture_scaling: Field<u16>,
    pub game_running: bool,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(tag = "key", content = "value", rename_all = "camelCase")]
pub enum SettingChange {
    ForceDeepNight(bool),
    DeepNightLevel(u8),
    UnlockDeepNight(bool),
    Lords(Vec<u8>),
    /// 永夜几率写回整合包默认
    LordsFactory,
    HealthScaling(u16),
    DamageScaling(u16),
    PostureScaling(u16),
}

impl SettingChange {
    /// 层数 1~5；倍率 10~300 且是 10 的倍数；名单 0~7，整理成升序去重
    pub fn validated(self) -> Result<SettingChange, String> {
        match self {
            SettingChange::DeepNightLevel(v) if !(1..=5).contains(&v) => {
                Err(format!("深夜层数超出范围：{v}"))
            }
            SettingChange::HealthScaling(v)
            | SettingChange::DamageScaling(v)
            | SettingChange::PostureScaling(v)
                if !(10..=300).contains(&v) || v % 10 != 0 =>
            {
                Err(format!("倍率超出范围：{v}"))
            }
            SettingChange::Lords(mut list) => {
                if let Some(bad) = list.iter().find(|&&id| id >= LORD_COUNT) {
                    return Err(format!("没有编号为 {bad} 的夜王"));
                }
                list.sort_unstable();
                list.dedup();
                Ok(SettingChange::Lords(list))
            }
            other => Ok(other),
        }
    }

    /// 同一种改动只留最新的一条
    pub fn same_key(&self, other: &SettingChange) -> bool {
        let is_lords =
            |c: &SettingChange| matches!(c, SettingChange::Lords(_) | SettingChange::LordsFactory);
        (is_lords(self) && is_lords(other)) || discriminant(self) == discriminant(other)
    }
}

/// game_running 为 false、lord_picks 为 None，由调用方填
pub fn read_game_settings(store: &ConfigStore) -> GameSettings {
    let nighter = read_nighter(store);
    let nrsc = store
        .read(ConfigFile::Nrsc)
        .map(|bytes| IniDoc::parse(&bytes))
        .map_err(|e| ConfigError::from(e).to_string());
    GameSettings {
        force_deep_night: pick_nighter(&nighter, |c| c.force_enable),
        deep_night_level: pick_nighter(&nighter, |c| c.level),
        unlock_deep_night: pick_nighter(&nighter, |c| c.force_unlock),
        lords: lords_field(store),
        lord_picks: None,
        health_scaling: scaling(&nrsc, HEALTH),
        damage_scaling: scaling(&nrsc, DAMAGE),
        posture_scaling: scaling(&nrsc, POSTURE),
        game_running: false,
    }
}

/// 按文件合并写回。出错时整批由调用方留着重试，重写是幂等的
pub fn apply_changes(
    store: &ConfigStore,
    changes: &[SettingChange],
) -> Result<ApplyReport, ConfigWriteError> {
    let mut report = ApplyReport::default();
    let nighter: Vec<&SettingChange> = changes.iter().filter(|c| is_nighter(c)).collect();
    if !nighter.is_empty() {
        report = report.merge(store.patch(ConfigFile::Nighter, |b| patch_nighter(b, &nighter))?);
    }
    let nrsc: Vec<(&str, u16)> = changes.iter().filter_map(nrsc_entry).collect();
    if !nrsc.is_empty() {
        report = report.merge(store.patch(ConfigFile::Nrsc, |b| patch_nrsc(b, &nrsc))?);
    }
    for change in changes {
        match change {
            SettingChange::Lords(list) => report = report.merge(apply_lords(store, list)?),
            SettingChange::LordsFactory => report = report.merge(apply_factory(store)?),
            _ => {}
        }
    }
    Ok(report)
}

/// 把还没写回的改动盖到读到的值上
pub fn overlay(view: &mut GameSettings, changes: &[SettingChange]) {
    for change in changes {
        match change {
            SettingChange::ForceDeepNight(v) => view.force_deep_night = Field::Ok { value: *v },
            SettingChange::DeepNightLevel(v) => view.deep_night_level = Field::Ok { value: *v },
            SettingChange::UnlockDeepNight(v) => view.unlock_deep_night = Field::Ok { value: *v },
            SettingChange::Lords(v) => set_lords(view, v.clone(), LordOdds::Picked),
            SettingChange::LordsFactory => set_lords(view, Vec::new(), LordOdds::Factory),
            SettingChange::HealthScaling(v) => view.health_scaling = Field::Ok { value: *v },
            SettingChange::DamageScaling(v) => view.damage_scaling = Field::Ok { value: *v },
            SettingChange::PostureScaling(v) => view.posture_scaling = Field::Ok { value: *v },
        }
    }
}

fn set_lords(view: &mut GameSettings, list: Vec<u8>, odds: LordOdds) {
    let has_factory = !matches!(&view.lords, Field::Ok { value } if !value.has_factory);
    view.lords = Field::Ok {
        value: LordsView {
            list,
            odds,
            has_factory,
        },
    };
}

/// 永夜王由 ChanceSetter 决定：写入要用到的键不全时这一行算读取失败
fn lords_field(store: &ConfigStore) -> Field<LordsView> {
    let failed = |message: String| Field::Failed {
        file: ConfigFile::ChanceSetter,
        message,
    };
    let cs = match store.read(ConfigFile::ChanceSetter) {
        Ok(bytes) => IniDoc::parse(&bytes),
        Err(e) => return failed(ConfigError::from(e).to_string()),
    };
    if let Err(message) = check_chance_setter(&cs) {
        return failed(message);
    }
    let factory = std::fs::read(ConfigFile::ChanceSetter.default_path(store.pack()))
        .ok()
        .map(|bytes| IniDoc::parse(&bytes));
    Field::Ok {
        value: read_lords(&cs, factory.as_ref()),
    }
}

fn read_nighter(store: &ConfigStore) -> Result<NighterConfig, String> {
    let bytes = store
        .read(ConfigFile::Nighter)
        .map_err(|e| ConfigError::from(e).to_string())?;
    NighterDoc::parse(&bytes)
        .and_then(|doc| doc.config())
        .map_err(|e| e.to_string())
}

fn pick_nighter<T>(
    nighter: &Result<NighterConfig, String>,
    pick: impl FnOnce(&NighterConfig) -> T,
) -> Field<T> {
    match nighter {
        Ok(config) => Field::Ok {
            value: pick(config),
        },
        Err(message) => Field::Failed {
            file: ConfigFile::Nighter,
            message: message.clone(),
        },
    }
}

/// 必须是整数；超范围或不在网格上的值照实显示，调值时再回到网格
fn scaling(nrsc: &Result<IniDoc, String>, key: &str) -> Field<u16> {
    let failed = |message: String| Field::Failed {
        file: ConfigFile::Nrsc,
        message,
    };
    let doc = match nrsc {
        Ok(doc) => doc,
        Err(message) => return failed(message.clone()),
    };
    match doc.get(SCALING, key) {
        None => failed(format!("缺少 [{SCALING}] {key}")),
        Some(raw) => match raw.parse() {
            Ok(value) => Field::Ok { value },
            Err(_) => failed(format!("{key} 应为整数，实际是 {raw}")),
        },
    }
}

fn is_nighter(change: &SettingChange) -> bool {
    matches!(
        change,
        SettingChange::ForceDeepNight(_)
            | SettingChange::DeepNightLevel(_)
            | SettingChange::UnlockDeepNight(_)
    )
}

fn nrsc_entry(change: &SettingChange) -> Option<(&'static str, u16)> {
    match change {
        SettingChange::HealthScaling(v) => Some((HEALTH, *v)),
        SettingChange::DamageScaling(v) => Some((DAMAGE, *v)),
        SettingChange::PostureScaling(v) => Some((POSTURE, *v)),
        _ => None,
    }
}

fn patch_nighter(bytes: &[u8], changes: &[&SettingChange]) -> Result<Vec<u8>, ConfigWriteError> {
    let malformed =
        |e: ConfigError| ConfigWriteError::Malformed(ConfigFile::Nighter, e.to_string());
    let mut doc = NighterDoc::parse(bytes).map_err(malformed)?;
    for change in changes {
        match change {
            SettingChange::ForceDeepNight(v) => doc.set_force_enable(*v),
            SettingChange::DeepNightLevel(v) => doc.set_level(*v),
            SettingChange::UnlockDeepNight(v) => doc.set_force_unlock(*v),
            _ => Ok(()),
        }
        .map_err(malformed)?;
    }
    Ok(doc.to_bytes())
}

fn patch_nrsc(bytes: &[u8], entries: &[(&str, u16)]) -> Result<Vec<u8>, ConfigWriteError> {
    let mut doc = IniDoc::parse(bytes);
    for (key, value) in entries {
        doc.set(SCALING, key, &value.to_string())
            .map_err(|e| ConfigWriteError::Malformed(ConfigFile::Nrsc, e.to_string()))?;
    }
    Ok(doc.to_bytes())
}
