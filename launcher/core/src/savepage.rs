//! 存档页：视图和操作都带明确的账号，前端只传账号和备份文件名，碰不到任意路径

use std::path::Path;

use chrono::NaiveDateTime;
use serde::Serialize;

use crate::backup::restore::RestoreError;
use crate::backup::{BackupEntry, BackupError, BackupKind, BackupOutcome, BackupStore};
use crate::os::{GAME_EXE, Os};
use crate::saves::{SaveLocation, account_dirs, resolve, vanilla_save};
use crate::suffix::SuffixView;

const SHOWN_TIME: &str = "%Y-%m-%d %H:%M:%S";

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupRow {
    pub name: String,
    /// auto / manual / prerestore
    pub kind: &'static str,
    /// 2026-11-06 21:40:05
    pub taken_at: String,
    pub size: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavesView {
    pub accounts: Vec<String>,
    pub active: Option<String>,
    pub current: Option<String>,
    pub save_file: Option<String>,
    pub backups: Vec<BackupRow>,
    pub suffix: SuffixView,
    pub can_copy_vanilla: bool,
    pub game_running: bool,
}

pub struct SaveContext<'a> {
    pub root: Option<&'a Path>,
    pub savefile: &'a str,
    pub active: Option<u64>,
    pub remembered: Option<u64>,
    pub os: &'a dyn Os,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum BackupNow {
    /// 游戏运行中，备份可能不完整，要玩家确认
    NeedsConfirm,
    Created {
        row: BackupRow,
    },
    Unchanged {
        row: BackupRow,
    },
    NoSave,
}

#[derive(Debug, thiserror::Error)]
pub enum RestoreRefused {
    #[error("游戏运行中，不能恢复存档")]
    GameRunning,
    #[error("找不到这份备份")]
    NotFound,
    #[error(transparent)]
    Restore(#[from] RestoreError),
}

impl SaveContext<'_> {
    fn game_running(&self) -> bool {
        self.os.find_process(GAME_EXE).is_some()
    }

    /// 指定账号的位置：显式传的账号不需要目录已存在
    pub fn location(&self, account: u64) -> Option<SaveLocation> {
        resolve(self.root?, self.savefile, Some(account), None).ok()
    }

    /// 没有显式账号时：登录账号 > 记下的 > 唯一的账号目录
    fn resolved(&self) -> Option<SaveLocation> {
        resolve(self.root?, self.savefile, self.active, self.remembered).ok()
    }
}

/// 账号：显式传的 account > resolve 的结果
pub fn saves_view(ctx: &SaveContext, account: Option<u64>, suffix: SuffixView) -> SavesView {
    let location = match account {
        Some(id) => ctx.location(id),
        None => ctx.resolved(),
    };
    let backups = location.as_ref().map_or_else(Vec::new, |save| {
        BackupStore::new(save)
            .list()
            .inspect_err(|e| log::warn!("读取备份列表失败：{e}"))
            .unwrap_or_default()
            .iter()
            .map(row)
            .collect()
    });
    let can_copy_vanilla = location
        .as_ref()
        .is_some_and(|save| !save.file.exists() && vanilla_save(save).is_file());
    SavesView {
        accounts: ctx
            .root
            .map_or_else(Vec::new, account_dirs)
            .iter()
            .map(u64::to_string)
            .collect(),
        active: ctx.active.map(|id| id.to_string()),
        current: location.as_ref().map(|save| save.account.to_string()),
        save_file: location
            .as_ref()
            .filter(|save| save.file.exists())
            .map(|save| save.file.display().to_string()),
        backups,
        suffix,
        can_copy_vanilla,
        game_running: ctx.game_running(),
    }
}

/// 游戏运行中且没有确认时交出 NeedsConfirm
pub fn backup_now(
    ctx: &SaveContext,
    account: u64,
    force: bool,
    now: NaiveDateTime,
) -> Result<BackupNow, BackupError> {
    if ctx.game_running() && !force {
        return Ok(BackupNow::NeedsConfirm);
    }
    let Some(save) = ctx.location(account) else {
        return Ok(BackupNow::NoSave);
    };
    Ok(
        match BackupStore::new(&save).create(BackupKind::Manual, now)? {
            BackupOutcome::Created(entry) => BackupNow::Created { row: row(&entry) },
            BackupOutcome::Unchanged(entry) => BackupNow::Unchanged { row: row(&entry) },
            BackupOutcome::NoSave => BackupNow::NoSave,
        },
    )
}

/// name 必须是这个账号备份列表里的文件名；返回这次的恢复前备份
pub fn restore_by_name(
    ctx: &SaveContext,
    account: u64,
    name: &str,
    now: NaiveDateTime,
) -> Result<Option<BackupRow>, RestoreRefused> {
    if ctx.game_running() {
        return Err(RestoreRefused::GameRunning);
    }
    let save = ctx.location(account).ok_or(RestoreRefused::NotFound)?;
    let store = BackupStore::new(&save);
    let entries = store
        .list()
        .map_err(|e| RestoreRefused::Restore(RestoreError::Io(e)))?;
    let entry = entries
        .iter()
        .find(|entry| file_name(entry) == name)
        .ok_or(RestoreRefused::NotFound)?;
    let outcome = store.restore(entry, now)?;
    Ok(outcome.prerestore.as_ref().map(row))
}

fn row(entry: &BackupEntry) -> BackupRow {
    BackupRow {
        name: file_name(entry),
        kind: entry.kind.tag(),
        taken_at: entry.taken_at.format(SHOWN_TIME).to_string(),
        size: entry.size,
    }
}

fn file_name(entry: &BackupEntry) -> String {
    entry
        .path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned()
}
