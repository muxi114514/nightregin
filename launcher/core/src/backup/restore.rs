//! 恢复存档：先留恢复前备份，把选中的备份写成临时文件核对后，替换存档和游戏自己的 .bak；
//! 中途任何一步失败，已替换的文件都用恢复前备份还原

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use chrono::NaiveDateTime;

use super::publish::{Fingerprint, fingerprint};
use super::{BackupEntry, BackupError, BackupKind, BackupOutcome, BackupStore};
use crate::fsutil::{atomic_write, with_suffix};
use crate::retry;

/// 游戏在存档旁边写的那一份
const GAME_BAK_SUFFIX: &str = ".bak";
const TMP_SUFFIX: &str = ".restore-tmp";

static RESTORE_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug, thiserror::Error)]
pub enum RestoreError {
    #[error("存档正被其他程序占用")]
    SaveBusy,
    #[error("这份备份是空文件，不能恢复")]
    EmptyBackup,
    #[error("恢复失败，已还原到恢复前：{0}")]
    RolledBack(String),
    #[error("恢复失败，还原也失败了，恢复前的存档在 {path}：{message}")]
    RollbackFailed { path: String, message: String },
    #[error("恢复失败：{0}")]
    Io(#[from] io::Error),
}

#[derive(Debug, PartialEq)]
pub struct RestoreOutcome {
    /// 这次的恢复前备份；当前没有存档时为 None
    pub prerestore: Option<BackupEntry>,
}

impl BackupStore {
    pub fn restore(
        &self,
        entry: &BackupEntry,
        now: NaiveDateTime,
    ) -> Result<RestoreOutcome, RestoreError> {
        self.restore_with(entry, now, &|_| Ok(()))
    }

    /// after_replace：每替换完一个文件调一次（测试在这里破坏文件）
    fn restore_with(
        &self,
        entry: &BackupEntry,
        now: NaiveDateTime,
        after_replace: &dyn Fn(&Path) -> io::Result<()>,
    ) -> Result<RestoreOutcome, RestoreError> {
        let _serial = RESTORE_LOCK.lock().unwrap_or_else(PoisonError::into_inner);
        if entry.size == 0 {
            return Err(RestoreError::EmptyBackup);
        }
        let keep = Some(entry.path.as_path());
        let prerestore = match self.create_keeping(BackupKind::PreRestore, now, keep) {
            Ok(BackupOutcome::Created(pre) | BackupOutcome::Unchanged(pre)) => Some(pre),
            Ok(BackupOutcome::NoSave) => None,
            Err(BackupError::SaveBusy) => return Err(RestoreError::SaveBusy),
            Err(BackupError::Io(e)) => return Err(RestoreError::Io(e)),
        };
        let targets = [
            self.save_file.clone(),
            with_suffix(&self.save_file, GAME_BAK_SUFFIX),
        ];
        let print = fingerprint_of(&entry.path)?;
        let tmps = prepare(&entry.path, &targets, &print)?;
        let mut replaced = Vec::new();
        let result = replace_all(&tmps, &targets, &print, &mut replaced, after_replace);
        remove_quietly(&tmps);
        match result {
            Ok(()) => Ok(RestoreOutcome { prerestore }),
            Err(e) => Err(rollback(prerestore.as_ref(), &replaced, &e)),
        }
    }
}

fn fingerprint_of(path: &Path) -> io::Result<Fingerprint> {
    fingerprint(&mut retry::transient(|| fs::File::open(path))?)
}

/// 每个目标旁边放一份临时文件并核对；失败时不留临时文件，现有存档还没动
fn prepare(
    backup: &Path,
    targets: &[PathBuf; 2],
    print: &Fingerprint,
) -> Result<Vec<PathBuf>, RestoreError> {
    let tmps: Vec<PathBuf> = targets.iter().map(|t| with_suffix(t, TMP_SUFFIX)).collect();
    let write = || -> io::Result<()> {
        for tmp in &tmps {
            retry::transient(|| fs::copy(backup, tmp))?;
            if fingerprint_of(tmp)? != *print {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "写入的临时文件与备份内容不一致",
                ));
            }
        }
        Ok(())
    };
    write().inspect_err(|_| remove_quietly(&tmps))?;
    Ok(tmps)
}

