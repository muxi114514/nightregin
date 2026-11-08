//! 存档备份：文件名、去重、先写临时文件再改名、存档占用与列表，都在临时目录的真实文件上验证；
//! 轮换和清理残留 .tmp 见 backup_rotation.rs

mod common;

use std::fs::{self, OpenOptions};
use std::io;
use std::os::windows::fs::OpenOptionsExt;

use chrono::TimeDelta;
use common::backup::{
    ACCOUNT, backup_dir, created, file_name, later, location, lock_exclusive, names, now, time,
};
use launcher_core::backup::BackupKind::{Auto, Manual, PreRestore};
use launcher_core::backup::{
    BACKUP_DIR_NAME, BackupEntry, BackupError, BackupOutcome, BackupStore,
};
use launcher_core::saves::SaveLocation;
use tempfile::TempDir;
use windows_sys::Win32::Storage::FileSystem::FILE_SHARE_READ;

#[test]
fn auto_backup_creates_named_file() {
    let tmp = TempDir::new().unwrap();
    let save = location(&tmp, &ACCOUNT.to_string());
    fs::write(&save.file, b"A").unwrap();
    let store = BackupStore::new(&save);

    let entry = created(store.create(Auto, now()).unwrap());
    let path = backup_dir(&save).join("NR0000_2026-11-06_214005_auto.co2.bak");
    let want = BackupEntry {
        path: path.clone(),
        kind: Auto,
        taken_at: now(),
        size: 1,
    };
    assert_eq!(entry, want);
    assert_eq!(fs::read(&path).unwrap(), b"A");
    // 不留 .tmp；存档原样不动
    assert_eq!(names(&save), ["NR0000_2026-11-06_214005_auto.co2.bak"]);
    assert_eq!(fs::read(&save.file).unwrap(), b"A");
    // list 从文件名读回同一份
    assert_eq!(store.list().unwrap(), [want]);
}

#[test]
fn auto_backup_skips_when_unchanged() {
    let tmp = TempDir::new().unwrap();
    let save = location(&tmp, &ACCOUNT.to_string());
    fs::write(&save.file, b"A").unwrap();
    let store = BackupStore::new(&save);

    let first = created(store.create(Auto, now()).unwrap());
    let again = store.create(Auto, later(1)).unwrap();
    assert_eq!(again, BackupOutcome::Unchanged(first.clone()));
    assert_eq!(names(&save), [file_name(&first.path)]);

    // 大小相同、内容不同：按哈希判断，要新建
    fs::write(&save.file, b"B").unwrap();
    let second = created(store.create(Auto, later(2)).unwrap());
    assert_eq!(
        file_name(&second.path),
        "NR0000_2026-11-06_214205_auto.co2.bak"
    );
    assert_eq!(fs::read(&second.path).unwrap(), b"B");

    // 最新一份读不了：无从比较，宁可再存一份，不报错
    let lock = lock_exclusive(&second.path);
    let third = created(store.create(Auto, later(3)).unwrap());
    drop(lock);
    assert_eq!(fs::read(&third.path).unwrap(), b"B");
    assert_eq!(store.list().unwrap(), [third, second, first]);
}

#[test]
fn auto_dedupes_only_against_latest_auto() {
    let tmp = TempDir::new().unwrap();
    let save = location(&tmp, &ACCOUNT.to_string());
    let store = BackupStore::new(&save);

    fs::write(&save.file, b"A").unwrap();
    let first = created(store.create(Auto, now()).unwrap());
    fs::write(&save.file, b"B").unwrap();
    let manual = created(store.create(Manual, later(1)).unwrap());
    // 最新一份是内容相同的手动备份：不算，自动备份照样新建
    let second = created(store.create(Auto, later(2)).unwrap());
    assert_eq!(fs::read(&second.path).unwrap(), b"B");
    // 内容没变：沿用最新的自动备份
    let outcome = store.create(Auto, later(3)).unwrap();
    assert_eq!(outcome, BackupOutcome::Unchanged(second.clone()));

    // 只比最新的自动备份：存档改回 A，更早的自动备份虽然也是 A，仍要新建
    fs::write(&save.file, b"A").unwrap();
    let newest = created(store.create(Auto, later(4)).unwrap());
    assert_eq!(fs::read(&newest.path).unwrap(), b"A");
    assert_eq!(store.list().unwrap(), [newest, second, manual, first]);
}

