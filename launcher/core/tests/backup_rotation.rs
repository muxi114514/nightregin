//! 存档备份的清理：按类型轮换、不删刚建的和被占用的、清掉崩溃留下的 .tmp；
//! 只删本存档名字合格的文件，别的一概不动

mod common;

use std::fs;

use common::backup::{
    ACCOUNT, backup_dir, created, file_name, later, location, lock_exclusive, names, now, of_kind,
};
use launcher_core::backup::BackupKind::{Auto, Manual, PreRestore};
use launcher_core::backup::{BackupEntry, BackupOutcome, BackupStore};
use tempfile::TempDir;

#[test]
fn rotation_keeps_ten_three_three() {
    let tmp = TempDir::new().unwrap();
    let save = location(&tmp, &ACCOUNT.to_string());
    let store = BackupStore::new(&save);
    let dir = backup_dir(&save);
    fs::create_dir_all(&dir).unwrap();
    // 都比真备份旧：轮换要是把它们当成备份，会先删它们
    let foreign = [
        "notes.txt",
        "NR0001_2020-01-01_000000_auto.co2.bak.tmp",
        "NR0001_2020-01-01_000000_auto.co2.bak",
        "NR0000_2020-01-01_000000_auto.sl2.bak",
        "NR0000_2020-01-01_000000-01_auto.co2.bak",
        "NR0000_2020-01-01_000000_Manual.co2.bak",
    ];
    for name in foreign {
        fs::write(dir.join(name), b"foreign").unwrap();
    }
    // 名字合格的目录
    let folder = dir.join("NR0000_2020-01-01_000001_auto.co2.bak");
    fs::create_dir(&folder).unwrap();

    let mut autos = Vec::new();
    for i in 0..12 {
        fs::write(&save.file, format!("auto {i}")).unwrap();
        autos.push(created(store.create(Auto, later(i)).unwrap()));
    }
    // 只剩最新的 10 份，最早两份删掉
    let kept_autos: Vec<BackupEntry> = autos[2..].iter().rev().cloned().collect();
    assert_eq!(of_kind(&store, Auto), kept_autos);
    assert!(!autos[0].path.exists() && !autos[1].path.exists());
    assert_eq!(fs::read(&kept_autos[0].path).unwrap(), b"auto 11");

    // 手动备份比自动备份都新：只轮换同类型，自动备份一份不少
    let mut manuals = Vec::new();
    for i in 0..5 {
        fs::write(&save.file, format!("manual {i}")).unwrap();
        manuals.push(created(store.create(Manual, later(100 + i)).unwrap()));
    }
    let kept_manuals: Vec<BackupEntry> = manuals[2..].iter().rev().cloned().collect();
    assert_eq!(of_kind(&store, Manual), kept_manuals);

    let mut prerestores = Vec::new();
    for i in 0..5 {
        fs::write(&save.file, format!("pre {i}")).unwrap();
        prerestores.push(created(store.create(PreRestore, later(200 + i)).unwrap()));
    }
    let kept_prerestores: Vec<BackupEntry> = prerestores[2..].iter().rev().cloned().collect();
    assert_eq!(of_kind(&store, PreRestore), kept_prerestores);
    assert_eq!(of_kind(&store, Manual), kept_manuals);
    assert_eq!(of_kind(&store, Auto), kept_autos);

    // 不归这个存档的文件和目录原样留着
    for name in foreign {
        assert_eq!(fs::read(dir.join(name)).unwrap(), b"foreign", "{name}");
    }
    assert!(folder.is_dir());
    assert_eq!(names(&save).len(), 10 + 3 + 3 + foreign.len() + 1);
}

