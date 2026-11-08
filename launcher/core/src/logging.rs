//! 启动器日志 <整合包>\launcher\logs\launcher.log：每次启动轮换，连当前一共留 5 份；
//! 单个文件写满 5MB 就地再轮换；panic 的信息、位置和堆栈也写进来

use std::backtrace::Backtrace;
use std::borrow::Cow;
use std::fmt::{self, Write as _};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::os::windows::fs::OpenOptionsExt;
use std::panic::{self, PanicHookInfo};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock, PoisonError};

use chrono::Local;
use log::{Level, LevelFilter, Log, Metadata, Record};

use crate::retry;

pub const LOG_FILE: &str = "launcher.log";
/// 连当前一共保留的份数：launcher.log、launcher.1.log ~ launcher.4.log
pub const KEEP_FILES: usize = 5;
/// 单个文件的上限
pub const MAX_BYTES: u64 = 5 * 1024 * 1024;

const TIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S%.3f";

static PANIC_CALLBACK: OnceLock<Box<dyn Fn() + Send + Sync>> = OnceLock::new();
static PANIC_REPORTED: AtomicBool = AtomicBool::new(false);

/// 全局 logger；init 之前 writer 为 None，什么都不写
static LOGGER: FileLogger = FileLogger {
    writer: Mutex::new(None),
};

/// launcher.4.log 删掉，launcher.3.log → .4、…、launcher.log → .1；遇到错误即停。
/// 不存在的文件跳过（目录不存在也算成功）；被短暂占用时每步重试几次；
/// launcher.log 一直被别的进程开着时什么都不动，返回错误
pub fn rotate(dir: &Path) -> io::Result<()> {
    let current = dir.join(LOG_FILE);
    ignore_missing(retry::transient(|| ensure_not_in_use(&current)))?;
    let oldest = dir.join(log_name(KEEP_FILES - 1));
    ignore_missing(retry::transient(|| fs::remove_file(&oldest)))?;
    for n in (1..KEEP_FILES).rev() {
        let (from, to) = (dir.join(log_name(n - 1)), dir.join(log_name(n)));
        ignore_missing(retry::transient(|| fs::rename(&from, &to)))?;
    }
    Ok(())
}

/// 建目录、轮换，再装 log 门面（Info 级）和 panic 钩子。
/// 只能装一次，再调用返回 AlreadyExists；启动时轮换失败不算失败：接着写旧文件并记一条警告，
/// 再写满一个上限后重试
pub fn init(dir: &Path) -> io::Result<()> {
    let mut slot = LOGGER.lock();
    if slot.is_some() {
        return Err(io::Error::new(io::ErrorKind::AlreadyExists, "日志已初始化"));
    }
    // 存绝对路径：之后就地轮换时工作目录可能已经变了
    let dir = std::path::absolute(dir)?;
    fs::create_dir_all(&dir)?;
    let rotated = rotate(&dir);
    let mut writer = LogWriter::open(dir, MAX_BYTES)?;
    if let Err(e) = rotated {
        writer.defer_rotation(&e);
    }
    *slot = Some(writer);
    drop(slot);
    install_panic_hook();
    log::set_logger(&LOGGER)
        .map_err(|_| io::Error::new(io::ErrorKind::AlreadyExists, "已装了别的 logger"))?;
    log::set_max_level(LevelFilter::Info);
    Ok(())
}

struct FileLogger {
    writer: Mutex<Option<LogWriter>>,
}

impl FileLogger {
    /// 写日志的线程 panic 过也照常取用：最坏丢半行
    fn lock(&self) -> MutexGuard<'_, Option<LogWriter>> {
        self.writer.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// 整行在锁外拼好，锁内的代码不会 panic，所以 panic 钩子里加锁不会自锁
    fn write(&self, line: &str) {
        let mut slot = self.lock();
        if let Some(writer) = slot.as_mut() {
            writer.write_line(line);
        }
    }
}

impl Log for FileLogger {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record<'_>) {
        if self.enabled(record.metadata()) {
            self.write(&format_line(record.level(), *record.args()));
        }
    }

    /// File 不带缓冲，每行已直接交给系统
    fn flush(&self) {}
}

/// 往 dir\launcher.log 追加整行；一行写进去会超过 max_bytes 时先就地轮换
struct LogWriter {
    dir: PathBuf,
    max_bytes: u64,
    /// 打开失败时为 None，下一行再试
    file: Option<File>,
    /// 当前文件的字节数
    len: u64,
    /// 轮换失败后的重试点：文件涨到这个大小再试，免得每行都试、每次都把旧日志往后挤
    retry_at: Option<u64>,
}

