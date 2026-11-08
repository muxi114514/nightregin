//! 文件落盘的两个基本动作：临时文件 + 整个替换、不覆盖的改名

use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use windows_sys::Win32::Storage::FileSystem::{MOVEFILE_WRITE_THROUGH, MoveFileExW};

use crate::retry;
use crate::text::wide_path;

/// 写到一半的文件：<文件名>.launcher-tmp，和目标同目录（同卷改名才是原子的）
const TMP_SUFFIX: &str = ".launcher-tmp";

/// 先写临时文件并刷盘，再整个替换目标；被占用时重试，失败时不留临时文件
pub fn atomic_write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let tmp = with_suffix(path, TMP_SUFFIX);
    let result =
        write_synced(&tmp, bytes).and_then(|()| retry::transient(|| fs::rename(&tmp, path)));
    if result.is_err() {
        // 删不掉也不影响下次覆盖写
        let _ = fs::remove_file(&tmp);
    }
    result
}

/// 同卷改名，目标已存在时报错，不覆盖
pub fn rename_no_replace(from: &Path, to: &Path) -> io::Result<()> {
    let (from, to) = (wide_path(from)?, wide_path(to)?);
    retry::transient(|| {
        // SAFETY: 两个路径都以 NUL 结尾，调用期间有效
        let ok = unsafe { MoveFileExW(from.as_ptr(), to.as_ptr(), MOVEFILE_WRITE_THROUGH) };
        if ok != 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    })
}

/// 同目录、文件名后加后缀
pub(crate) fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = OsString::from(path.as_os_str());
    name.push(suffix);
    PathBuf::from(name)
}

/// 写完刷到磁盘，断电时不会留下半个文件
fn write_synced(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = File::create(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}
