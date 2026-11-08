//! ConfigStore：写前重读磁盘、首次备份、内容相同不写、出错不坏原文件、恢复默认

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::thread;

use common::{FakePack, fake_pack};
use launcher_core::config::store::ConfigStore;
use launcher_core::config::{ConfigFile, ConfigWriteError};
use launcher_core::ini::IniDoc;
use launcher_core::paths::PackLayout;

const FILE: ConfigFile = ConfigFile::Nrsc;

fn setup() -> (FakePack, ConfigStore, PathBuf) {
    let fp = fake_pack("pack");
    let pack = PackLayout {
        root: fp.pack_root.clone(),
    };
    let path = FILE.path(&pack);
    (fp, ConfigStore::new(pack), path)
}

fn set_key(
    key: &'static str,
    value: String,
) -> impl FnOnce(&[u8]) -> Result<Vec<u8>, ConfigWriteError> {
    move |bytes| {
        let mut doc = IniDoc::parse(bytes);
        doc.set("SCALING", key, &value)
            .map_err(|e| ConfigWriteError::Malformed(FILE, e.to_string()))?;
        Ok(doc.to_bytes())
    }
}

fn value(path: &Path, key: &str) -> Option<String> {
    IniDoc::parse(&fs::read(path).unwrap()).get("SCALING", key)
}

fn bak(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(".launcher-bak");
    PathBuf::from(name)
}

#[test]
fn first_patch_creates_launcher_bak_once() {
    let (_fp, store, path) = setup();
    let original = fs::read(&path).unwrap();
    store
        .patch(FILE, set_key("health_scaling", "200".into()))
        .unwrap();
    assert_eq!(fs::read(bak(&path)).unwrap(), original);
    store
        .patch(FILE, set_key("health_scaling", "210".into()))
        .unwrap();
    assert_eq!(fs::read(bak(&path)).unwrap(), original);
    assert_eq!(value(&path, "health_scaling").as_deref(), Some("210"));
}

#[test]
fn patch_rereads_disk_and_reports_external() {
    let (_fp, store, path) = setup();
    store.read(FILE).unwrap();
    // 启动器开着时，用户用记事本改了别的键
    let mut doc = IniDoc::parse(&fs::read(&path).unwrap());
    doc.set("SCALING", "damage_scaling", "77").unwrap();
    fs::write(&path, doc.to_bytes()).unwrap();

    let report = store
        .patch(FILE, set_key("health_scaling", "200".into()))
        .unwrap();
    assert!(report.external_change);
    assert!(report.wrote);
    assert_eq!(value(&path, "damage_scaling").as_deref(), Some("77"));
    assert_eq!(value(&path, "health_scaling").as_deref(), Some("200"));

    // 这次写入之后磁盘和记下的一致，不再算外部改动
    let report = store
        .patch(FILE, set_key("health_scaling", "210".into()))
        .unwrap();
    assert!(!report.external_change);
}

#[test]
fn never_read_is_not_external() {
    let (_fp, store, _path) = setup();
    let report = store
        .patch(FILE, set_key("health_scaling", "200".into()))
        .unwrap();
    assert!(!report.external_change);
}

#[test]
fn no_change_neither_writes_nor_backs_up() {
    let (_fp, store, path) = setup();
    let before = fs::metadata(&path).unwrap().modified().unwrap();
    let report = store
        .patch(FILE, set_key("health_scaling", "120".into()))
        .unwrap();
    assert!(!report.wrote);
    assert!(!bak(&path).exists());
    assert_eq!(fs::metadata(&path).unwrap().modified().unwrap(), before);
}

#[test]
fn patch_failure_keeps_original() {
    let (_fp, store, path) = setup();
    let original = fs::read(&path).unwrap();
    let mut perm = fs::metadata(&path).unwrap().permissions();
    perm.set_readonly(true);
    fs::set_permissions(&path, perm.clone()).unwrap();

    let err = store
        .patch(FILE, set_key("health_scaling", "200".into()))
        .unwrap_err();
    assert!(
        matches!(err, ConfigWriteError::Io(ConfigFile::Nrsc, _)),
        "{err}"
    );
    assert_eq!(fs::read(&path).unwrap(), original);
    let dir = path.parent().unwrap();
    let leftovers = fs::read_dir(dir)
        .unwrap()
        .filter(|e| {
            e.as_ref()
                .unwrap()
                .file_name()
                .to_string_lossy()
                .ends_with(".launcher-tmp")
        })
        .count();
    assert_eq!(leftovers, 0);

    #[allow(clippy::permissions_set_readonly_false)]
    perm.set_readonly(false);
    fs::set_permissions(&path, perm).unwrap();
}

#[test]
fn change_error_leaves_file_alone() {
    let (_fp, store, path) = setup();
    let original = fs::read(&path).unwrap();
    let err = store
        .patch(FILE, set_key("no_such_key", "1".into()))
        .unwrap_err();
    assert!(matches!(
        err,
        ConfigWriteError::Malformed(ConfigFile::Nrsc, _)
    ));
    assert_eq!(fs::read(&path).unwrap(), original);
    assert!(!bak(&path).exists());
}

#[test]
fn concurrent_patches_keep_both_keys() {
    let (_fp, store, path) = setup();
    thread::scope(|s| {
        for key in ["health_scaling", "damage_scaling"] {
            let store = &store;
            s.spawn(move || {
                for i in 0..50 {
                    store
                        .patch(FILE, set_key(key, (i * 10 + 10).to_string()))
                        .unwrap();
                }
            });
        }
    });
    assert_eq!(value(&path, "health_scaling").as_deref(), Some("500"));
    assert_eq!(value(&path, "damage_scaling").as_deref(), Some("500"));
}

#[test]
fn restore_default_replaces_file_and_keeps_bak() {
    let (fp, store, path) = setup();
    let pack = PackLayout {
        root: fp.pack_root.clone(),
    };
    let factory = FILE.default_path(&pack);
    fs::create_dir_all(factory.parent().unwrap()).unwrap();
    fs::write(&factory, b"[SCALING]\nhealth_scaling = 120\n").unwrap();
    fs::write(&path, b"garbage").unwrap();

    store.restore_default(FILE).unwrap();
    assert_eq!(fs::read(&path).unwrap(), fs::read(&factory).unwrap());
    assert_eq!(fs::read(bak(&path)).unwrap(), b"garbage");
}

#[test]
fn restore_default_recreates_missing_file() {
    let (fp, store, path) = setup();
    let pack = PackLayout {
        root: fp.pack_root.clone(),
    };
    let factory = FILE.default_path(&pack);
    fs::create_dir_all(factory.parent().unwrap()).unwrap();
    fs::write(&factory, b"[SCALING]\n").unwrap();
    fs::remove_dir_all(path.parent().unwrap()).unwrap();

    store.restore_default(FILE).unwrap();
    assert_eq!(fs::read(&path).unwrap(), b"[SCALING]\n");
    assert!(!bak(&path).exists());
}

#[test]
fn restore_default_without_factory_reports_missing() {
    let (_fp, store, _path) = setup();
    let err = store.restore_default(FILE).unwrap_err();
    assert!(matches!(
        err,
        ConfigWriteError::DefaultsMissing(ConfigFile::Nrsc)
    ));
}