impl LogWriter {
    fn open(dir: PathBuf, max_bytes: u64) -> io::Result<LogWriter> {
        let file = open_append(&dir.join(LOG_FILE))?;
        let len = file.metadata()?.len();
        Ok(LogWriter {
            dir,
            max_bytes,
            file: Some(file),
            len,
            retry_at: None,
        })
    }

    fn write_line(&mut self, line: &str) {
        let limit = self.retry_at.unwrap_or(self.max_bytes);
        // 空文件不轮换：单行超过上限时整行写进新文件
        if self.len > 0 && self.len.saturating_add(line.len() as u64) > limit {
            self.rotate_in_place();
        }
        self.append(line);
    }

    /// 不看上限直接追加；写失败只能丢掉这一行：日志自己的错误没有地方可记
    fn append(&mut self, line: &str) {
        if self.file.is_none() {
            self.reopen();
        }
        if let Some(file) = self.file.as_mut()
            && file.write_all(line.as_bytes()).is_ok()
        {
            self.len += line.len() as u64;
        }
    }

    /// 先关掉当前文件（不然句柄会跟着改名后的 launcher.1.log 写下去），轮换后新开 launcher.log
    fn rotate_in_place(&mut self) {
        self.file = None;
        let rotated = rotate(&self.dir);
        self.reopen();
        match rotated {
            Ok(()) => self.retry_at = None,
            Err(e) => self.defer_rotation(&e),
        }
    }

    /// 轮换失败：接着写当前文件并记一条警告，再写满一个上限后重试
    fn defer_rotation(&mut self, e: &io::Error) {
        self.append(&format_line(
            Level::Warn,
            format_args!("日志轮换失败，先接着写 {LOG_FILE}，再写满一个上限后重试：{e}"),
        ));
        self.retry_at = Some(self.len.saturating_add(self.max_bytes));
    }

    fn reopen(&mut self) {
        self.file = open_append(&self.dir.join(LOG_FILE)).ok();
        self.len = self
            .file
            .as_ref()
            .and_then(|file| file.metadata().ok())
            .map_or(0, |meta| meta.len());
    }
}

/// 一条记录一行：本地时间、级别、消息；消息里的换行写成字面 \n
fn format_line(level: Level, message: fmt::Arguments<'_>) -> String {
    let mut text = String::new();
    // Display 实现报错时保留已写出的部分，不 panic
    let _ = text.write_fmt(message);
    let mut line = String::with_capacity(text.len() + 32);
    let _ = write!(line, "{} {level:<5} ", Local::now().format(TIME_FORMAT));
    line.push_str(&single_line(&text));
    line.push_str("\r\n");
    line
}

/// \r\n、\n、\r 都算一个换行
fn single_line(text: &str) -> Cow<'_, str> {
    if text.contains(['\r', '\n']) {
        Cow::Owned(text.replace("\r\n", "\n").replace(['\r', '\n'], "\\n"))
    } else {
        Cow::Borrowed(text)
    }
}

/// 先把 panic 写进日志，再调原来的钩子（默认往 stderr 打印）
fn install_panic_hook() {
    // take_hook / set_hook 在正在 panic 的线程里调用会 panic
    if std::thread::panicking() {
        return;
    }
    let previous = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        LOGGER.write(&format_line(
            Level::Error,
            format_args!("{}", panic_text(info)),
        ));
        previous(info);
        if !PANIC_REPORTED.swap(true, Ordering::SeqCst)
            && let Some(callback) = PANIC_CALLBACK.get()
        {
            callback();
        }
    }));
}

/// panic 后要做的事（弹对话框问要不要整理问题记录）；进程内最多调用一次
pub fn set_panic_callback(callback: Box<dyn Fn() + Send + Sync>) {
    if PANIC_CALLBACK.set(callback).is_err() {
        log::warn!("panic 回调已经设置过");
    }
}

/// 信息、位置和堆栈；堆栈的换行同样写成字面 \n，整条仍占一行
fn panic_text(info: &PanicHookInfo<'_>) -> String {
    let message = info.payload_as_str().unwrap_or("（非文本 panic）");
    let location = info
        .location()
        .map_or_else(|| "未知位置".to_owned(), ToString::to_string);
    format!(
        "panic：{message}（{location}）；堆栈：\n{}",
        Backtrace::force_capture()
    )
}