/// 逐个替换，全部替换完再逐个核对大小和 SHA-256；replaced 记下动过的文件
fn replace_all(
    tmps: &[PathBuf],
    targets: &[PathBuf; 2],
    print: &Fingerprint,
    replaced: &mut Vec<PathBuf>,
    after_replace: &dyn Fn(&Path) -> io::Result<()>,
) -> io::Result<()> {
    for (tmp, target) in tmps.iter().zip(targets) {
        retry::transient(|| fs::rename(tmp, target))?;
        replaced.push(target.clone());
        after_replace(target)?;
    }
    for target in targets {
        if fingerprint_of(target)? != *print {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "恢复后的文件与备份内容不一致",
            ));
        }
    }
    Ok(())
}

/// 动过的文件写回恢复前的内容；当前原本没有存档时删掉写入的文件
fn rollback(
    prerestore: Option<&BackupEntry>,
    replaced: &[PathBuf],
    cause: &io::Error,
) -> RestoreError {
    let failed = |path: String, e: io::Error| RestoreError::RollbackFailed {
        path,
        message: format!("{cause}；还原时：{e}"),
    };
    match prerestore {
        Some(pre) => {
            let original = match retry::transient(|| fs::read(&pre.path)) {
                Ok(bytes) => bytes,
                Err(e) => return failed(pre.path.display().to_string(), e),
            };
            for target in replaced {
                if let Err(e) = atomic_write(target, &original) {
                    return failed(pre.path.display().to_string(), e);
                }
            }
        }
        None => {
            for target in replaced {
                if let Err(e) = retry::transient(|| fs::remove_file(target)) {
                    return failed(target.display().to_string(), e);
                }
            }
        }
    }
    RestoreError::RolledBack(cause.to_string())
}

fn remove_quietly(paths: &[PathBuf]) {
    for path in paths {
        if let Err(e) = fs::remove_file(path)
            && e.kind() != io::ErrorKind::NotFound
        {
            log::warn!("删除临时文件 {} 失败：{e}", path.display());
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use chrono::NaiveDate;
    use tempfile::TempDir;

    use super::RestoreError;
    use crate::backup::{BackupKind, BackupOutcome, BackupStore};
    use crate::saves::SaveLocation;

    #[test]
    fn verify_mismatch_rolls_back_both() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().to_path_buf();
        let file = dir.join("NR0000.co2");
        let game_bak = PathBuf::from(format!("{}.bak", file.display()));
        let store = BackupStore::new(&SaveLocation {
            account: 1,
            dir: dir.clone(),
            file: file.clone(),
        });
        let now = NaiveDate::from_ymd_opt(2026, 11, 6)
            .unwrap()
            .and_hms_opt(21, 40, 5)
            .unwrap();

        fs::write(&file, b"old").unwrap();
        let BackupOutcome::Created(backup) = store.create(BackupKind::Manual, now).unwrap() else {
            panic!()
        };
        fs::write(&file, b"current").unwrap();
        fs::write(&game_bak, b"game-bak").unwrap();

        // 第二个文件替换完就被破坏：核对对不上
        let broken = game_bak.clone();
        let err = store
            .restore_with(&backup, now, &move |p| {
                if p == broken {
                    fs::write(p, b"corrupted")?;
                }
                Ok(())
            })
            .unwrap_err();
        assert!(matches!(err, RestoreError::RolledBack(_)), "{err}");
        // 动过的两个文件都回到恢复前的存档内容
        assert_eq!(fs::read(&file).unwrap(), b"current");
        assert_eq!(fs::read(&game_bak).unwrap(), b"current");
    }
}
