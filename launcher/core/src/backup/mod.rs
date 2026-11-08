//! 存档备份：<账号目录>\黄金时代备份\<stem>_YYYY-MM-DD_HHMMSS[-n]_<类型>.<ext>.bak；
//! 先写 .tmp、重读核对 SHA-256 再改名，创建后按类型轮换。
//! 只往这个文件夹里写；只删名字合格的备份，以及去掉 .tmp 后名字合格的临时文件

use std::cmp::Reverse;
use std::fs::{self, File};
use std::io::{self, Seek};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};
use std::time::SystemTime;

use chrono::{NaiveDateTime, SubsecRound};
use serde::Serialize;

use self::naming::{Naming, parse_entry};
use self::publish::{
    Fingerprint, TMP_SUFFIX, fingerprint, occupied, open_save, save_error, tmp_path, write_verified,
};
use crate::retry;
use crate::saves::SaveLocation;

mod naming;
pub(crate) mod publish;
pub mod restore;

/// 备份文件夹，在账号目录下
pub const BACKUP_DIR_NAME: &str = "黄金时代备份";

/// 同一秒同名时序号的上限，免得死循环
const MAX_SEQ: u32 = 999;

/// 进程内的创建串行执行：并发的两次创建可能在同一秒挑中同一个名字
static CREATE_LOCK: Mutex<()> = Mutex::new(());

/// 排序键：名字里的时间、序号，再按写入时间（同一秒不同类型的备份名字里分不出先后）
type SortKey = (NaiveDateTime, u32, Option<SystemTime>);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum BackupKind {
    /// 开始游戏前自动做的
    Auto,
    /// 玩家手动做的
    Manual,
    /// 恢复存档前留的
    PreRestore,
}

