//! init 与 panic 钩子。装的是进程级全局状态，单独成一个测试文件（独立进程），只放这一个测试

use std::fs;
use std::io;
use std::thread;

use chrono::NaiveDateTime;
use launcher_core::logging::{self, LOG_FILE};
use tempfile::TempDir;

/// 行首 23 字节是本地时间，返回其后的部分
fn after_time(line: &str) -> &str {
    let (time, rest) = line.split_at(23);
    assert!(
        NaiveDateTime::parse_from_str(time, "%Y-%m-%d %H:%M:%S%.3f").is_ok(),
        "{line}"
    );
    rest
}

#[test]
fn init_installs_logger_and_panic_hook() {
    let tmp = TempDir::new().unwrap();
    // 日志目录还不存在，路径带空格和中文
    let dir = tmp.path().join("黑夜 君临").join("logs");
    logging::init(&dir).unwrap();
    log::info!("启动器启动");
    log::debug!("调试信息不写");

    // 第二次：报错，不 panic
    assert_eq!(
        logging::init(&dir).unwrap_err().kind(),
        io::ErrorKind::AlreadyExists
    );

    // 子线程 panic：写一条 ERROR，带信息和 文件:行 位置
    let (worker, line) = (thread::spawn(|| panic!("测试 panic 42")), line!());
    assert!(worker.join().is_err());

    let text = fs::read_to_string(dir.join(LOG_FILE)).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert!(!text.contains("调试信息不写"), "{text}");
    let info = lines.iter().find(|l| l.ends_with("启动器启动")).unwrap();
    assert_eq!(after_time(info), " INFO  启动器启动");
    // 只有一条：第二次 init 没有再装一个钩子
    let panic_lines: Vec<_> = lines
        .iter()
        .filter(|l| l.contains("测试 panic 42"))
        .collect();
    assert_eq!(panic_lines.len(), 1, "{text}");
    let panic_line = panic_lines[0];
    assert!(
        after_time(panic_line).starts_with(" ERROR panic："),
        "{panic_line}"
    );
    assert!(
        panic_line.contains(&format!("logging_init.rs:{line}:")),
        "{panic_line}"
    );
}
