//! 日志轮换。init 装的是进程级全局状态（log 门面、panic 钩子），一个进程只能装一次，这里不调用

use std::fs::{self, OpenOptions};
use std::path::Path;

use launcher_core::logging::{KEEP_FILES, LOG_FILE, MAX_BYTES, rotate};
use tempfile::TempDir;

fn read(dir: &Path, name: &str) -> String {
    fs::read_to_string(dir.join(name)).unwrap()
}

#[test]
fn rotate_keeps_five() {
    assert_eq!(
        (LOG_FILE, KEEP_FILES, MAX_BYTES),
        ("launcher.log", 5, 5 * 1024 * 1024)
    );
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    for name in [
        "launcher.log",
        "launcher.1.log",
        "launcher.2.log",
        "launcher.3.log",
        "launcher.4.log",
    ] {
        fs::write(dir.join(name), name).unwrap();
    }

    rotate(dir).unwrap();
    assert!(!dir.join("launcher.log").exists());
    assert_eq!(read(dir, "launcher.1.log"), "launcher.log");
    assert_eq!(read(dir, "launcher.2.log"), "launcher.1.log");
    assert_eq!(read(dir, "launcher.3.log"), "launcher.2.log");
    assert_eq!(read(dir, "launcher.4.log"), "launcher.3.log");
    assert!(!dir.join("launcher.5.log").exists());
    // 加上之后新建的 launcher.log 正好 5 份
    assert_eq!(fs::read_dir(dir).unwrap().count(), KEEP_FILES - 1);
}

#[test]
fn rotate_tolerates_missing_files() {
    let tmp = TempDir::new().unwrap();
    // 第一次运行：日志目录还不存在，也不替它建
    let logs = tmp.path().join("logs");
    rotate(&logs).unwrap();
    assert!(!logs.exists());

    // 只有部分文件，中间有空档
    let dir = tmp.path();
    fs::write(dir.join("launcher.log"), "new").unwrap();
    fs::write(dir.join("launcher.2.log"), "old").unwrap();
    rotate(dir).unwrap();
    assert_eq!(read(dir, "launcher.1.log"), "new");
    assert_eq!(read(dir, "launcher.3.log"), "old");
    assert!(!dir.join("launcher.log").exists());
    assert!(!dir.join("launcher.2.log").exists());
}

#[test]
fn rotate_skips_log_in_use() {
    // 启动器已在运行时又双击了一次：新进程不能把正在写的 launcher.log 挪走
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    fs::write(dir.join("launcher.log"), "live").unwrap();
    fs::write(dir.join("launcher.4.log"), "oldest").unwrap();
    let running = OpenOptions::new()
        .append(true)
        .open(dir.join("launcher.log"))
        .unwrap();

    assert!(rotate(dir).is_err());
    assert_eq!(read(dir, "launcher.log"), "live");
    assert_eq!(read(dir, "launcher.4.log"), "oldest");
    assert!(!dir.join("launcher.1.log").exists());

    // 那个进程退出后照常轮换
    drop(running);
    rotate(dir).unwrap();
    assert_eq!(read(dir, "launcher.1.log"), "live");
    assert!(!dir.join("launcher.4.log").exists());
}