#[test]
fn manual_dedupes_against_latest_manual() {
    let tmp = TempDir::new().unwrap();
    let save = location(&tmp, &ACCOUNT.to_string());
    fs::write(&save.file, b"A").unwrap();
    let store = BackupStore::new(&save);

    let auto = created(store.create(Auto, now()).unwrap());
    // 只和手动备份比：内容和最新的自动备份相同也新建
    let first = created(store.create(Manual, later(1)).unwrap());
    assert_eq!(
        file_name(&first.path),
        "NR0000_2026-11-06_214105_manual.co2.bak"
    );
    // 和最新一份手动备份相同：不新建，免得连按几下把更早的手动备份挤掉
    assert_eq!(
        store.create(Manual, later(2)).unwrap(),
        BackupOutcome::Unchanged(first.clone())
    );
    fs::write(&save.file, b"B").unwrap();
    let second = created(store.create(Manual, later(3)).unwrap());
    assert_eq!(
        file_name(&second.path),
        "NR0000_2026-11-06_214305_manual.co2.bak"
    );
    assert_eq!(fs::read(&first.path).unwrap(), b"A");
    assert_eq!(store.list().unwrap(), [second, first, auto]);
}

#[test]
fn prerestore_reuses_matching_prerestore() {
    let tmp = TempDir::new().unwrap();
    let save = location(&tmp, &ACCOUNT.to_string());
    let store = BackupStore::new(&save);

    fs::write(&save.file, b"A").unwrap();
    let a = created(store.create(PreRestore, now()).unwrap());
    assert_eq!(
        file_name(&a.path),
        "NR0000_2026-11-06_214005_prerestore.co2.bak"
    );
    // 和最新的恢复前备份相同：沿用它
    let outcome = store.create(PreRestore, later(1)).unwrap();
    assert_eq!(outcome, BackupOutcome::Unchanged(a.clone()));
    // 内容相同的手动备份成了最新一份：不影响，仍沿用最新的恢复前备份
    let manual = created(store.create(Manual, later(2)).unwrap());
    let outcome = store.create(PreRestore, later(3)).unwrap();
    assert_eq!(outcome, BackupOutcome::Unchanged(a.clone()));

    fs::write(&save.file, b"B").unwrap();
    let b = created(store.create(PreRestore, later(4)).unwrap());
    // 更早的恢复前备份是 A：沿用它（规格：和已有的某一份相同就不新建）
    fs::write(&save.file, b"A").unwrap();
    let outcome = store.create(PreRestore, later(5)).unwrap();
    assert_eq!(outcome, BackupOutcome::Unchanged(a.clone()));
    assert_eq!(store.list().unwrap(), [b, manual, a]);
}

#[test]
fn same_second_collision_gets_suffix() {
    let tmp = TempDir::new().unwrap();
    let save = location(&tmp, &ACCOUNT.to_string());
    fs::write(&save.file, b"A").unwrap();
    let store = BackupStore::new(&save);

    let first = created(store.create(Manual, now()).unwrap());
    fs::write(&save.file, b"B").unwrap();
    let second = created(store.create(Manual, now()).unwrap());
    fs::write(&save.file, b"C").unwrap();
    // 同一秒里稍晚的时刻：名字和 taken_at 都只到秒
    let third = created(
        store
            .create(Manual, now() + TimeDelta::milliseconds(600))
            .unwrap(),
    );
    assert_eq!(
        file_name(&first.path),
        "NR0000_2026-11-06_214005_manual.co2.bak"
    );
    assert_eq!(
        file_name(&second.path),
        "NR0000_2026-11-06_214005-1_manual.co2.bak"
    );
    assert_eq!(
        file_name(&third.path),
        "NR0000_2026-11-06_214005-2_manual.co2.bak"
    );
    assert_eq!(third.taken_at, now());
    // 同一秒按序号排，后建的在前
    assert_eq!(store.list().unwrap(), [third, second, first]);
}

