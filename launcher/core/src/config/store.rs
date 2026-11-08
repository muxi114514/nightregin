//! 单个配置文件的写回：每次都在最新的磁盘内容上改，首次改写前留 .launcher-bak，临时文件 + 整个替换

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;
use std::sync::{Mutex, MutexGuard, PoisonError};

use sha2::{Digest, Sha256};

use super::{ApplyReport, ConfigFile, ConfigWriteError};
use crate::fsutil::{atomic_write, with_suffix};
use crate::paths::PackLayout;
use crate::retry;

const BACKUP_SUFFIX: &str = ".launcher-bak";

type Digests = HashMap<ConfigFile, [u8; 32]>;

pub struct ConfigStore {
    pack: PackLayout,
    /// 每个文件上次读到（或写入）内容的摘要；锁同时让全部读写排队
    seen: Mutex<Digests>,
}

impl ConfigStore {
    pub fn new(pack: PackLayout) -> ConfigStore {
        ConfigStore {
            pack,
            seen: Mutex::new(HashMap::new()),
        }
    }

    pub fn pack(&self) -> &PackLayout {
        &self.pack
    }

    /// 读盘并记下摘要
    pub fn read(&self, file: ConfigFile) -> io::Result<Vec<u8>> {
        let mut seen = self.lock();
        let bytes = retry::transient(|| fs::read(file.path(&self.pack))).inspect_err(|_| {
            seen.remove(&file);
        })?;
        seen.insert(file, digest(&bytes));
        Ok(bytes)
    }

    /// 三个文件和上次读到的还一样吗：内容变了、被删了、新出现了都算变了
    pub fn changed_on_disk(&self) -> bool {
        let seen = self.lock();
        ConfigFile::ALL.iter().any(|file| {
            let now = fs::read(file.path(&self.pack))
                .ok()
                .map(|bytes| digest(&bytes));
            now != seen.get(file).copied()
        })
    }

    /// 读盘 → change → 和磁盘相同就不写 → 首次备份 → 替换
    pub fn patch(
        &self,
        file: ConfigFile,
        change: impl FnOnce(&[u8]) -> Result<Vec<u8>, ConfigWriteError>,
    ) -> Result<ApplyReport, ConfigWriteError> {
        let mut seen = self.lock();
        let path = file.path(&self.pack);
        let disk =
            retry::transient(|| fs::read(&path)).map_err(|e| ConfigWriteError::Io(file, e))?;
        let on_disk = digest(&disk);
        let external_change = seen.get(&file).is_some_and(|known| *known != on_disk);
        let out = change(&disk)?;
        if out == disk {
            seen.insert(file, on_disk);
            return Ok(ApplyReport {
                wrote: false,
                external_change,
            });
        }
        first_backup(&path, &disk).map_err(|e| ConfigWriteError::Io(file, e))?;
        atomic_write(&path, &out).map_err(|e| ConfigWriteError::Io(file, e))?;
        seen.insert(file, digest(&out));
        Ok(ApplyReport {
            wrote: true,
            external_change,
        })
    }

    /// 换成 launcher\defaults 里的出厂副本；原文件存在（哪怕已损坏）时按首次备份的规则留一份
    pub fn restore_default(&self, file: ConfigFile) -> Result<(), ConfigWriteError> {
        let mut seen = self.lock();
        let factory = match retry::transient(|| fs::read(file.default_path(&self.pack))) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                return Err(ConfigWriteError::DefaultsMissing(file));
            }
            Err(e) => return Err(ConfigWriteError::Io(file, e)),
        };
        let path = file.path(&self.pack);
        let io_err = |e| ConfigWriteError::Io(file, e);
        match retry::transient(|| fs::read(&path)) {
            Ok(old) => first_backup(&path, &old).map_err(io_err)?,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                if let Some(dir) = path.parent() {
                    fs::create_dir_all(dir).map_err(io_err)?;
                }
            }
            Err(e) => return Err(ConfigWriteError::Io(file, e)),
        }
        atomic_write(&path, &factory).map_err(io_err)?;
        seen.insert(file, digest(&factory));
        Ok(())
    }

    /// 持锁时只做赋值和文件操作，中毒时数据仍完整，照常取用
    fn lock(&self) -> MutexGuard<'_, Digests> {
        self.seen.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// 备份只写一次，之后不再覆盖它
fn first_backup(path: &Path, original: &[u8]) -> io::Result<()> {
    let bak = with_suffix(path, BACKUP_SUFFIX);
    if bak.exists() {
        return Ok(());
    }
    atomic_write(&bak, original)
}

fn digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}
