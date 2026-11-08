//! 几处共用的文本小工具：BOM、JSON 报错压缩、Windows 宽字符串

use std::io;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;

pub const UTF8_BOM: &[u8] = b"\xEF\xBB\xBF";

/// 去掉开头的 UTF-8 BOM（没有时原样返回）
pub fn strip_utf8_bom(bytes: &[u8]) -> &[u8] {
    bytes.strip_prefix(UTF8_BOM).unwrap_or(bytes)
}

/// JSON 语法错误压成「第 N 行：原因」，与 .me3 的报错一致
pub fn compact_json_error(e: &serde_json::Error) -> String {
    let full = e.to_string();
    let position = format!(" at line {} column {}", e.line(), e.column());
    let reason = full.strip_suffix(&position).unwrap_or(&full);
    format!("第 {} 行：{reason}", e.line())
}

/// 以 NUL 结尾的 UTF-16
pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain([0]).collect()
}

/// 以 NUL 结尾的 UTF-16；路径自带 NUL 会被 API 截断，直接报错
pub fn wide_path(path: &Path) -> io::Result<Vec<u16>> {
    let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    if wide.contains(&0) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "路径含 NUL 字符",
        ));
    }
    wide.push(0);
    Ok(wide)
}
