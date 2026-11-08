//! 存档后缀一致性：.me3 的 savefile、无缝联机 nrsc、ChanceSetter 三方比较

use std::path::Path;

use serde::Serialize;

use crate::config::store::ConfigStore;
use crate::config::{ApplyReport, ConfigFile, ConfigWriteError};
use crate::ini::IniDoc;
use crate::profile::Me3Profile;

/// nrsc_settings.ini 缺失或没写后缀时无缝联机用的后缀
pub const NRSC_DEFAULT_EXT: &str = "co2";
/// ChanceSetter.dll 在 [save] path 缺失时用的默认值
pub const CHANCE_SETTER_DEFAULT_PATH: &str = "auto";
/// ChanceSetter.dll 在 auto 模式下 autoExtension 缺失时用的默认值
pub const CHANCE_SETTER_DEFAULT_EXT: &str = ".dbb";

/// 去两端空白、成对引号和开头的一个点，再转小写：".CL_SAVE" → "cl_save"
pub fn normalize_ext(s: &str) -> String {
    let s = unquote(s);
    s.strip_prefix('.').unwrap_or(s).to_lowercase()
}

/// 两端成对的双引号或单引号（GetPrivateProfileString 也会去掉）
fn unquote(s: &str) -> &str {
    let s = s.trim();
    ['"', '\'']
        .iter()
        .find_map(|&q| s.strip_prefix(q).and_then(|rest| rest.strip_suffix(q)))
        .unwrap_or(s)
}

/// savefile 的扩展名；没有 savefile 时为 None，文件名没有扩展名时为空串
pub fn me3_ext(profile: &Me3Profile) -> Option<String> {
    profile.savefile.as_deref().map(file_ext)
}

/// [SAVE] save_file_extension 原文；文件或键缺失时为 "co2"
pub fn nrsc_ext(doc: Option<&IniDoc>) -> String {
    doc.and_then(|doc| doc.get("SAVE", "save_file_extension"))
        .unwrap_or_else(|| NRSC_DEFAULT_EXT.to_owned())
}

/// [save] path 为 auto（ChanceSetter 比较时不分大小写）时取 autoExtension 原文，
/// 否则取 path 的扩展名。缺键时同 ChanceSetter.dll 取默认值，所以总是 Some
pub fn chance_setter_ext(doc: &IniDoc) -> Option<String> {
    let path = doc
        .get("save", "path")
        .unwrap_or_else(|| CHANCE_SETTER_DEFAULT_PATH.to_owned());
    let path = unquote(&path);
    if path.eq_ignore_ascii_case("auto") {
        let ext = doc.get("save", "autoExtension");
        Some(ext.unwrap_or_else(|| CHANCE_SETTER_DEFAULT_EXT.to_owned()))
    } else {
        Some(file_ext(path))
    }
}

/// 不带点的扩展名，没有时为空串
fn file_ext(name: &str) -> String {
    Path::new(name)
        .extension()
        .map(|ext| ext.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// 三方后缀保留原文给界面显示，比较时才规范化
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SuffixCheck {
    pub me3: Option<String>,
    pub nrsc: String,
    pub chance_setter: Option<String>,
}

impl SuffixCheck {
    /// me3 与 nrsc 不一致：两边会读写不同的存档文件。.me3 没有 savefile 时无从比较
    pub fn blocking(&self) -> bool {
        self.me3
            .as_deref()
            .is_some_and(|me3| !same_ext(me3, &self.nrsc))
    }

    /// ChanceSetter 与 me3 不一致：ChanceSetter 读不到当前存档。任一方未知时不提示
    pub fn warning(&self) -> bool {
        match (&self.me3, &self.chance_setter) {
            (Some(me3), Some(chance_setter)) => !same_ext(me3, chance_setter),
            _ => false,
        }
    }
}

fn same_ext(a: &str, b: &str) -> bool {
    normalize_ext(a) == normalize_ext(b)
}

/// 存档页显示用
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SuffixView {
    pub me3: Option<String>,
    pub nrsc: String,
    pub chance_setter: Option<String>,
    pub consistent: bool,
    pub blocking: bool,
    /// ChanceSetter 的 path 是手动指定的，一键对齐不改它
    pub path_manual: bool,
}

pub fn suffix_view(check: &SuffixCheck, cs: Option<&IniDoc>) -> SuffixView {
    SuffixView {
        me3: check.me3.clone(),
        nrsc: check.nrsc.clone(),
        chance_setter: check.chance_setter.clone(),
        consistent: !check.blocking() && !check.warning(),
        blocking: check.blocking(),
        path_manual: cs.is_some_and(path_is_manual),
    }
}

/// 把 nrsc 和 ChanceSetter 的 autoExtension 对齐到 .me3 的后缀；path 不是 auto 时不改 ChanceSetter。
/// 已经一致（不分大小写、不看前导点）的不写
pub fn align_suffix(store: &ConfigStore, me3_ext: &str) -> Result<ApplyReport, ConfigWriteError> {
    let ext = normalize_ext(me3_ext);
    let nrsc = store.patch(ConfigFile::Nrsc, |bytes| {
        let mut doc = IniDoc::parse(bytes);
        if doc
            .get("SAVE", "save_file_extension")
            .map(|v| normalize_ext(&v))
            != Some(ext.clone())
        {
            doc.set("SAVE", "save_file_extension", &ext)
                .map_err(|e| ConfigWriteError::Malformed(ConfigFile::Nrsc, e.to_string()))?;
        }
        Ok(doc.to_bytes())
    })?;
    let chance = store.patch(ConfigFile::ChanceSetter, |bytes| {
        let mut doc = IniDoc::parse(bytes);
        let same = doc.get("save", "autoExtension").map(|v| normalize_ext(&v)) == Some(ext.clone());
        if !path_is_manual(&doc) && !same {
            doc.set("save", "autoExtension", &format!(".{ext}"))
                .map_err(|e| {
                    ConfigWriteError::Malformed(ConfigFile::ChanceSetter, e.to_string())
                })?;
        }
        Ok(doc.to_bytes())
    })?;
    Ok(nrsc.merge(chance))
}

fn path_is_manual(doc: &IniDoc) -> bool {
    doc.get("save", "path")
        .is_some_and(|path| !unquote(&path).eq_ignore_ascii_case("auto"))
}