/// 第 n 份的文件名：0 是 launcher.log，其余是 launcher.n.log
fn log_name(n: usize) -> String {
    if n == 0 {
        LOG_FILE.to_owned()
    } else {
        format!("launcher.{n}.log")
    }
}

fn open_append(path: &Path) -> io::Result<File> {
    OpenOptions::new().create(true).append(true).open(path)
}

/// 独占打开试一下：别的进程（例如又双击了一次启动器）开着 launcher.log 时不轮换，免得挪走它正在写的文件
fn ensure_not_in_use(path: &Path) -> io::Result<()> {
    OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(path)
        .map(drop)
}

/// 文件不存在不算错
fn ignore_missing(result: io::Result<()>) -> io::Result<()> {
    match result {
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use std::fs::{self, OpenOptions};
    use std::path::Path;

    use chrono::{Local, NaiveDateTime};
    use log::Level;
    use tempfile::TempDir;

    use super::{LogWriter, format_line};

    fn read(dir: &Path, name: &str) -> String {
        fs::read_to_string(dir.join(name)).unwrap()
    }

    /// 上限是参数：用 10 字节测就地轮换，不碰全局 logger
    #[test]
    fn writer_rotates_before_exceeding_cap() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();
        let mut writer = LogWriter::open(dir.to_path_buf(), 10).unwrap();
        writer.write_line("1234567\n"); // 8
        writer.write_line("ab\n"); // 8 + 3 > 10：先轮换
        writer.write_line("cd\n"); // 3 + 3 <= 10：同一个文件
        writer.write_line("0123456789abc\n"); // 单行超过上限：独占一个文件，整行写入
        writer.write_line("x\n");
        drop(writer);
        assert_eq!(read(dir, "launcher.log"), "x\n");
        assert_eq!(read(dir, "launcher.1.log"), "0123456789abc\n");
        assert_eq!(read(dir, "launcher.2.log"), "ab\ncd\n");
        assert_eq!(read(dir, "launcher.3.log"), "1234567\n");
    }

    #[test]
    fn writer_keeps_at_most_five_files() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();
        let mut writer = LogWriter::open(dir.to_path_buf(), 10).unwrap();
        for i in 0..8 {
            writer.write_line(&format!("line {i}\n")); // 每行 7 字节，两行就超
        }
        drop(writer);
        assert_eq!(read(dir, "launcher.log"), "line 7\n");
        assert_eq!(read(dir, "launcher.4.log"), "line 3\n");
        assert!(!dir.join("launcher.5.log").exists());
        assert_eq!(fs::read_dir(dir).unwrap().count(), 5);
    }

    #[test]
    fn writer_retries_rotation_after_another_cap() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();
        let mut writer = LogWriter::open(dir.to_path_buf(), 10).unwrap();
        // 别的进程（杀毒、日志查看器、另一个启动器）开着 launcher.log
        let other = OpenOptions::new()
            .append(true)
            .open(dir.join("launcher.log"))
            .unwrap();
        writer.write_line("1234567\n");
        writer.write_line("ab\n"); // 该轮换了，但文件被占用：接着写，记一条警告
        writer.write_line("cd\n"); // 失败后要再写满 10 字节才重试，不会每行都试
        drop(other);
        writer.write_line("ef\n"); // 还没写满，不试
        writer.write_line("gh\n"); // 写满了：重试成功
        writer.write_line("ij\n"); // 成功后回到按上限轮换
        drop(writer);
        let old = read(dir, "launcher.1.log");
        assert!(old.starts_with("1234567\n"), "{old}");
        assert!(old.ends_with("ab\ncd\nef\n"), "{old}");
        assert_eq!(old.matches(" WARN  日志轮换失败").count(), 1, "{old}");
        assert_eq!(read(dir, "launcher.log"), "gh\nij\n");
    }

    #[test]
    fn line_has_local_time_level_and_no_inner_newline() {
        let line = format_line(Level::Warn, format_args!("a\r\nb\nc\rd"));
        assert!(line.ends_with(" WARN  a\\nb\\nc\\nd\r\n"), "{line}");
        assert_eq!(line.matches('\n').count(), 1, "{line}");
        let time = NaiveDateTime::parse_from_str(&line[..23], "%Y-%m-%d %H:%M:%S%.3f").unwrap();
        // 本地时间，不是 UTC
        let drift = Local::now().naive_local() - time;
        assert!(drift.num_seconds().abs() < 60, "{line}");
    }
}
