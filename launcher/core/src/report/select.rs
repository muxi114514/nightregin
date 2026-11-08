//! 问题记录要带哪些文件：me3 最近 2 份日志、各模组最近 3 天的日志、崩溃转储的取舍

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use crate::dumpinfo::{DumpKind, classify};
use crate::retry;

pub const RECENT: Duration = Duration::from_secs(72 * 3600);
const ME3_LOGS: usize = 2;
const MAX_DUMPS: usize = 2;
pub const MAX_DUMP_BYTES: u64 = 20 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq)]
pub struct DumpFile {
    pub name: String,
    pub path: PathBuf,
    pub size: u64,
    pub real_crash: bool,
    pub modified: SystemTime,
}

/// 选中的转储，以及放不下的（写进清单）
#[derive(Debug, Default)]
pub struct DumpSelection {
    pub picked: Vec<DumpFile>,
    pub too_big: Vec<DumpFile>,
}

/// 按修改时间最新的 2 份
pub fn latest_me3_logs(dir: &Path) -> Vec<PathBuf> {
    let mut files = list(dir);
    files.sort_by_key(|(_, modified, _)| std::cmp::Reverse(*modified));
    files.truncate(ME3_LOGS);
    files.into_iter().map(|(path, ..)| path).collect()
}

/// 72 小时内修改过的文件，不递归，按名字排
pub fn recent_files(dir: &Path, now: SystemTime) -> Vec<PathBuf> {
    let since = now.checked_sub(RECENT).unwrap_or(SystemTime::UNIX_EPOCH);
    let mut files: Vec<PathBuf> = list(dir)
        .into_iter()
        .filter(|(_, modified, _)| *modified >= since)
        .map(|(path, ..)| path)
        .collect();
    files.sort();
    files
}

/// 目录里的全部文件，不递归
pub fn all_files(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = list(dir).into_iter().map(|(path, ..)| path).collect();
    files.sort();
    files
}

pub fn select_dumps(dir: &Path, span_start: SystemTime) -> Vec<DumpFile> {
    dump_selection(dir, span_start).picked
}

/// 这个整合包每次正常退出都写转储，只按时间取的话真崩溃会被挤掉：真实崩溃排前面，同类按新到旧。
/// 取前 2 个；总大小超限只留第一个；第一个单独就超限时不带
pub fn dump_selection(dir: &Path, span_start: SystemTime) -> DumpSelection {
    let mut dumps: Vec<DumpFile> = list(dir)
        .into_iter()
        .filter(|(path, modified, _)| {
            *modified >= span_start
                && path
                    .extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("dmp"))
        })
        .map(|(path, modified, size)| DumpFile {
            name: path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            real_crash: classify(&path) != DumpKind::ExitCrash,
            path,
            size,
            modified,
        })
        .collect();
    dumps.sort_by_key(|d| (!d.real_crash, std::cmp::Reverse(d.modified)));
    dumps.truncate(MAX_DUMPS);
    let total: u64 = dumps.iter().map(|d| d.size).sum();
    if total > MAX_DUMP_BYTES {
        dumps.truncate(1);
    }
    let (picked, too_big) = dumps.into_iter().partition(|d| d.size <= MAX_DUMP_BYTES);
    DumpSelection { picked, too_big }
}

fn list(dir: &Path) -> Vec<(PathBuf, SystemTime, u64)> {
    let Ok(entries) = retry::transient(|| fs::read_dir(dir)) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let meta = entry.metadata().ok().filter(fs::Metadata::is_file)?;
            Some((entry.path(), meta.modified().ok()?, meta.len()))
        })
        .collect()
}
