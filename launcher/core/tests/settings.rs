//! 启动器设置：缺文件用默认值、损坏的改名 .bad、不认识的键原样保留、临时文件 + rename

use std::fs::{self, OpenOptions};
use std::os::windows::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::thread;

use launcher_core::settings::{AfterLaunch, Settings, SettingsStore};
use serde_json::json;
use tempfile::TempDir;
use windows_sys::Win32::Storage::FileSystem::FILE_SHARE_READ;

/// <整合包>\launcher\settings.json；整合包路径带空格和中文，launcher 目录不预先建
fn settings_path(tmp: &TempDir) -> PathBuf {
    tmp.path()
        .join("黑夜 君临")
        .join("launcher")
        .join("settings.json")
}

/// 建好父目录后原样写入
fn write_raw(path: &Path, bytes: &[u8]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

#[test]
fn settings_defaults_when_missing() {
    let tmp = TempDir::new().unwrap();
    let path = settings_path(&tmp);
    let (settings, note) = SettingsStore::new(path.clone()).load();
    let expected = Settings {
        schema_version: 1,
        game_dir: None,
        save_account: None,
        backup_before_start: true,
        after_launch: AfterLaunch::Tray,
        low_effects: false,
        dev_mode: false,
        last_launch_at: None,
        exit_check_pending: false,
        last_exit_abnormal: false,
        everdark_picks: None,
        first_run_done: false,
        extra: serde_json::Map::new(),
    };
    assert_eq!(settings, expected);
    assert_eq!(Settings::default(), expected);
    assert_eq!(note, None);
    // 只读，不建文件
    assert!(!path.exists());
}

#[test]
fn settings_corrupt_file_renamed_bad() {
    let tmp = TempDir::new().unwrap();
    let path = settings_path(&tmp);
    let bad = path.with_file_name("settings.json.bad");
    let store = SettingsStore::new(path.clone());

    write_raw(&path, b"{oops");
    let (settings, note) = store.load();
    assert_eq!(settings, Settings::default());
    assert_eq!(fs::read_to_string(&bad).unwrap(), "{oops");
    assert!(!path.exists());
    let note = note.unwrap();
    assert!(note.contains("settings.json.bad"), "{note}");

    // 顶层不是对象同样算损坏；新的 .bad 覆盖旧的
    let wrong_type = "[1]";
    write_raw(&path, wrong_type.as_bytes());
    let (settings, note) = store.load();
    assert_eq!(settings, Settings::default());
    assert!(note.is_some_and(|n| n.contains("settings.json.bad")));
    assert_eq!(fs::read_to_string(&bad).unwrap(), wrong_type);

    // 改名之后等同于没有文件：默认值、没有提示
    assert_eq!(store.load(), (Settings::default(), None));
}

#[test]
fn settings_tolerates_utf8_bom() {
    // PowerShell 5.1 写出的 UTF-8 文件带 BOM
    let tmp = TempDir::new().unwrap();
    let path = settings_path(&tmp);
    write_raw(&path, b"\xEF\xBB\xBF{\"devMode\":true}");
    let (settings, note) = SettingsStore::new(path.clone()).load();
    assert_eq!(note, None);
    assert_eq!(
        settings,
        Settings {
            dev_mode: true,
            ..Settings::default()
        }
    );
    assert!(path.exists());
}

#[test]
fn settings_preserves_unknown_fields() {
    let tmp = TempDir::new().unwrap();
    let path = settings_path(&tmp);
    let store = SettingsStore::new(path.clone());
    write_raw(
        &path,
        br#"{"schemaVersion":1,"futureKey":42,"futureTable":{"list":[1,"x"]}}"#,
    );

    let (mut settings, note) = store.load();
    assert_eq!(note, None);
    assert_eq!(settings.extra.get("futureKey"), Some(&json!(42)));
    settings.dev_mode = true;
    store.save(&settings).unwrap();

    let text = fs::read_to_string(&path).unwrap();
    assert!(text.contains(r#""futureKey":42"#), "{text}");
    assert!(text.contains(r#""futureTable":{"list":[1,"x"]}"#), "{text}");
    assert!(text.contains(r#""devMode":true"#), "{text}");
}

#[test]
fn settings_save_leaves_no_temp() {
    let tmp = TempDir::new().unwrap();
    let path = settings_path(&tmp);
    let tmp_file = path.with_file_name("settings.json.tmp");
    let store = SettingsStore::new(path.clone());
    let settings = Settings {
        game_dir: Some(PathBuf::from(r"D:\Steam Library\ELDEN RING 黑夜君临")),
        // 超过 2^53 的 SteamID64 要原样往返
        save_account: Some(76_561_198_012_345_679),
        after_launch: AfterLaunch::Close,
        last_launch_at: Some(1_762_436_405),
        exit_check_pending: true,
        ..Settings::default()
    };

    // launcher 目录还不存在，save 自己建
    store.save(&settings).unwrap();
    assert!(!tmp_file.exists());
    assert_eq!(store.load(), (settings.clone(), None));

    // 覆盖已有文件
    let changed = Settings {
        low_effects: true,
        ..settings
    };
    store.save(&changed).unwrap();
    assert!(!tmp_file.exists());
    assert_eq!(store.load(), (changed, None));
}

#[test]
fn settings_json_uses_camel_case() {
    let tmp = TempDir::new().unwrap();
    let path = settings_path(&tmp);
    SettingsStore::new(path.clone())
        .save(&Settings::default())
        .unwrap();

    let text = fs::read_to_string(&path).unwrap();
    assert!(text.contains(r#""backupBeforeStart""#), "{text}");
    assert!(text.contains(r#""afterLaunch":"tray""#), "{text}");
    assert!(text.contains(r#""schemaVersion":1"#), "{text}");
    // 没有 snake_case 的键
    assert!(!text.contains('_'), "{text}");
    assert_eq!(
        serde_json::to_string(&AfterLaunch::Keep).unwrap(),
        r#""keep""#
    );
    assert_eq!(
        serde_json::to_string(&AfterLaunch::Close).unwrap(),
        r#""close""#
    );
}

#[test]
fn settings_unreadable_file_kept_as_bad_before_save() {
    let tmp = TempDir::new().unwrap();
    let path = settings_path(&tmp);
    let bad = path.with_file_name("settings.json.bad");
    let original = br#"{"devMode":true,"afterLaunch":"keep"}"#;
    write_raw(&path, original);
    let store = SettingsStore::new(path.clone());

    // 别的程序独占打开着：重试后仍读不到，先用默认值，原文件不动
    let held = OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&path)
        .unwrap();
    let (settings, note) = store.load();
    drop(held);
    assert_eq!(settings, Settings::default());
    let note = note.unwrap();
    assert!(note.contains("settings.json.bad"), "{note}");
    assert!(!bad.exists());
    assert_eq!(fs::read(&path).unwrap(), original);

    // 下次保存覆盖之前，先把原文件留成 .bad
    let fresh = Settings {
        low_effects: true,
        ..Settings::default()
    };
    store.save(&fresh).unwrap();
    assert_eq!(fs::read(&bad).unwrap(), original);
    assert_eq!(store.load(), (fresh, None));
    // 只留这一次：之后的保存不再动 .bad
    store.save(&Settings::default()).unwrap();
    assert_eq!(fs::read(&bad).unwrap(), original);
}

#[test]
fn settings_corrupt_file_kept_when_rename_fails() {
    let tmp = TempDir::new().unwrap();
    let path = settings_path(&tmp);
    let bad = path.with_file_name("settings.json.bad");
    write_raw(&path, b"{oops");
    let store = SettingsStore::new(path.clone());

    // 别的程序开着它（允许读、不允许改名）：读得到，改名重试后仍失败
    let held = OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .open(&path)
        .unwrap();
    let (settings, note) = store.load();
    drop(held);
    assert_eq!(settings, Settings::default());
    let note = note.unwrap();
    assert!(note.contains("失败"), "{note}");
    assert!(!bad.exists());

    // 覆盖前先把损坏的原文留成 .bad
    store.save(&Settings::default()).unwrap();
    assert_eq!(fs::read_to_string(&bad).unwrap(), "{oops");
    assert_eq!(store.load(), (Settings::default(), None));
}

#[test]
fn settings_concurrent_saves_stay_whole() {
    // 启动线程与界面命令可能同时保存，甚至各自建了 store；同一路径共用一个 settings.json.tmp
    let tmp = TempDir::new().unwrap();
    let path = settings_path(&tmp);
    let workers: Vec<_> = (0..4i64)
        .map(|id| {
            let store = SettingsStore::new(path.clone());
            thread::spawn(move || {
                for round in 0..10 {
                    let settings = Settings {
                        last_launch_at: Some(id * 100 + round),
                        ..Settings::default()
                    };
                    store.save(&settings).unwrap();
                }
            })
        })
        .collect();
    for worker in workers {
        worker.join().unwrap();
    }
    let (settings, note) = SettingsStore::new(path.clone()).load();
    assert_eq!(note, None);
    assert!(
        settings.last_launch_at.is_some_and(|t| t % 100 == 9),
        "{settings:?}"
    );
    assert!(!path.with_file_name("settings.json.tmp").exists());
}

/// 回退旧版遇到新取值（或手改错一个值）：只有这个字段回到默认，其余设置都还在
#[test]
fn bad_enum_value_only_resets_that_field() {
    let tmp = TempDir::new().unwrap();
    let path = settings_path(&tmp);
    write_raw(
        &path,
        br#"{"afterLaunch":"bogus","backupBeforeStart":false}"#,
    );
    let (s, warning) = SettingsStore::new(path.clone()).load();
    assert_eq!(s.after_launch, AfterLaunch::Tray);
    assert!(!s.backup_before_start);
    assert_eq!(warning, None);
    assert!(path.is_file());
    assert!(!path.with_file_name("settings.json.bad").exists());
}

/// 字段类型不对时取这个字段自己的默认值（backupBeforeStart 默认开，不是 false）
#[test]
fn bad_field_uses_that_fields_own_default() {
    let tmp = TempDir::new().unwrap();
    let path = settings_path(&tmp);
    write_raw(&path, br#"{"backupBeforeStart":"x","lowEffects":true}"#);
    let (s, _) = SettingsStore::new(path).load();
    assert!(s.backup_before_start);
    assert!(s.low_effects);
}

#[test]
fn syntax_error_still_moves_to_bad() {
    let tmp = TempDir::new().unwrap();
    let path = settings_path(&tmp);
    write_raw(&path, b"{ not json");
    let (s, warning) = SettingsStore::new(path.clone()).load();
    assert_eq!(s, Settings::default());
    assert!(warning.is_some());
    assert!(path.with_file_name("settings.json.bad").is_file());
}

#[test]
fn non_object_root_moves_to_bad() {
    let tmp = TempDir::new().unwrap();
    let path = settings_path(&tmp);
    write_raw(&path, b"[1,2]");
    let (_, warning) = SettingsStore::new(path.clone()).load();
    assert!(warning.is_some());
    assert!(path.with_file_name("settings.json.bad").is_file());
}

#[test]
fn new_fields_roundtrip_and_unknown_kept() {
    let tmp = TempDir::new().unwrap();
    let path = settings_path(&tmp);
    write_raw(
        &path,
        br#"{"everdarkPicks":[1,3],"firstRunDone":true,"futureKey":{"a":1}}"#,
    );
    let store = SettingsStore::new(path.clone());
    let (s, _) = store.load();
    assert_eq!(s.everdark_picks, Some(vec![1, 3]));
    assert!(s.first_run_done);
    store.save(&s).unwrap();
    let saved: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    assert_eq!(saved["everdarkPicks"], json!([1, 3]));
    assert_eq!(saved["firstRunDone"], json!(true));
    assert_eq!(saved["futureKey"], json!({"a": 1}));
}

#[test]
fn first_run_defaults_to_not_done() {
    assert!(!Settings::default().first_run_done);
    assert_eq!(Settings::default().everdark_picks, None);
}
