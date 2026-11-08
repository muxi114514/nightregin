//! 存档位置 %APPDATA%\Nightreign\<SteamID64>\<savefile>；账号按「当前登录 > 记住的 > 唯一的账号目录」确定

use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::backup::BackupError;
use crate::backup::publish::{fingerprint, open_save};
use crate::fsutil::{rename_no_replace, with_suffix};
use crate::retry;

/// 账号目录名（SteamID64）的位数
const ACCOUNT_DIGITS: usize = 17;

/// 一个账号的存档位置；目录和文件都可能还不存在（新玩家）
#[derive(Clone, Debug, PartialEq)]
pub struct SaveLocation {
    /// SteamID64
    pub account: u64,
    /// <根目录>\<SteamID64>
    pub dir: PathBuf,
    /// dir 下的存档文件
    pub file: PathBuf,
}

#[derive(Debug, PartialEq)]
pub enum SaveLocateError {
    /// 没登录、记住的账号没有目录，根目录下也没有账号目录
    NoAccounts,
    /// 同上但账号目录不止一个：不猜，升序列出
    Ambiguous(Vec<u64>),
}

/// %APPDATA%\Nightreign；APPDATA 没设或不是绝对路径时为 None。不管目录在不在
pub fn nightreign_save_root() -> Option<PathBuf> {
    let appdata = PathBuf::from(std::env::var_os("APPDATA")?);
    appdata.is_absolute().then(|| appdata.join("Nightreign"))
}

#[derive(Debug, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum VanillaCopy {
    Copied,
    AlreadyHasSave,
    NoVanilla,
}

/// 同名的原版存档 .sl2
pub fn vanilla_save(save: &SaveLocation) -> PathBuf {
    save.file.with_extension("sl2")
}

/// 从原版存档复制一份：只复制，不覆盖已有存档、不动 sl2；核对大小和 SHA-256 后才改名
pub fn copy_from_vanilla(save: &SaveLocation) -> io::Result<VanillaCopy> {
    if save.file.exists() {
        return Ok(VanillaCopy::AlreadyHasSave);
    }
    let sl2 = vanilla_save(save);
    if !sl2.is_file() {
        return Ok(VanillaCopy::NoVanilla);
    }
    let tmp = with_suffix(&save.file, ".vanilla-tmp");
    let copy = || -> io::Result<()> {
        retry::transient(|| fs::copy(&sl2, &tmp))?;
        let source = fingerprint(&mut retry::transient(|| fs::File::open(&sl2))?)?;
        if fingerprint(&mut retry::transient(|| fs::File::open(&tmp))?)? != source {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "复制出的存档与原版不一致",
            ));
        }
        rename_no_replace(&tmp, &save.file)
    };
    copy().inspect_err(|_| {
        let _ = fs::remove_file(&tmp);
    })?;
    Ok(VanillaCopy::Copied)
}

/// 存档正被别的程序独占
pub fn save_busy(save: &SaveLocation) -> bool {
    matches!(open_save(&save.file), Err(BackupError::SaveBusy))
}

/// 名字恰好是 17 位数字的目录（指向目录的链接也算），升序；根目录不存在或读不了时为空
pub fn account_dirs(root: &Path) -> Vec<u64> {
    let entries = match retry::transient(|| fs::read_dir(root)) {
        Ok(entries) => entries,
        Err(e) => {
            if e.kind() != io::ErrorKind::NotFound {
                log::warn!("读取存档目录 {} 失败：{e}", root.display());
            }
            return Vec::new();
        }
    };
    let mut ids: Vec<u64> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| account_id(&entry.file_name()).filter(|_| entry.path().is_dir()))
        .collect();
    ids.sort_unstable();
    ids
}

/// active 有值就用它（目录不存在也用，存档在不在交给备份判断）；
/// 否则用目录还在的 remembered；否则账号目录恰好一个时用它
pub fn resolve(
    root: &Path,
    savefile: &str,
    active: Option<u64>,
    remembered: Option<u64>,
) -> Result<SaveLocation, SaveLocateError> {
    let chosen = active.or_else(|| remembered.filter(|&id| account_dir(root, id).is_dir()));
    let account = match chosen {
        Some(id) => id,
        None => only_account(root)?,
    };
    let dir = account_dir(root, account);
    Ok(SaveLocation {
        account,
        file: dir.join(savefile),
        dir,
    })
}

fn only_account(root: &Path) -> Result<u64, SaveLocateError> {
    match account_dirs(root).as_slice() {
        [] => Err(SaveLocateError::NoAccounts),
        [only] => Ok(*only),
        many => Err(SaveLocateError::Ambiguous(many.to_vec())),
    }
}

fn account_dir(root: &Path, account: u64) -> PathBuf {
    root.join(account.to_string())
}

/// 先逐字查 ASCII 数字：u64 解析会认开头的 +
fn account_id(name: &OsStr) -> Option<u64> {
    let name = name.to_str()?;
    let digits = name.len() == ACCOUNT_DIGITS && name.bytes().all(|b| b.is_ascii_digit());
    digits.then(|| name.parse().ok()).flatten()
}