#[test]
fn no_save_returns_nosave() {
    let tmp = TempDir::new().unwrap();
    // 新玩家：连账号目录都还没有
    let dir = tmp.path().join(ACCOUNT.to_string());
    let save = SaveLocation {
        account: ACCOUNT,
        file: dir.join("NR0000.co2"),
        dir: dir.clone(),
    };
    let store = BackupStore::new(&save);
    assert_eq!(store.create(Auto, now()).unwrap(), BackupOutcome::NoSave);
    assert!(!dir.exists());
    assert!(store.list().unwrap().is_empty());

    // 账号目录有了、存档还没有：三种都跳过，也不建备份文件夹
    fs::create_dir_all(&dir).unwrap();
    for kind in [Auto, Manual, PreRestore] {
        assert_eq!(store.create(kind, now()).unwrap(), BackupOutcome::NoSave);
    }
    assert!(!backup_dir(&save).exists());

    // 存档被删了：已有的备份不动
    fs::write(&save.file, b"A").unwrap();
    let entry = created(store.create(Auto, now()).unwrap());
    fs::remove_file(&save.file).unwrap();
    assert_eq!(store.create(Auto, later(1)).unwrap(), BackupOutcome::NoSave);
    assert_eq!(store.list().unwrap(), [entry]);
}

#[test]
fn busy_save_reports_savebusy() {
    let tmp = TempDir::new().unwrap();
    let save = location(&tmp, &ACCOUNT.to_string());
    fs::write(&save.file, b"A").unwrap();
    let store = BackupStore::new(&save);

    {
        // 别的程序独占打开着存档
        let _exclusive = lock_exclusive(&save.file);
        let err = store.create(Auto, now()).unwrap_err();
        assert!(matches!(err, BackupError::SaveBusy), "{err:?}");
        assert_eq!(err.to_string(), "存档正被其他程序占用");
    }
    {
        // 正在写存档（允许别人读）：读到的可能是半截，同样算占用
        let _writer = OpenOptions::new()
            .write(true)
            .share_mode(FILE_SHARE_READ)
            .open(&save.file)
            .unwrap();
        let err = store.create(Manual, now()).unwrap_err();
        assert!(matches!(err, BackupError::SaveBusy), "{err:?}");
    }
    // 什么都没写下
    assert!(!backup_dir(&save).exists());

    // 只读的占用（如杀毒扫描）不算
    let _reader = OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .open(&save.file)
        .unwrap();
    let entry = created(store.create(Auto, now()).unwrap());
    assert_eq!(fs::read(&entry.path).unwrap(), b"A");
}

