//! 游戏 exe 的版本：版本资源里的固定文件版本号加文件大小

use std::ffi::c_void;
use std::fs;
use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};
use windows_sys::Win32::Storage::FileSystem::{
    GetFileVersionInfoSizeW, GetFileVersionInfoW, VS_FFI_SIGNATURE, VS_FIXEDFILEINFO,
    VerQueryValueW,
};

use crate::retry;
use crate::text::wide_path;

/// JSON 里写成 {"fileVersion":"1.3.3.0","size":118835320}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameVersion {
    #[serde(with = "dotted_serde")]
    pub file_version: [u16; 4],
    /// 字节数：版本号相同时区分改过的 exe
    pub size: u64,
}

/// 相对路径先按工作目录转绝对（版本 API 对裸文件名会走 DLL 搜索顺序）；文件被短暂占用时重试。
/// 注意：进程没有声明支持 Win10 的清单时，系统自带文件的 10.0 会被报成 6.2（构建号不变），游戏 exe 不受影响
pub fn read_exe_version(exe: &Path) -> io::Result<GameVersion> {
    let exe = std::path::absolute(exe)?;
    let size = retry::transient(|| fs::metadata(&exe))?.len();
    let file_version = retry::transient(|| fixed_file_version(&exe))?;
    Ok(GameVersion { file_version, size })
}

pub fn dotted(v: [u16; 4]) -> String {
    let [a, b, c, d] = v;
    format!("{a}.{b}.{c}.{d}")
}

/// 恰好四段十进制数字，每段 0~65535；不认空白和正负号
pub fn parse_dotted(s: &str) -> Option<[u16; 4]> {
    let mut parts = s.split('.');
    let mut v = [0u16; 4];
    for slot in &mut v {
        *slot = parse_part(parts.next()?)?;
    }
    parts.next().is_none().then_some(v)
}

/// 先逐字查 ASCII 数字：u16 解析会认开头的 +
fn parse_part(s: &str) -> Option<u16> {
    let digits = !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    digits.then(|| s.parse().ok()).flatten()
}

/// 版本资源根块 "\" 里的 VS_FIXEDFILEINFO，按 主.次.构建.修订 排
fn fixed_file_version(exe: &Path) -> io::Result<[u16; 4]> {
    let path = wide_path(exe)?;
    let mut unused = 0u32;
    // SAFETY: path 以 NUL 结尾
    let len = unsafe { GetFileVersionInfoSizeW(path.as_ptr(), &mut unused) };
    if len == 0 {
        return Err(io::Error::last_os_error());
    }
    // 按 u32 分配，保证块按 4 字节对齐
    let mut block = vec![0u32; (len as usize).div_ceil(4)];
    // SAFETY: block 至少有 len 字节可写
    if unsafe { GetFileVersionInfoW(path.as_ptr(), 0, len, block.as_mut_ptr().cast()) } == 0 {
        return Err(io::Error::last_os_error());
    }
    // 查询可能往块里写转换结果，所以用可变指针
    let block_ptr: *const c_void = block.as_mut_ptr().cast_const().cast();
    let root = [u16::from(b'\\'), 0];
    let mut info: *mut c_void = std::ptr::null_mut();
    let mut info_len = 0u32;
    // SAFETY: block_ptr 指向上面填好的版本资源，root 以 NUL 结尾
    let found = unsafe { VerQueryValueW(block_ptr, root.as_ptr(), &mut info, &mut info_len) };
    // 结构体必须完整落在 block 已填的 len 字节内才读
    let need = size_of::<VS_FIXEDFILEINFO>();
    let (start, at) = (block_ptr as usize, info as usize);
    let in_block = at >= start
        && at
            .checked_add(need)
            .is_some_and(|end| end <= start + len as usize);
    if found == 0 || (info_len as usize) < need || !in_block {
        return Err(invalid("没有版本信息"));
    }
    // SAFETY: 上面确认了 [info, info + need) 在 block 内，block 仍存活
    let fixed = unsafe { info.cast::<VS_FIXEDFILEINFO>().read_unaligned() };
    if fixed.dwSignature != VS_FFI_SIGNATURE as u32 {
        return Err(invalid("版本信息签名不对"));
    }
    let (ms, ls) = (fixed.dwFileVersionMS, fixed.dwFileVersionLS);
    Ok([(ms >> 16) as u16, ms as u16, (ls >> 16) as u16, ls as u16])
}

fn invalid(what: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, what)
}

/// [u16; 4] 在 JSON 里写成 "1.3.3.0"
mod dotted_serde {
    use serde::de::Error;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(v: &[u16; 4], s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&super::dotted(*v))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[u16; 4], D::Error> {
        let text = String::deserialize(d)?;
        super::parse_dotted(&text)
            .ok_or_else(|| D::Error::custom(format!("版本号应为四段数字，如 1.3.3.0：{text}")))
    }
}
