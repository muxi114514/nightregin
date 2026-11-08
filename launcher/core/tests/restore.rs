//! 恢复存档：先留恢复前备份、两个文件换成同一份内容、中途失败两个文件都回到恢复前

mod common;

use std::fs;

use common::backup::{created, later, location, lock_exclusive, now, of_kind};
use launcher_core::backup::BackupKind::{Manual, PreRestore};
use launcher_core::backup::restore::RestoreError;
use launcher_core::backup::{BackupEntry, BackupStore};
use launcher_core::saves::SaveLocation;
use tempfile::TempDir;

/// 存档 + 游戏自己写的 .bak，内容都是 current；再做一份手动备份，内容是 old
fn setup(tmp: &TempDir) -> (SaveLocation, BackupStore, BackupEntry) {
    let save = location(tmp, "76561198000000001");
    fs::write(&save.file, b"old").unwrap();
    let store = BackupStore::new(&save);
    let backup = created(store.create(Manual, now()).unwrap());
    fs::write(&save.file, b"current").unwrap();
    fs::write(bak(&save), b"game-bak").unwrap();
    (save, store, backup)
}

fn bak(save: &SaveLocation) -> std::path::PathBuf {
    save.dir.join("NR0000.co2.bak")
}

#[test]
fn restore_replaces_save_and_bak_same_content() {
    let tmp = TempDir::new().unwrap();
    let (save, store, backup) = setup(&tmp);
    store.restore(&backup, later(1)).unwrap();
    assert_eq!(fs::read(&save.file).unwrap(), b"old");
    assert_eq!(fs::read(bak(&save)).unwrap(), b"old");
}

#[test]
fn restore_creates_prerestore_first() {
    let tmp = TempDir::new().unwrap();
    let (_save, store, backup) = setup(&tmp);
    let outcome = store.restore(&backup, later(1)).unwrap();
    let pre = outcome.prerestore.unwrap();
    assert_eq!(pre.kind, PreRestore);
    assert_eq!(fs::read(&pre.path).unwrap(), b"current");
}

#[test]
fn prerestore_reuses_same_sha() {
    let tmp = TempDir::new().unwrap();
    let (save, store, backup) = setup(&tmp);
    let first = store
        .restore(&backup, later(1))
        .unwrap()
        .prerestore
        .unwrap();
    // 回到 current 再恢复一次：当前内容和第一份恢复前备份相同
    fs::write(&save.file, b"middle").unwrap();
    store.restore(&backup, later(2)).unwrap();
    fs::write(&save.file, b"current").unwrap();
    let again = store
        .restore(&backup, later(3))
        .unwrap()
        .prerestore
        .unwrap();
    assert_eq!(again, first);
    assert_eq!(of_kind(&store, PreRestore).len(), 2);
}

#[test]
fn prerestore_keep_three() {
    let tmp = TempDir::new().unwrap();
    let (save, store, backup) = setup(&tmp);
    for i in 0..5 {
        fs::write(&save.file, format!("state-{i}")).unwrap();
        store.restore(&backup, later(i + 1)).unwrap();
    }
    assert_eq!(of_kind(&store, PreRestore).len(), 3);
}

#[test]
fn restore_empty_backup_refused() {
    let tmp = TempDir::new().unwrap();
    let (save, store, mut backup) = setup(&tmp);
    fs::write(&backup.path, b"").unwrap();
    backup.size = 0;
    assert!(matches!(
        store.restore(&backup, later(1)),
        Err(RestoreError::EmptyBackup)
    ));
    assert_eq!(fs::read(&save.file).unwrap(), b"current");
}

#[test]
fn restore_without_current_save_works() {
    let tmp = TempDir::new().unwrap();
    let (save, store, backup) = setup(&tmp);
    fs::remove_file(&save.file).unwrap();
    fs::remove_file(bak(&save)).unwrap();
    assert_eq!(store.restore(&backup, later(1)).unwrap().prerestore, None);
    assert_eq!(fs::read(&save.file).unwrap(), b"old");
    assert_eq!(fs::read(bak(&save)).unwrap(), b"old");
}

#[test]
fn restore_busy_save_reports_busy_and_changes_nothing() {
    let tmp = TempDir::new().unwrap();
    let (save, store, backup) = setup(&tmp);
    let held = lock_exclusive(&save.file);
    assert!(matches!(
        store.restore(&backup, later(1)),
        Err(RestoreError::SaveBusy)
    ));
    drop(held);
    assert_eq!(fs::read(&save.file).unwrap(), b"current");
    assert_eq!(fs::read(bak(&save)).unwrap(), b"game-bak");
}

#[test]
fn restore_bak_locked_rolls_back_both() {
    let tmp = TempDir::new().unwrap();
    let (save, store, backup) = setup(&tmp);
    let held = lock_exclusive(&bak(&save));
    let err = store.restore(&backup, later(1)).unwrap_err();
    assert!(matches!(err, RestoreError::RolledBack(_)), "{err}");
    drop(held);
    assert_eq!(fs::read(&save.file).unwrap(), b"current");
    assert_eq!(fs::read(bak(&save)).unwrap(), b"game-bak");
    let leftovers = fs::read_dir(&save.dir)
        .unwrap()
        .filter(|e| {
            e.as_ref()
                .unwrap()
                .file_name()
                .to_string_lossy()
                .ends_with("-tmp")
        })
        .count();
    assert_eq!(leftovers, 0);
}

/// 已有 3 份恢复前备份，恢复最旧的那份：新建恢复前备份的轮换不能把选中的这份删掉
#[test]
fn restoring_oldest_prerestore_keeps_it() {
    let tmp = TempDir::new().unwrap();
    let (save, store, backup) = setup(&tmp);
    for i in 0..3 {
        fs::write(&save.file, format!("state-{i}")).unwrap();
        store.restore(&backup, later(i + 1)).unwrap();
    }
    let oldest = of_kind(&store, PreRestore).pop().unwrap();
    assert_eq!(fs::read(&oldest.path).unwrap(), b"state-0");
    fs::write(&save.file, b"state-new").unwrap();

    store.restore(&oldest, later(10)).unwrap();
    assert_eq!(fs::read(&save.file).unwrap(), b"state-0");
    assert!(oldest.path.exists(), "选中的备份不能被轮换删掉");
    assert_eq!(of_kind(&store, PreRestore).len(), 3);
}
