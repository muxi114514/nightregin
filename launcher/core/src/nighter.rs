//! 只读解析深夜模组配置 DepthChange\nighter.json

use std::path::Path;

use serde::Serialize;
use serde_json::Value;
use serde_json::ser::{PrettyFormatter, Serializer};

use crate::profile::ConfigError;
use crate::text::{UTF8_BOM, compact_json_error, strip_utf8_bom};

#[derive(Clone, Debug, PartialEq)]
pub struct NighterConfig {
    /// forceUnlockDeepNight
    pub force_unlock: bool,
    /// forceDeepNight.enable
    pub force_enable: bool,
    /// forceDeepNight.level
    pub level: u8,
    /// superNightLordList（永夜王编号）
    pub super_lords: Vec<u8>,
}

/// nighter.json 全文：保留键顺序、未知字段、BOM、换行风格、末尾换行和缩进宽度
#[derive(Clone, Debug)]
pub struct NighterDoc {
    root: Value,
    crlf: bool,
    bom: bool,
    trailing_newline: bool,
    indent: Vec<u8>,
}

impl NighterDoc {
    pub fn parse(bytes: &[u8]) -> Result<NighterDoc, ConfigError> {
        let body = strip_utf8_bom(bytes);
        let text = std::str::from_utf8(body)
            .map_err(|_| ConfigError::Parse("文件不是 UTF-8 编码".to_owned()))?;
        let root =
            serde_json::from_str(text).map_err(|e| ConfigError::Parse(compact_json_error(&e)))?;
        Ok(NighterDoc {
            root,
            crlf: text.contains("\r\n"),
            bom: body.len() != bytes.len(),
            trailing_newline: text.ends_with('\n'),
            indent: detect_indent(text),
        })
    }

    /// 只读四个键，其余键忽略；缺键或类型不对都算格式错误
    pub fn config(&self) -> Result<NighterConfig, ConfigError> {
        let root = &self.root;
        Ok(NighterConfig {
            force_unlock: field(root, "forceUnlockDeepNight")?
                .as_bool()
                .ok_or_else(|| invalid("forceUnlockDeepNight", "true 或 false"))?,
            force_enable: field(root, "forceDeepNight.enable")?
                .as_bool()
                .ok_or_else(|| invalid("forceDeepNight.enable", "true 或 false"))?,
            level: as_u8(field(root, "forceDeepNight.level")?)
                .ok_or_else(|| invalid("forceDeepNight.level", "0~255 的整数"))?,
            super_lords: field(root, "superNightLordList")?
                .as_array()
                .and_then(|list| list.iter().map(as_u8).collect())
                .ok_or_else(|| invalid("superNightLordList", "0~255 的整数数组"))?,
        })
    }

    pub fn set_force_enable(&mut self, v: bool) -> Result<(), ConfigError> {
        self.put("forceDeepNight.enable", Value::Bool(v))
    }

    pub fn set_level(&mut self, v: u8) -> Result<(), ConfigError> {
        self.put("forceDeepNight.level", Value::from(v))
    }

    pub fn set_force_unlock(&mut self, v: bool) -> Result<(), ConfigError> {
        self.put("forceUnlockDeepNight", Value::Bool(v))
    }

    pub fn set_super_lords(&mut self, list: &[u8]) -> Result<(), ConfigError> {
        self.put("superNightLordList", Value::from(list.to_vec()))
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        let format = PrettyFormatter::with_indent(&self.indent);
        let mut ser = Serializer::with_formatter(&mut out, format);
        // 写进内存的 Value 序列化不会失败
        if let Err(e) = self.root.serialize(&mut ser) {
            log::error!("序列化 nighter.json 失败：{e}");
        }
        if self.trailing_newline {
            out.push(b'\n');
        }
        let mut bytes = Vec::with_capacity(out.len() + 32);
        if self.bom {
            bytes.extend_from_slice(UTF8_BOM);
        }
        for b in out {
            if b == b'\n' && self.crlf {
                bytes.push(b'\r');
            }
            bytes.push(b);
        }
        bytes
    }

    /// 只改已有的键，缺键算格式错误
    fn put(&mut self, key: &str, value: Value) -> Result<(), ConfigError> {
        let slot = self
            .root
            .pointer_mut(&pointer(key))
            .ok_or_else(|| ConfigError::Parse(format!("缺少 {key}")))?;
        *slot = value;
        Ok(())
    }
}

/// 只读四个键，其余键忽略；缺键或类型不对都算格式错误
pub fn read_nighter(path: &Path) -> Result<NighterConfig, ConfigError> {
    NighterDoc::parse(&std::fs::read(path)?)?.config()
}

/// 第一行带缩进的内容用的缩进；没有时 2 个空格（serde_json 默认）
fn detect_indent(text: &str) -> Vec<u8> {
    text.lines()
        .map(|line| {
            let end = line.len() - line.trim_start_matches([' ', '\t']).len();
            &line.as_bytes()[..end]
        })
        .find(|indent| !indent.is_empty())
        .map_or_else(|| b"  ".to_vec(), <[u8]>::to_vec)
}

fn pointer(key: &str) -> String {
    format!("/{}", key.replace('.', "/"))
}

/// key 用点号分层，如 forceDeepNight.level
fn field<'a>(root: &'a Value, key: &str) -> Result<&'a Value, ConfigError> {
    root.pointer(&pointer(key))
        .ok_or_else(|| ConfigError::Parse(format!("缺少 {key}")))
}

fn as_u8(value: &Value) -> Option<u8> {
    value.as_u64().and_then(|n| u8::try_from(n).ok())
}

fn invalid(key: &str, expected: &str) -> ConfigError {
    ConfigError::Parse(format!("{key} 应为 {expected}"))
}
