//! 只读解析 me3 配置（.me3，TOML）：存档名与启用的 natives / packages 路径

use std::path::{Path, PathBuf};

use toml_edit::{Document, Item, Table, TableLike, TomlError, Value};

/// 一个 native（DLL）或 package（目录）
#[derive(Clone, Debug, PartialEq)]
pub struct Me3Entry {
    /// 已转成绝对路径
    pub path: PathBuf,
    /// me3 只有 native 有 optional，package 恒为 false
    pub optional: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Me3Profile {
    /// .me3 所在目录（绝对路径）
    pub dir: PathBuf,
    pub savefile: Option<String>,
    /// 只含 enabled 不为 false 的条目，顺序同配置
    pub natives: Vec<Me3Entry>,
    pub packages: Vec<Me3Entry>,
}

/// 读配置文件的错误，各配置模块共用
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("读取失败：{0}")]
    Io(#[from] std::io::Error),
    #[error("格式错误：{0}")]
    Parse(String),
}

/// 两类条目的差别：路径键（package 另认 me3 的别名 source）、有没有 optional
struct EntryKind {
    key: &'static str,
    path_keys: &'static [&'static str],
    has_optional: bool,
}

const NATIVES: EntryKind = EntryKind {
    key: "natives",
    path_keys: &["path"],
    has_optional: true,
};

const PACKAGES: EntryKind = EntryKind {
    key: "packages",
    path_keys: &["path", "source"],
    has_optional: false,
};

impl Me3Profile {
    /// 条目路径相对 .me3 所在目录；我们读的字段类型不对时（me3 同样不认）报格式错误
    pub fn load(path: &Path) -> Result<Me3Profile, ConfigError> {
        let file = std::path::absolute(path)?;
        let text = String::from_utf8(std::fs::read(&file)?)
            .map_err(|_| ConfigError::Parse("文件不是 UTF-8 编码".to_owned()))?;
        let doc = Document::parse(text.as_str()).map_err(|e| syntax_error(&text, &e))?;
        let mut dir = file;
        dir.pop();
        let savefile = doc
            .get("savefile")
            .map(|item| {
                let name = item.as_str().map(str::to_owned);
                name.ok_or_else(|| ConfigError::Parse("savefile 应为字符串".to_owned()))
            })
            .transpose()?;
        Ok(Me3Profile {
            natives: read_entries(&doc, &NATIVES, &dir)?,
            packages: read_entries(&doc, &PACKAGES, &dir)?,
            savefile,
            dir,
        })
    }

    /// 不存在的 natives（optional 的跳过）和 packages，顺序同配置
    pub fn missing_files(&self) -> Vec<PathBuf> {
        self.natives
            .iter()
            .chain(&self.packages)
            .filter(|e| !e.optional && !e.path.exists())
            .map(|e| e.path.clone())
            .collect()
    }
}

/// TOML 语法错误压成一行：「第 N 行：原因」
fn syntax_error(text: &str, e: &TomlError) -> ConfigError {
    let line = e
        .span()
        .and_then(|span| text.as_bytes().get(..span.start))
        .map(|before| before.iter().filter(|&&b| b == b'\n').count() + 1);
    ConfigError::Parse(match line {
        Some(n) => format!("第 {n} 行：{}", e.message()),
        None => e.message().to_owned(),
    })
}

/// 取 kind.key 下启用的条目
fn read_entries(doc: &Table, kind: &EntryKind, dir: &Path) -> Result<Vec<Me3Entry>, ConfigError> {
    let mut entries = Vec::new();
    for (i, table) in tables(doc, kind.key)?.into_iter().enumerate() {
        let entry = read_entry(table, kind, dir)
            .map_err(|why| ConfigError::Parse(format!("{} 第 {} 项：{why}", kind.key, i + 1)))?;
        entries.extend(entry);
    }
    Ok(entries)
}

/// [[key]] 与 key = [{ … }] 两种表数组写法都认；没有这个键时为空
fn tables<'a>(doc: &'a Table, key: &str) -> Result<Vec<&'a dyn TableLike>, ConfigError> {
    let not_tables = || ConfigError::Parse(format!("{key} 应为表数组"));
    match doc.get(key) {
        None => Ok(Vec::new()),
        Some(Item::ArrayOfTables(array)) => Ok(array.iter().map(|t| t as &dyn TableLike).collect()),
        Some(Item::Value(Value::Array(array))) => array
            .iter()
            .map(|v| {
                v.as_inline_table()
                    .map(|t| t as &dyn TableLike)
                    .ok_or_else(not_tables)
            })
            .collect(),
        Some(_) => Err(not_tables()),
    }
}

/// 一个条目；未启用时为 None。Err 是不带位置的原因
fn read_entry(
    table: &dyn TableLike,
    kind: &EntryKind,
    dir: &Path,
) -> Result<Option<Me3Entry>, String> {
    let enabled = get_bool(table, "enabled")?.unwrap_or(true);
    let optional = kind.has_optional && get_bool(table, "optional")?.unwrap_or(false);
    let rel = kind
        .path_keys
        .iter()
        .find_map(|key| table.get(key))
        .ok_or_else(|| format!("缺少 {}", kind.path_keys.join(" 或 ")))?
        .as_str()
        .ok_or("路径应为字符串")?;
    Ok(enabled.then(|| Me3Entry {
        path: resolve(dir, rel),
        optional,
    }))
}

fn get_bool(table: &dyn TableLike, key: &str) -> Result<Option<bool>, String> {
    table
        .get(key)
        .map(|item| {
            item.as_bool()
                .ok_or_else(|| format!("{key} 应为 true 或 false"))
        })
        .transpose()
}

/// 相对路径按 .me3 所在目录解析，顺带去掉 `.` 并统一分隔符
fn resolve(dir: &Path, rel: &str) -> PathBuf {
    dir.join(rel).components().collect()
}
