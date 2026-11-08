//! 问题记录脱敏：Windows 用户名（UTF-8、ANSI、UTF-16、8.3 短路径写法）和 SteamID64 换成占位符

use std::env;
use std::path::{Path, PathBuf};
use std::ptr::null;

use windows_sys::Win32::Globalization::{CP_ACP, WideCharToMultiByte};
use windows_sys::Win32::Storage::FileSystem::GetShortPathNameW;

use crate::steam::STEAM_ID64_BASE;
use crate::text::wide_path;

const USER_MARK: &[u8] = "<用户名>".as_bytes();
const ID_MARK: &[u8] = b"<SteamID>";
const STEAM_ID_DIGITS: usize = 17;
const STEAM_ID_PREFIX: &[u8] = b"7656119";
/// 字节数少于这个的用户名只在路径里替换，否则会把日志打烂
const MIN_PLAIN_NAME: usize = 3;
/// 账号号至少这么多位才替换，太短的数字到处都是
const MIN_ACCOUNT_DIGITS: usize = 6;
const USER_DIR_PREFIXES: [&[u8]; 3] = [br"Users\\", br"Users\", b"Users/"];

/// names 是用户名每种写法的字节
#[derive(Clone, Debug)]
pub struct Identities {
    pub names: Vec<Vec<u8>>,
    pub steam_ids: Vec<u64>,
}

impl Identities {
    /// USERNAME、USERPROFILE 末段、短路径末段；各取 UTF-8、ANSI、UTF-16LE
    pub fn from_system(steam_ids: Vec<u64>) -> Identities {
        let mut texts: Vec<String> = Vec::new();
        texts.extend(env::var("USERNAME").ok());
        if let Some(profile) = env::var_os("USERPROFILE").map(PathBuf::from) {
            texts.extend(leaf(&profile));
            texts.extend(short_path(&profile).as_deref().and_then(leaf));
        }
        let mut names: Vec<Vec<u8>> = Vec::new();
        for text in texts.iter().filter(|t| !t.is_empty()) {
            let utf16 = text.encode_utf16().flat_map(u16::to_le_bytes).collect();
            for form in [text.as_bytes().to_vec(), ansi_bytes(text), utf16] {
                if !form.is_empty() && !names.contains(&form) {
                    names.push(form);
                }
            }
        }
        Identities { names, steam_ids }
    }
}

pub struct Sanitizer {
    /// 长的在前，免得短的写法先命中
    plain: Vec<Vec<u8>>,
    path_only: Vec<Vec<u8>>,
    accounts: Vec<Vec<u8>>,
}

impl Sanitizer {
    pub fn new(identities: &Identities) -> Sanitizer {
        let (mut plain, path_only): (Vec<_>, Vec<_>) = identities
            .names
            .iter()
            .filter(|n| !n.is_empty())
            .cloned()
            .partition(|n| n.len() >= MIN_PLAIN_NAME);
        plain.sort_by_key(|n| std::cmp::Reverse(n.len()));
        let accounts = identities
            .steam_ids
            .iter()
            .filter_map(|id| id.checked_sub(STEAM_ID64_BASE))
            .map(|low| low.to_string().into_bytes())
            .filter(|digits| digits.len() >= MIN_ACCOUNT_DIGITS)
            .collect();
        Sanitizer {
            plain,
            path_only,
            accounts,
        }
    }

    /// 按字节处理，非 UTF-8 的内容原样保留
    pub fn apply(&self, input: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(input.len());
        let mut i = 0;
        while i < input.len() {
            if let Some(len) = self.name_at(input, i) {
                out.extend_from_slice(USER_MARK);
                i += len;
            } else if input[i].is_ascii_digit() {
                let run = input[i..].iter().take_while(|b| b.is_ascii_digit()).count();
                let digits = &input[i..i + run];
                if is_steam_id(digits) || self.accounts.iter().any(|a| a == digits) {
                    out.extend_from_slice(ID_MARK);
                } else {
                    out.extend_from_slice(digits);
                }
                i += run;
            } else {
                out.push(input[i]);
                i += 1;
            }
        }
        out
    }

    fn name_at(&self, input: &[u8], i: usize) -> Option<usize> {
        let rest = &input[i..];
        if let Some(name) = self.plain.iter().find(|n| starts_with(rest, n)) {
            return Some(name.len());
        }
        if !after_user_dir(input, i) {
            return None;
        }
        self.path_only
            .iter()
            .find(|n| starts_with(rest, n) && ends_word(rest.get(n.len())))
            .map(Vec::len)
    }
}

fn starts_with(hay: &[u8], name: &[u8]) -> bool {
    hay.len() >= name.len() && hay[..name.len()].eq_ignore_ascii_case(name)
}

fn after_user_dir(input: &[u8], i: usize) -> bool {
    USER_DIR_PREFIXES
        .iter()
        .any(|p| i >= p.len() && input[i - p.len()..i].eq_ignore_ascii_case(p))
}

fn ends_word(next: Option<&u8>) -> bool {
    next.is_none_or(|b| !b.is_ascii_alphanumeric() && *b != b'_' && *b != b'-')
}

fn is_steam_id(digits: &[u8]) -> bool {
    digits.len() == STEAM_ID_DIGITS && digits.starts_with(STEAM_ID_PREFIX)
}

fn leaf(path: &Path) -> Option<String> {
    Some(path.file_name()?.to_string_lossy().into_owned())
}

fn short_path(path: &Path) -> Option<PathBuf> {
    let wide = wide_path(path).ok()?;
    let mut buf = [0u16; 520];
    // SAFETY: wide 以 NUL 结尾；buf 可写 buf.len() 个字符
    let len = unsafe { GetShortPathNameW(wide.as_ptr(), buf.as_mut_ptr(), buf.len() as u32) };
    let len = len as usize;
    (len > 0 && len < buf.len()).then(|| PathBuf::from(String::from_utf16_lossy(&buf[..len])))
}

/// 系统默认代码页的写法（C/C++ 模组的日志常用它）
fn ansi_bytes(text: &str) -> Vec<u8> {
    let wide: Vec<u16> = text.encode_utf16().collect();
    let mut buf = vec![0u8; wide.len() * 4 + 1];
    // SAFETY: wide 和 buf 都按给出的长度访问；默认字符参数为空
    let len = unsafe {
        WideCharToMultiByte(
            CP_ACP,
            0,
            wide.as_ptr(),
            wide.len() as i32,
            buf.as_mut_ptr(),
            buf.len() as i32,
            null(),
            std::ptr::null_mut(),
        )
    };
    buf.truncate(usize::try_from(len).unwrap_or(0));
    buf
}