impl BackupKind {
    /// 文件名里的类型标记
    pub fn tag(self) -> &'static str {
        match self {
            BackupKind::Auto => "auto",
            BackupKind::Manual => "manual",
            BackupKind::PreRestore => "prerestore",
        }
    }

    /// 同类型最多留几份
    pub fn keep(self) -> usize {
        match self {
            BackupKind::Auto => 10,
            BackupKind::Manual | BackupKind::PreRestore => 3,
        }
    }

    fn from_tag(tag: &str) -> Option<BackupKind> {
        [BackupKind::Auto, BackupKind::Manual, BackupKind::PreRestore]
            .into_iter()
            .find(|kind| kind.tag() == tag)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BackupEntry {
    pub path: PathBuf,
    pub kind: BackupKind,
    /// 取自文件名，精确到秒
    pub taken_at: NaiveDateTime,
    /// 字节数
    pub size: u64,
}

#[derive(Debug, PartialEq)]
pub enum BackupOutcome {
    Created(BackupEntry),
    /// 和已有的一份内容相同，没有新建
    Unchanged(BackupEntry),
    /// 存档文件不存在（新玩家）
    NoSave,
}

#[derive(Debug, thiserror::Error)]
pub enum BackupError {
    #[error("存档正被其他程序占用")]
    SaveBusy,
    #[error("备份失败：{0}")]
    Io(#[from] std::io::Error),
}

/// 一个存档文件的全部备份
pub struct BackupStore {
    save_file: PathBuf,
    dir: PathBuf,
}

impl BackupStore {
    /// 备份放在 save.dir\黄金时代备份；名字只取 save.file 的文件名部分
    pub fn new(save: &SaveLocation) -> BackupStore {
        BackupStore {
            save_file: save.file.clone(),
            dir: save.dir.join(BACKUP_DIR_NAME),
        }
    }

    /// 新的在前；文件夹不存在时为空；名字对不上格式的文件、目录和链接都忽略
    pub fn list(&self) -> io::Result<Vec<BackupEntry>> {
        let naming = Naming::of(&self.save_file);
        let mut found: Vec<(SortKey, BackupEntry)> = self
            .dir_entries()?
            .iter()
            .filter_map(|entry| parse_entry(&naming, entry))
            .collect();
        found.sort_by_key(|(key, _)| Reverse(*key));
        Ok(found.into_iter().map(|(_, entry)| entry).collect())
    }

    /// 存档不存在时为 NoSave，被别的程序占用时为 SaveBusy，内容重复时为 Unchanged（规则见 find_same）。
    /// 时间只取 now（截到秒），不读时钟
    pub fn create(
        &self,
        kind: BackupKind,
        now: NaiveDateTime,
    ) -> Result<BackupOutcome, BackupError> {
        self.create_keeping(kind, now, None)
    }

    /// 同 create；keep 这一份不参与轮换（恢复时正要读它）
    fn create_keeping(
        &self,
        kind: BackupKind,
        now: NaiveDateTime,
        keep: Option<&Path>,
    ) -> Result<BackupOutcome, BackupError> {
        let _serial = CREATE_LOCK.lock().unwrap_or_else(PoisonError::into_inner);
        // 持锁时没有正在写的 .tmp，剩下的都是崩溃或断电留下的
        self.remove_stale_tmps();
        let Some(mut save) = open_save(&self.save_file)? else {
            return Ok(BackupOutcome::NoSave);
        };
        let print = fingerprint(&mut save).map_err(save_error)?;
        let existing = self.list()?;
        if let Some(same) = find_same(kind, &existing, &print) {
            return Ok(BackupOutcome::Unchanged(same.clone()));
        }
        let taken_at = now.trunc_subsecs(0);
        let target = self.free_path(kind, taken_at)?;
        retry::transient(|| fs::create_dir_all(&self.dir))?;
        save.rewind()?;
        let size = write_verified(&mut save, &target, &print)?;
        // 尽早放开存档，轮换不需要它
        drop(save);
        self.rotate(kind, &target, keep);
        Ok(BackupOutcome::Created(BackupEntry {
            path: target,
            kind,
            taken_at,
            size,
        }))
    }

    /// 同一秒已有同类型的备份时序号取现有最大值 + 1（轮换删掉旧的后也不会回头，排序才不倒置）；
    /// 同名的文件或它的 .tmp 占着时继续往后找
    fn free_path(&self, kind: BackupKind, at: NaiveDateTime) -> io::Result<PathBuf> {
        let naming = Naming::of(&self.save_file);
        // 年份不是四位时名字读不回来：list 看不到这份备份，轮换也删不掉
        if naming.parse(&naming.name(kind, at, 0)).is_none() {
            let message = format!("备份时间超出范围：{at}");
            return Err(io::Error::new(io::ErrorKind::InvalidInput, message));
        }
        let next = self
            .dir_entries()?
            .iter()
            .filter_map(|e| naming.parse(e.file_name().to_str()?))
            .filter(|(k, t, _)| *k == kind && *t == at)
            .map(|(_, _, seq)| seq + 1)
            .max()
            .unwrap_or(0);
        (next..=MAX_SEQ)
            .map(|seq| self.dir.join(naming.name(kind, at, seq)))
            .find(|path| !occupied(path) && !occupied(&tmp_path(path)))
            .ok_or_else(|| io::Error::new(io::ErrorKind::AlreadyExists, "同一秒内的备份名已用完"))
    }

    /// 同类型只留最新的 keep() 份；刚建的那份不删（时钟往回调过时它的名字最旧）。
    /// 删不掉的记警告，下次创建时再删
    fn rotate(&self, kind: BackupKind, created: &Path, keep: Option<&Path>) {
        let entries = match self.list() {
            Ok(entries) => entries,
            Err(e) => {
                log::warn!("轮换备份时读取 {} 失败：{e}", self.dir.display());
                return;
            }
        };
        let same_kind: Vec<&BackupEntry> = entries.iter().filter(|e| e.kind == kind).collect();
        let excess = same_kind.len().saturating_sub(kind.keep());
        for old in same_kind
            .into_iter()
            .rev()
            .filter(|e| e.path != created && Some(e.path.as_path()) != keep)
            .take(excess)
        {
            if let Err(e) = retry::transient(|| fs::remove_file(&old.path)) {
                log::warn!("删除旧备份 {} 失败：{e}", old.path.display());
            }
        }
    }

    /// 只删去掉 .tmp 后是本存档备份名的普通文件（每个都是一整份存档）；删不掉的记警告
    fn remove_stale_tmps(&self) {
        let entries = match self.dir_entries() {
            Ok(entries) => entries,
            Err(e) => {
                log::warn!("清理临时文件时读取 {} 失败：{e}", self.dir.display());
                return;
            }
        };
        let naming = Naming::of(&self.save_file);
        for entry in entries {
            let ours = entry
                .file_name()
                .to_str()
                .and_then(|name| name.strip_suffix(TMP_SUFFIX))
                .is_some_and(|base| naming.parse(base).is_some());
            // Windows 上 DirEntry::metadata 不跟随链接
            if !ours || !entry.metadata().is_ok_and(|meta| meta.is_file()) {
                continue;
            }
            let path = entry.path();
            if let Err(e) = retry::transient(|| fs::remove_file(&path)) {
                log::warn!("删除残留的临时文件 {} 失败：{e}", path.display());
            }
        }
    }

    /// 备份文件夹里的目录项；文件夹不存在时为空
    fn dir_entries(&self) -> io::Result<Vec<fs::DirEntry>> {
        match retry::transient(|| fs::read_dir(&self.dir)) {
            Ok(entries) => Ok(entries.filter_map(Result::ok).collect()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(e) => Err(e),
        }
    }
}

/// 去重：自动、手动备份只和同类型最新的一份比（手动连按几下不会把更早的挤掉）；
/// 恢复前备份和已有的每一份比（规格：相同就沿用那份）
fn find_same<'a>(
    kind: BackupKind,
    existing: &'a [BackupEntry],
    print: &Fingerprint,
) -> Option<&'a BackupEntry> {
    // existing 新的在前
    let mut same_kind = existing.iter().filter(|entry| entry.kind == kind);
    if kind == BackupKind::PreRestore {
        same_kind.find(|entry| same_content(entry, print))
    } else {
        same_kind.next().filter(|entry| same_content(entry, print))
    }
}

/// 大小不同直接算不同；读不了的备份也算不同（宁可多存一份），记警告
fn same_content(entry: &BackupEntry, print: &Fingerprint) -> bool {
    if entry.size != print.len {
        return false;
    }
    let read =
        retry::transient(|| File::open(&entry.path)).and_then(|mut file| fingerprint(&mut file));
    match read {
        Ok(other) => other == *print,
        Err(e) => {
            log::warn!(
                "读取备份 {} 失败，按内容不同处理：{e}",
                entry.path.display()
            );
            false
        }
    }
}
