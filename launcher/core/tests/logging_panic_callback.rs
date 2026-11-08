//! panic 回调只跑一次（进程级全局状态，单独一个测试文件）

use std::sync::atomic::{AtomicU32, Ordering};
use std::thread;

use launcher_core::logging;
use tempfile::TempDir;

static CALLS: AtomicU32 = AtomicU32::new(0);

#[test]
fn panic_callback_runs_once() {
    let tmp = TempDir::new().unwrap();
    logging::init(&tmp.path().join("logs")).unwrap();
    logging::set_panic_callback(Box::new(|| {
        CALLS.fetch_add(1, Ordering::SeqCst);
    }));
    for _ in 0..3 {
        assert!(thread::spawn(|| panic!("测试 panic")).join().is_err());
    }
    assert_eq!(CALLS.load(Ordering::SeqCst), 1);
}
