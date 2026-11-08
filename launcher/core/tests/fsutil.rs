//! 原子写入与不覆盖的改名

use std::fs;
use std::path::Path;

use launcher_core::fsutil::{atomic_write, rename_no_replace};
use tempfile::TempDir;

fn names(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    v.sort();
    v
}

#[test]
fn atomic_write_replaces_content() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("黑夜 配置.ini");
    fs::write(&path, b"old").unwrap();
    atomic_write(&path, b"new").unwrap();
    assert_eq!(fs::read(&path).unwrap(), b"new");
    assert_eq!(names(tmp.path()), ["黑夜 配置.ini"]);
}

#[test]
fn atomic_write_leaves_no_tmp_on_failure() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("a.ini");
    fs::write(&path, b"old").unwrap();
    let mut perm = fs::metadata(&path).unwrap().permissions();
    perm.set_readonly(true);
    fs::set_permissions(&path, perm.clone()).unwrap();
    assert!(atomic_write(&path, b"new").is_err());
    assert_eq!(fs::read(&path).unwrap(), b"old");
    assert_eq!(names(tmp.path()), ["a.ini"]);
    #[allow(clippy::permissions_set_readonly_false)]
    perm.set_readonly(false);
    fs::set_permissions(&path, perm).unwrap();
}

#[test]
fn rename_no_replace_moves_file() {
    let tmp = TempDir::new().unwrap();
    let (from, to) = (tmp.path().join("a"), tmp.path().join("b"));
    fs::write(&from, b"x").unwrap();
    rename_no_replace(&from, &to).unwrap();
    assert_eq!(names(tmp.path()), ["b"]);
    assert_eq!(fs::read(&to).unwrap(), b"x");
}

#[test]
fn rename_no_replace_refuses_existing_target() {
    let tmp = TempDir::new().unwrap();
    let (from, to) = (tmp.path().join("a"), tmp.path().join("b"));
    fs::write(&from, b"x").unwrap();
    fs::write(&to, b"keep").unwrap();
    assert!(rename_no_replace(&from, &to).is_err());
    assert_eq!(fs::read(&to).unwrap(), b"keep");
    assert_eq!(fs::read(&from).unwrap(), b"x");
}
