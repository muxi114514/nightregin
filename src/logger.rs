//! 文件日志：DLL 同目录 logs/nr_stat_alloc.log，每次启动覆盖（上一次的改名为 .prev.log），避免无限增长。

use std::fs::{File, OpenOptions, create_dir_all};
use std::io::Write;
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use log::{LevelFilter, Log, Metadata, Record};

struct FileLogger {
    file: Mutex<File>,
    level: LevelFilter,
    start: Instant,
}

impl Log for FileLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let t = self.start.elapsed().as_secs_f64();
        // 锁中毒时直接放弃本条日志，不能因为日志让游戏崩溃
        if let Ok(mut f) = self.file.lock() {
            let _ = writeln!(f, "[{t:>9.3}] [{:<5}] {}", record.level(), record.args());
            let _ = f.flush();
        }
    }

    fn flush(&self) {
        if let Ok(mut f) = self.file.lock() {
            let _ = f.flush();
        }
    }
}

static LOGGER: OnceLock<FileLogger> = OnceLock::new();

pub fn init(dir: &Path, level: LevelFilter) -> std::io::Result<()> {
    let logs = dir.join("logs");
    create_dir_all(&logs)?;
    let path = logs.join("nr_stat_alloc.log");
    // 保留上一次会话的日志，便于对照测试
    let _ = std::fs::rename(&path, logs.join("nr_stat_alloc.prev.log"));
    let file = OpenOptions::new().create(true).write(true).truncate(true).open(&path)?;
    let logger = LOGGER.get_or_init(|| FileLogger { file: Mutex::new(file), level, start: Instant::now() });
    if log::set_logger(logger).is_ok() {
        log::set_max_level(level);
    }
    Ok(())
}
