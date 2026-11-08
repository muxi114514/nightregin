//! 备份文件名：<stem>_YYYY-MM-DD_HHMMSS[-n]_<类型>.<ext>.bak 的拼写与解析

use std::fs;
use std::path::Path;

use chrono::NaiveDateTime;

use super::{BackupEntry, BackupKind, SortKey};

/// 文件名里的时间，定长 17 个字符：2026-11-06_214005
const TIME_FORMAT: &str = "%Y-%m-%d_%H%M%S";
const TIME_LEN: usize = 17;

/// 文件名里随存档变的两头：<stem>_ 和 .<ext>.bak（存档没有扩展名时为 .bak）
pub(super) struct Naming {
    prefix: String,
    suffix: String,
}

impl Naming {
    /// 只取文件名部分：savefile 没校验过，可能带目录
    pub(super) fn of(save_file: &Path) -> Naming {
        let stem = save_file.file_stem().unwrap_or_default().to_string_lossy();
        let suffix = match save_file.extension().filter(|ext| !ext.is_empty()) {
            Some(ext) => format!(".{}.bak", ext.to_string_lossy()),
            None => ".bak".to_owned(),
        };
        Naming {
            prefix: format!("{stem}_"),
            suffix,
        }
    }

    pub(super) fn name(&self, kind: BackupKind, at: NaiveDateTime, seq: u32) -> String {
        let seq = if seq == 0 {
            String::new()
        } else {
            format!("-{seq}")
        };
        let time = at.format(TIME_FORMAT);
        format!("{}{time}{seq}_{}{}", self.prefix, kind.tag(), self.suffix)
    }

    /// name 的逆运算，只认 name 写得出的规范写法；返回类型、时间和序号
    pub(super) fn parse(&self, name: &str) -> Option<(BackupKind, NaiveDateTime, u32)> {
        let middle = name
            .strip_prefix(&self.prefix)?
            .strip_suffix(&self.suffix)?;
        let (stamp, tag) = middle.rsplit_once('_')?;
        let kind = BackupKind::from_tag(tag)?;
        let time = stamp.get(..TIME_LEN)?;
        let at = NaiveDateTime::parse_from_str(time, TIME_FORMAT).ok()?;
        // chrono 解析时位数可多可少，格式化回去一致才算规范
        if at.format(TIME_FORMAT).to_string() != time {
            return None;
        }
        Some((kind, at, parse_seq(stamp.get(TIME_LEN..)?)?))
    }
}

/// 空串为 0；"-n" 为 n（n ≥ 1，不带前导 0）
fn parse_seq(s: &str) -> Option<u32> {
    if s.is_empty() {
        return Some(0);
    }
    let digits = s.strip_prefix('-')?;
    let canonical = digits.bytes().all(|b| b.is_ascii_digit()) && !digits.starts_with('0');
    canonical.then(|| digits.parse().ok()).flatten()
}

/// 名字合格的普通文件
pub(super) fn parse_entry(naming: &Naming, entry: &fs::DirEntry) -> Option<(SortKey, BackupEntry)> {
    let (kind, taken_at, seq) = naming.parse(entry.file_name().to_str()?)?;
    // Windows 上 DirEntry::metadata 不跟随链接，链接的 is_file 为 false
    let meta = entry.metadata().ok().filter(fs::Metadata::is_file)?;
    let key = (taken_at, seq, meta.modified().ok());
    let entry = BackupEntry {
        path: entry.path(),
        kind,
        taken_at,
        size: meta.len(),
    };
    Some((key, entry))
}
