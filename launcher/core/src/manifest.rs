//! 整合包 manifest.json：读整合包版本、适配的游戏版本、me3 版本和本版内容（计划 4 扩展）

use std::fs;
use std::io;
use std::path::Path;

use serde::Deserialize;

use crate::gamever::GameVersion;
use crate::retry;
use crate::text::strip_utf8_bom;

/// 其余键（如 files）忽略；每一项都可以没有
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ManifestInfo {
    /// 整合包版本，如 1.9.5
    pub version: Option<String>,
    /// 整合包适配的游戏版本，可以有多个
    pub game: Vec<GameVersion>,
    /// 整合包带的 me3 版本
    pub me3: Option<String>,
    /// 本版内容；带图鉴条目 id 的在图鉴里标「新」
    pub notes: Vec<ManifestNote>,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct ManifestNote {
    pub text: String,
    pub wiki: Option<String>,
}

/// 文件不存在（开发时常见）或解析失败时为 None；不存在以外的失败写一条警告
pub fn read_manifest_info(path: &Path) -> Option<ManifestInfo> {
    let bytes = match retry::transient(|| fs::read(path)) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return None,
        Err(e) => {
            log::warn!("读取 {} 失败：{e}", path.display());
            return None;
        }
    };
    let body = strip_utf8_bom(&bytes);
    match serde_json::from_slice(body) {
        Ok(info) => Some(info),
        Err(e) => {
            log::warn!("{} 格式错误：{e}", path.display());
            None
        }
    }
}