#[test]
fn rotation_spares_new_and_locked_backups() {
    let tmp = TempDir::new().unwrap();
    let save = location(&tmp, &ACCOUNT.to_string());
    let store = BackupStore::new(&save);
    let mut autos = Vec::new();
    for i in 0..10 {
        fs::write(&save.file, format!("auto {i}")).unwrap();
        autos.push(created(store.create(Auto, later(10 + i)).unwrap()));
    }
    // 时钟往回调过：新备份的名字最旧，留下它，删原来最旧的
    fs::write(&save.file, b"rewound").unwrap();
    let rewound = created(store.create(Auto, now()).unwrap());
    assert_eq!(fs::read(&rewound.path).unwrap(), b"rewound");
    assert!(!autos[0].path.exists());
    assert_eq!(of_kind(&store, Auto).len(), 10);

    // 该删的那份被占用：照样新建，它留到下次再删
    let lock = lock_exclusive(&rewound.path);
    fs::write(&save.file, b"next").unwrap();
    created(store.create(Auto, later(30)).unwrap());
    drop(lock);
    assert_eq!(of_kind(&store, Auto).len(), 11);
    fs::write(&save.file, b"last").unwrap();
    created(store.create(Auto, later(31)).unwrap());
    assert_eq!(of_kind(&store, Auto).len(), 10);
    assert!(!rewound.path.exists() && !autos[1].path.exists());
}

#[test]
fn create_removes_stale_tmp_files() {
    let tmp = TempDir::new().unwrap();
    let save = location(&tmp, &ACCOUNT.to_string());
    fs::write(&save.file, b"A").unwrap();
    let store = BackupStore::new(&save);
    let first = created(store.create(Auto, now()).unwrap());
    let dir = backup_dir(&save);
    // 崩溃或断电留下的本存档 .tmp
    let stale = dir.join("NR0000_2026-11-06_214000_auto.co2.bak.tmp");
    let locked = dir.join("NR0000_2026-11-05_080000_manual.co2.bak.tmp");
    fs::write(&stale, b"partial").unwrap();
    fs::write(&locked, b"partial").unwrap();
    // 别的 .tmp、别的存档的 .tmp、去掉 .tmp 后仍不合格的、名字合格的目录：一律不动
    let foreign = [
        "notes.tmp",
        "NR0001_2026-11-06_214000_auto.co2.bak.tmp",
        "NR0000_2026-11-06_214000_auto.co2.bak.tmp.tmp",
    ];
    for name in foreign {
        fs::write(dir.join(name), b"foreign").unwrap();
    }
    let folder = dir.join("NR0000_2026-11-04_080000_auto.co2.bak.tmp");
    fs::create_dir(&folder).unwrap();

    // 内容没变、不新建也照样清理；删不掉的跳过，不影响这次备份
    let lock = lock_exclusive(&locked);
    let outcome = store.create(Auto, later(1)).unwrap();
    drop(lock);
    assert_eq!(outcome, BackupOutcome::Unchanged(first.clone()));
    assert!(!stale.exists());
    assert!(locked.exists());
    // 放开后下一次 create 清掉
    store.create(Auto, later(2)).unwrap();
    assert!(!locked.exists());

    for name in foreign {
        assert_eq!(fs::read(dir.join(name)).unwrap(), b"foreign", "{name}");
    }
    assert!(folder.is_dir());
    let mut want = vec![
        file_name(&first.path).to_owned(),
        file_name(&folder).to_owned(),
    ];
    want.extend(foreign.map(String::from));
    want.sort();
    assert_eq!(names(&save), want);
}

/// 一秒内 keep+3 次同类型备份：序号一直往上走，轮换留下的是最新的
#[test]
fn same_second_sequence_is_monotonic() {
    let tmp = TempDir::new().unwrap();
    let save = location(&tmp, "account");
    fs::write(&save.file, b"x").unwrap();
    let store = BackupStore::new(&save);
    for i in 0..6 {
        fs::write(&save.file, format!("{i}")).unwrap();
        created(store.create(Manual, now()).unwrap());
    }
    let mut left = names(&save);
    left.sort();
    assert_eq!(
        left,
        [
            "NR0000_2026-11-06_214005-3_manual.co2.bak",
            "NR0000_2026-11-06_214005-4_manual.co2.bak",
            "NR0000_2026-11-06_214005-5_manual.co2.bak",
        ]
    );
}
