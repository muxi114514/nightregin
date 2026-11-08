//! 启动器管理的三个模组配置：读写规则在 store，游戏设置在 game，永夜王在 everdark

use std::fmt;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::paths::PackLayout;

pub mod everdark;
pub mod game;
pub mod queue;
pub mod service;
pub mod store;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ConfigFile {
    Nighter,
    Nrsc,
    ChanceSetter,
}

impl ConfigFile {
    pub const ALL: [ConfigFile; 3] = [
        ConfigFile::Nighter,
        ConfigFile::Nrsc,
        ConfigFile::ChanceSetter,
    ];

    pub fn path(self, pack: &PackLayout) -> PathBuf {
        match self {
            ConfigFile::Nighter => pack.nighter_json(),
            ConfigFile::Nrsc => pack.nrsc_ini(),
            ConfigFile::ChanceSetter => pack.chance_setter_ini(),
        }
    }

    /// launcher\defaults 下的出厂副本
    pub fn default_path(self, pack: &PackLayout) -> PathBuf {
        let name = match self {
            ConfigFile::Nighter => "nighter.json",
            ConfigFile::Nrsc => "nrsc_settings.ini",
            ConfigFile::ChanceSetter => "ChanceSetter.ini",
        };
        pack.defaults_dir().join(name)
    }
}

/// 给玩家看的名字：配置文件名只在开发模式显示
impl fmt::Display for ConfigFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            ConfigFile::Nighter => "深夜设置",
            ConfigFile::Nrsc => "无缝联机设置",
            ConfigFile::ChanceSetter => "夜王几率设置",
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigWriteError {
    #[error("{0}读写失败：{1}")]
    Io(ConfigFile, std::io::Error),
    #[error("{0}内容有误：{1}")]
    Malformed(ConfigFile, String),
    #[error("整合包缺少{0}的出厂配置，请重新解压")]
    DefaultsMissing(ConfigFile),
    #[error("读不到启动配置的存档后缀")]
    NoMe3Suffix,
}

impl ConfigWriteError {
    /// 文件缺失、内容不对：重试也不会好，不留在队列里
    pub fn is_permanent(&self) -> bool {
        match self {
            ConfigWriteError::Io(_, e) => e.kind() == std::io::ErrorKind::NotFound,
            ConfigWriteError::Malformed(..)
            | ConfigWriteError::DefaultsMissing(_)
            | ConfigWriteError::NoMe3Suffix => true,
        }
    }
}

/// 一次写回的结果；external_change：写之前发现磁盘内容被别的程序改过
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ApplyReport {
    pub wrote: bool,
    pub external_change: bool,
}

impl ApplyReport {
    /// 同一批写了多个文件时合并
    pub fn merge(self, other: ApplyReport) -> ApplyReport {
        ApplyReport {
            wrote: self.wrote || other.wrote,
            external_change: self.external_change || other.external_change,
        }
    }
}
