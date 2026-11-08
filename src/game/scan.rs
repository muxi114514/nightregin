//! 运行时特征码扫描，仅在静态 RVA 校验失败时兜底使用。

use super::memory::{is_readable, read};
use super::module::GameModule;

pub struct Pattern {
    bytes: Vec<Option<u8>>,
}

impl Pattern {
    /// "48 8B ?? E8" 形式，? 或 ?? 为通配
    pub fn parse(text: &str) -> Option<Self> {
        let mut bytes = Vec::new();
        for tok in text.split_whitespace() {
            if tok.chars().all(|c| c == '?') {
                bytes.push(None);
            } else {
                bytes.push(Some(u8::from_str_radix(tok, 16).ok()?));
            }
        }
        (!bytes.is_empty()).then_some(Self { bytes })
    }

    fn matches(&self, hay: &[u8]) -> bool {
        self.bytes.iter().zip(hay).all(|(p, h)| p.is_none_or(|b| b == *h))
    }
}

/// 在第一个 .text 节中查找全部匹配（最多 max_hits 个）
pub fn find_all(module: &GameModule, pattern: &Pattern, max_hits: usize) -> Vec<usize> {
    let Some(text) = module.section(".text") else { return Vec::new() };
    let len = text.end - text.start;
    if !is_readable(text.start, len) {
        return Vec::new();
    }
    let hay = unsafe { std::slice::from_raw_parts(text.start as *const u8, len) };
    let n = pattern.bytes.len();
    let mut hits = Vec::new();
    for i in 0..len.saturating_sub(n) {
        if pattern.matches(&hay[i..i + n]) {
            hits.push(text.start + i);
            if hits.len() >= max_hits {
                break;
            }
        }
    }
    hits
}

/// 解析 rel32：addr 指向 rel32 字段本身
pub fn rel32_target(addr: usize) -> Option<usize> {
    let rel = read::<i32>(addr)? as isize;
    Some((addr as isize + 4 + rel) as usize)
}
