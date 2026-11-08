//! 整理问题记录：把日志、配置、版本号和最近的崩溃转储打成一个 zip 放到桌面，文本全部脱敏。
//! 单个文件读不了就跳过并写进清单，不影响整体

pub mod sanitize;
pub mod select;

use std::fs::{self, File};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::ptr::null_mut;
use std::time::SystemTime;

use chrono::NaiveDateTime;
use serde::Serialize;
use windows_sys::Win32::System::Com::CoTaskMemFree;
use windows_sys::Win32::UI::Shell::{FOLDERID_Desktop, SHGetKnownFolderPath};
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

use self::sanitize::{Identities, Sanitizer};
use self::select::{RECENT, all_files, dump_selection, latest_me3_logs, recent_files};
use crate::fsutil::{rename_no_replace, with_suffix};
use crate::paths::{GameDir, PackLayout};
use crate::retry;

/// 单个文本文件最多带这么多，超出取末尾
const MAX_TEXT_BYTES: u64 = 8 * 1024 * 1024;
const NAME_TIME: &str = "%Y-%m-%d_%H%M";

pub struct ReportInputs {
    pub pack: PackLayout,
    pub game: Option<GameDir>,
    pub me3_log_dir: Option<PathBuf>,
    pub desktop: PathBuf,
    pub identities: Identities,
    /// 写进「版本.txt」的内容
    pub versions: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DumpRow {
    pub name: String,
    pub size: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportPreview {
    pub dumps: Vec<DumpRow>,
    pub dump_bytes: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum ReportError {
    #[error("写问题记录失败：{0}")]
    Io(#[from] io::Error),
    #[error("打包失败：{0}")]
    Zip(String),
}

impl From<zip::result::ZipError> for ReportError {
    fn from(e: zip::result::ZipError) -> ReportError {
        ReportError::Zip(e.to_string())
    }
}

/// 崩溃转储没法脱敏，生成前先列给玩家看
pub fn preview(i: &ReportInputs, now: SystemTime) -> ReportPreview {
    let dumps: Vec<DumpRow> = dumps_of(i, now)
        .picked
        .into_iter()
        .map(|d| DumpRow {
            name: d.name,
            size: d.size,
        })
        .collect();
    ReportPreview {
        dump_bytes: dumps.iter().map(|d| d.size).sum(),
        dumps,
    }
}

/// 先写 <名字>.tmp，完成后改名；重名时加 (2)、(3)
pub fn create(
    i: &ReportInputs,
    now: SystemTime,
    local: NaiveDateTime,
) -> Result<PathBuf, ReportError> {
    let base = format!("黄金时代问题记录_{}", local.format(NAME_TIME));
    let target = free_target(&i.desktop, &base);
    let tmp = with_suffix(&target, ".tmp");
    let result = write_zip(i, now, &tmp)
        .and_then(|()| rename_no_replace(&tmp, &target).map_err(ReportError::from));
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result.map(|()| target)
}

/// 桌面位置（OneDrive 重定向也算）；取不到时调用方退回 <整合包>\launcher
pub fn desktop_dir() -> Option<PathBuf> {
    let mut raw = null_mut();
    // SAFETY: 输出指针有效；成功时返回的字符串由 CoTaskMemFree 释放
    let hr = unsafe { SHGetKnownFolderPath(&FOLDERID_Desktop, 0, null_mut(), &mut raw) };
    let path = (hr >= 0 && !raw.is_null()).then(|| {
        // SAFETY: 成功时 raw 是以 NUL 结尾的 UTF-16 串
        let len = (0..).take_while(|&n| unsafe { *raw.add(n) } != 0).count();
        // SAFETY: 上面数过 len 个字符可读
        let units = unsafe { std::slice::from_raw_parts(raw, len) };
        PathBuf::from(String::from_utf16_lossy(units))
    });
    // SAFETY: raw 由 SHGetKnownFolderPath 分配，失败时为空，空指针可以释放
    unsafe { CoTaskMemFree(raw.cast()) };
    path
}

/// %LOCALAPPDATA%\garyttierney\me3\data\logs\褪色燃烧
pub fn me3_log_dir() -> Option<PathBuf> {
    let local = PathBuf::from(std::env::var_os("LOCALAPPDATA")?);
    local
        .is_absolute()
        .then(|| local.join(r"garyttierney\me3\data\logs\褪色燃烧"))
}

fn free_target(desktop: &Path, base: &str) -> PathBuf {
    let first = desktop.join(format!("{base}.zip"));
    if !first.exists() {
        return first;
    }
    (2..)
        .map(|n| desktop.join(format!("{base} ({n}).zip")))
        .find(|p| !p.exists())
        .unwrap_or(first)
}

/// 范围从较旧那份 me3 日志开始；取创建和修改时间里早的那个；没有 me3 日志时用 72 小时
fn dumps_of(i: &ReportInputs, now: SystemTime) -> select::DumpSelection {
    let Some(game) = &i.game else {
        return select::DumpSelection::default();
    };
    let oldest_log = i
        .me3_log_dir
        .as_deref()
        .map(latest_me3_logs)
        .and_then(|logs| logs.last().and_then(|p| fs::metadata(p).ok()));
    let span_start = oldest_log
        .and_then(|m| match (m.created().ok(), m.modified().ok()) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        })
        .unwrap_or_else(|| now.checked_sub(RECENT).unwrap_or(SystemTime::UNIX_EPOCH));
    dump_selection(&game.dump_reports(), span_start)
}

struct Packer {
    zip: ZipWriter<File>,
    options: SimpleFileOptions,
    sanitizer: Sanitizer,
    notes: Vec<String>,
}

fn write_zip(i: &ReportInputs, now: SystemTime, tmp: &Path) -> Result<(), ReportError> {
    let mut p = Packer {
        zip: ZipWriter::new(File::create(tmp)?),
        options: SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated),
        sanitizer: Sanitizer::new(&i.identities),
        notes: Vec::new(),
    };
    if let Some(dir) = &i.me3_log_dir {
        p.add_texts("me3", &latest_me3_logs(dir))?;
    }
    let root = &i.pack.root;
    p.add_texts("launcher", &all_files(&i.pack.logs_dir()))?;
    p.add_texts(
        "StatAlloc",
        &recent_files(&root.join(r"StatAlloc\logs"), now),
    )?;
    p.add_texts("logs", &recent_files(&root.join("logs"), now))?;
    p.add_texts("dll_logs", &recent_files(&root.join(r"dll\logs"), now))?;
    let configs = [
        i.pack.nighter_json(),
        i.pack.chance_setter_ini(),
        i.pack.nrsc_ini(),
        root.join(r"StatAlloc\nr_stat_alloc.toml"),
    ];
    p.add_texts("config", &configs)?;
    let versions = p.sanitizer.apply(i.versions.as_bytes());
    p.add_bytes("版本.txt", &versions)?;
    let dumps = dumps_of(i, now);
    for dump in &dumps.picked {
        p.add_dump(&dump.name, &dump.path)?;
    }
    for dump in &dumps.too_big {
        p.notes
            .push(format!("跳过 {}：{} 字节，超过上限", dump.name, dump.size));
    }
    p.finish()
}

impl Packer {
    fn add_texts(&mut self, prefix: &str, files: &[PathBuf]) -> Result<(), ReportError> {
        for path in files {
            let name = file_name(path);
            match read_tail(path) {
                Ok((bytes, truncated)) => {
                    if truncated {
                        self.notes.push(format!("截断 {prefix}/{name}：只保留末尾"));
                    }
                    let clean = self.sanitizer.apply(&bytes);
                    self.add_bytes(&format!("{prefix}/{name}"), &clean)?;
                }
                Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                Err(e) => self.notes.push(format!("跳过 {prefix}/{name}：{e}")),
            }
        }
        Ok(())
    }

    fn add_bytes(&mut self, entry: &str, bytes: &[u8]) -> Result<(), ReportError> {
        let entry = self.clean_name(entry);
        self.zip.start_file(entry, self.options)?;
        self.zip.write_all(bytes)?;
        Ok(())
    }

    /// 转储不脱敏，但文件名要
    fn add_dump(&mut self, name: &str, path: &Path) -> Result<(), ReportError> {
        let mut file = match retry::transient(|| File::open(path)) {
            Ok(file) => file,
            Err(e) => {
                self.notes.push(format!("跳过 dumps/{name}：{e}"));
                return Ok(());
            }
        };
        let entry = self.clean_name(&format!("dumps/{name}"));
        self.zip.start_file(entry, self.options)?;
        io::copy(&mut file, &mut self.zip)?;
        Ok(())
    }

    fn finish(mut self) -> Result<(), ReportError> {
        let mut text = self.notes.join("\r\n");
        if text.is_empty() {
            text = "没有跳过或截断的文件".to_owned();
        }
        let clean = self.sanitizer.apply(text.as_bytes());
        self.add_bytes("清单.txt", &clean)?;
        let file = self.zip.finish()?;
        file.sync_all()?;
        Ok(())
    }

    fn clean_name(&self, name: &str) -> String {
        String::from_utf8_lossy(&self.sanitizer.apply(name.as_bytes())).into_owned()
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned()
}

/// 整个文件，超过上限时只读末尾；第二项表示截断过
fn read_tail(path: &Path) -> io::Result<(Vec<u8>, bool)> {
    let mut file = retry::transient(|| File::open(path))?;
    let len = file.metadata()?.len();
    let truncated = len > MAX_TEXT_BYTES;
    if truncated {
        file.seek(SeekFrom::Start(len - MAX_TEXT_BYTES))?;
    }
    let mut bytes = Vec::with_capacity(len.min(MAX_TEXT_BYTES) as usize);
    file.read_to_end(&mut bytes)?;
    Ok((bytes, truncated))
}