#[test]
fn list_newest_first_ignores_foreign_files() {
    let tmp = TempDir::new().unwrap();
    let save = location(&tmp, &ACCOUNT.to_string());
    let store = BackupStore::new(&save);
    // 还没有备份文件夹
    assert!(store.list().unwrap().is_empty());

    fs::write(&save.file, b"A").unwrap();
    let auto = created(store.create(Auto, now()).unwrap());
    fs::write(&save.file, b"BB").unwrap();
    let prerestore = created(store.create(PreRestore, later(2)).unwrap());
    // 后建、但名字里的时间更早：按名字里的时间排
    let manual = created(store.create(Manual, later(1)).unwrap());

    let dir = backup_dir(&save);
    // 手放进来的合格备份：taken_at 和类型都从名字读
    let handmade = dir.join("NR0000_2026-11-05_080000_manual.co2.bak");
    fs::write(&handmade, b"old").unwrap();
    let foreign = [
        "notes.txt",
        "NR0000_2026-11-01_000000_auto.co2.bak.tmp",
        "NR0001_2026-11-01_000000_auto.co2.bak",
        "NR0000_2026-11-01_000000_auto.sl2.bak",
        "NR0000_2026-11-01_000000_auto.bak",
        "NR0000_2026-11-01_000000_backup.co2.bak",
        "NR0000_2026-11-01_000000_Auto.co2.bak",
        "NR0000_2026-13-01_000000_auto.co2.bak",
        "NR0000_2026-11-1_000000_auto.co2.bak",
        "NR0000_2026-11-01_00000_auto.co2.bak",
        "NR0000_2026-11-01 000000_auto.co2.bak",
        "NR0000_2026-11-01_000000-0_auto.co2.bak",
        "NR0000_2026-11-01_000000-01_auto.co2.bak",
        "NR0000_2026-11-01_000000-_auto.co2.bak",
        "NR0000__2026-11-01_000000_auto.co2.bak",
    ];
    for name in foreign {
        fs::write(dir.join(name), b"foreign").unwrap();
    }
    fs::create_dir(dir.join("NR0000_2026-11-02_000000_auto.co2.bak")).unwrap();

    let old = BackupEntry {
        path: handmade,
        kind: Manual,
        taken_at: time(2026, 11, 5, 8, 0, 0),
        size: 3,
    };
    assert_eq!(prerestore.size, 2);
    assert_eq!(store.list().unwrap(), [prerestore, manual, auto, old]);
}

#[test]
fn backup_in_cjk_path() {
    let tmp = TempDir::new().unwrap();
    let save = location(&tmp, "测试 账号");
    fs::write(&save.file, "存档内容").unwrap();
    let store = BackupStore::new(&save);

    let entry = created(store.create(Auto, now()).unwrap());
    let want = save
        .dir
        .join("黄金时代备份")
        .join("NR0000_2026-11-06_214005_auto.co2.bak");
    assert_eq!(entry.path, want);
    assert_eq!(fs::read_to_string(&want).unwrap(), "存档内容");
    assert_eq!(store.list().unwrap(), std::slice::from_ref(&entry));
    assert_eq!(
        store.create(Auto, later(1)).unwrap(),
        BackupOutcome::Unchanged(entry)
    );
}

#[test]
fn backup_name_uses_save_file_name_only() {
    let tmp = TempDir::new().unwrap();
    // 账号目录名带点、存档没有扩展名
    let dir = tmp.path().join("v1.2.co2");
    fs::create_dir_all(dir.join("sub")).unwrap();
    let plain = SaveLocation {
        account: ACCOUNT,
        file: dir.join("NR0000"),
        dir: dir.clone(),
    };
    fs::write(&plain.file, b"A").unwrap();
    let plain_store = BackupStore::new(&plain);
    let entry = created(plain_store.create(Auto, now()).unwrap());
    let backups = dir.join(BACKUP_DIR_NAME);
    assert_eq!(
        entry.path,
        backups.join("NR0000_2026-11-06_214005_auto.bak")
    );

    // savefile 带子目录：备份仍进账号目录下的备份文件夹，名字只取文件名
    let nested = SaveLocation {
        account: ACCOUNT,
        file: dir.join("sub").join("NR0001.co2"),
        dir: dir.clone(),
    };
    fs::write(&nested.file, b"B").unwrap();
    let nested_store = BackupStore::new(&nested);
    let other = created(nested_store.create(Manual, now()).unwrap());
    assert_eq!(
        other.path,
        backups.join("NR0001_2026-11-06_214005_manual.co2.bak")
    );
    assert!(!dir.join("sub").join(BACKUP_DIR_NAME).exists());

    // 年份超出四位：名字读不回来，不建
    let err = plain_store
        .create(Manual, time(10000, 1, 1, 0, 0, 0))
        .unwrap_err();
    assert!(
        matches!(&err, BackupError::Io(e) if e.kind() == io::ErrorKind::InvalidInput),
        "{err:?}"
    );

    // 同一文件夹里两个存档的备份互不相认
    assert_eq!(plain_store.list().unwrap(), [entry]);
    assert_eq!(nested_store.list().unwrap(), [other]);
}
