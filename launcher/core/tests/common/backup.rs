//! 备份测试共用：固定时间、临时目录里的存档位置、查看备份文件夹

use std::fs::{self, File, OpenOptions};
use std::os::windows::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use chrono::{NaiveDate, NaiveDateTime, TimeDelta};
use launcher_core::backup::{BACKUP_DIR_NAME, BackupEntry, BackupKind, BackupOutcome, BackupStore};
use launcher_core::saves::SaveLocation;
use tempfile::TempDir;

/// 测试用账号
pub const ACCOUNT: u64 = 76561198000000009;

pub fn time(y: i32, mo: u32, d: u32, h: u32, mi: u32, s: u32) -> NaiveDateTime {
    NaiveDate::from_ymd_opt(y, mo, d)
        .unwrap()
        .and_hms_opt(h, mi, s)
        .unwrap()
}

/// 固定的 now：2026-11-06 21:40:05
pub fn now() -> NaiveDateTime {
    time(2026, 11, 6, 21, 40, 5)
}

/// now 之后 n 分钟
pub fn later(minutes: i64) -> NaiveDateTime {
    now() + TimeDelta::minutes(minutes)
}

/// 别的程序独占打开着这个文件，直到返回值析构
pub fn lock_exclusive(path: &Path) -> File {
    OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(path)
        .unwrap()
}

/// <tmp>\<dir_name>\NR0000.co2：建好账号目录，存档不建
pub fn location(tmp: &TempDir, dir_name: &str) -> SaveLocation {
    let dir = tmp.path().join(dir_name);
    fs::create_dir_all(&dir).unwrap();
    SaveLocation {
        account: ACCOUNT,
        file: dir.join("NR0000.co2"),
        dir,
    }
}

pub fn backup_dir(save: &SaveLocation) -> PathBuf {
    save.dir.join(BACKUP_DIR_NAME)
}

/// 备份文件夹里的全部名字（含目录），升序；文件夹不存在时为空
pub fn names(save: &SaveLocation) -> Vec<String> {
    let dir = backup_dir(save);
    if !dir.exists() {
        return Vec::new();
    }
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    names
}

pub fn file_name(path: &Path) -> &str {
    path.file_name().unwrap().to_str().unwrap()
}

pub fn created(outcome: BackupOutcome) -> BackupEntry {
    match outcome {
        BackupOutcome::Created(entry) => entry,
        other => panic!("应新建备份，实际是 {other:?}"),
    }
}

/// list() 里某一类型的备份，新的在前
pub fn of_kind(store: &BackupStore, kind: BackupKind) -> Vec<BackupEntry> {
    store
        .list()
        .unwrap()
        .into_iter()
        .filter(|entry| entry.kind == kind)
        .collect()
}
