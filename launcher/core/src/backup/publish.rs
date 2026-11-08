//! 备份落盘：内容指纹、只共享读地打开存档、写 .tmp 重读核对后改名

use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read};
use std::os::windows::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use windows_sys::Win32::Foundation::{ERROR_LOCK_VIOLATION, ERROR_SHARING_VIOLATION};
use windows_sys::Win32::Storage::FileSystem::FILE_SHARE_READ;

use super::BackupError;
use crate::fsutil::rename_no_replace;
use crate::retry;

/// 写到一半的备份：<目标名>.tmp
pub(super) const TMP_SUFFIX: &str = ".tmp";
/// 哈希时每块的大小
const CHUNK: usize = 64 * 1024;

/// 内容指纹：字节数和 SHA-256
#[derive(PartialEq)]
pub(crate) struct Fingerprint {
    pub(crate) len: u64,
    pub(crate) sha256: [u8; 32],
}

/// 按 64KB 分块读到末尾
pub(crate) fn fingerprint(reader: &mut impl Read) -> io::Result<Fingerprint> {
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; CHUNK];
    let mut len = 0u64;
    loop {
        match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                hasher.update(&buf[..n]);
                len += n as u64;
            }
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
    Ok(Fingerprint {
        len,
        sha256: hasher.finalize().into(),
    })
}

/// 只共享读：有程序开着它写时打不开（算占用），读的过程中也没人能改，哈希和复制读到同一份内容。
/// 不存在时为 None
pub(crate) fn open_save(path: &Path) -> Result<Option<File>, BackupError> {
    let open = || {
        OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ)
            .open(path)
    };
    match retry::transient(open) {
        Ok(file) => Ok(Some(file)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(save_error(e)),
    }
}

/// 共享冲突、区域锁冲突算被占用；拒绝访问分不清原因（权限、是目录、正在删除），原样交出
pub(super) fn save_error(e: io::Error) -> BackupError {
    match e.raw_os_error().map(|code| code as u32) {
        Some(ERROR_SHARING_VIOLATION | ERROR_LOCK_VIOLATION) => BackupError::SaveBusy,
        _ => BackupError::Io(e),
    }
}

/// 复制到 <目标>.tmp 并落盘，重读核对 SHA-256 后改名为目标，返回字节数。
/// .tmp 只新建、不覆盖已有文件；建好之后任一步失败都删掉它
pub(super) fn write_verified(
    save: &mut File,
    target: &Path,
    expected: &Fingerprint,
) -> io::Result<u64> {
    let tmp = tmp_path(target);
    let out = OpenOptions::new().write(true).create_new(true).open(&tmp)?;
    let result =
        fill_and_check(save, out, &tmp, expected).and_then(|()| rename_no_replace(&tmp, target));
    if result.is_err()
        && let Err(e) = retry::transient(|| fs::remove_file(&tmp))
    {
        log::warn!("删除临时文件 {} 失败：{e}", tmp.display());
    }
    result.map(|()| expected.len)
}

/// 写入、落盘、关闭，再重读核对
fn fill_and_check(
    save: &mut File,
    mut out: File,
    tmp: &Path,
    expected: &Fingerprint,
) -> io::Result<()> {
    io::copy(save, &mut out)?;
    out.sync_all()?;
    drop(out);
    let mut written = retry::transient(|| File::open(tmp))?;
    if fingerprint(&mut written)? == *expected {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "写入的备份与存档内容不一致",
        ))
    }
}

/// 同目录、名字后加 .tmp
pub(super) fn tmp_path(target: &Path) -> PathBuf {
    let mut name = OsString::from(target.as_os_str());
    name.push(TMP_SUFFIX);
    PathBuf::from(name)
}

/// 已有同名的文件、目录或链接；查不了也当作有
pub(super) fn occupied(path: &Path) -> bool {
    !matches!(fs::symlink_metadata(path), Err(e) if e.kind() == io::ErrorKind::NotFound)
}

#[cfg(test)]
mod tests {
    use std::fs::{self, File};
    use std::io;
    use std::path::{Path, PathBuf};

    use tempfile::TempDir;

    use super::{Fingerprint, fingerprint, write_verified};

    /// <tmp>\NR0000.co2，内容 "A"；返回存档路径和它的指纹
    fn make_save(tmp: &TempDir) -> (PathBuf, Fingerprint) {
        let path = tmp.path().join("NR0000.co2");
        fs::write(&path, b"A").unwrap();
        let print = fingerprint(&mut File::open(&path).unwrap()).unwrap();
        (path, print)
    }

    fn names(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn verify_mismatch_removes_tmp() {
        let tmp = TempDir::new().unwrap();
        let (save, print) = make_save(&tmp);
        let target = tmp.path().join("NR0000_x.co2.bak");
        let wrong = Fingerprint {
            len: print.len,
            sha256: [0; 32],
        };
        let err = write_verified(&mut File::open(&save).unwrap(), &target, &wrong).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        // 不改名，也不留 .tmp
        assert_eq!(names(tmp.path()), ["NR0000.co2"]);

        // 核对得上才改名
        let size = write_verified(&mut File::open(&save).unwrap(), &target, &print).unwrap();
        assert_eq!(size, 1);
        assert_eq!(fs::read(&target).unwrap(), b"A");
        assert_eq!(names(tmp.path()), ["NR0000.co2", "NR0000_x.co2.bak"]);
    }

    #[test]
    fn existing_tmp_is_never_overwritten() {
        let tmp = TempDir::new().unwrap();
        let (save, print) = make_save(&tmp);
        let target = tmp.path().join("NR0000_x.co2.bak");
        let other = tmp.path().join("NR0000_x.co2.bak.tmp");
        fs::write(&other, b"other").unwrap();
        let err = write_verified(&mut File::open(&save).unwrap(), &target, &print).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);
        // 不是自己建的 .tmp：不覆盖也不删
        assert_eq!(fs::read(&other).unwrap(), b"other");
        assert!(!target.exists());
    }
}
