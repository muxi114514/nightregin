//! nighter.json：真实整合包文件、BOM 与 CRLF、错误是值

mod common;

use std::fs;
use std::path::Path;

use common::fixture;
use launcher_core::nighter::{NighterConfig, NighterDoc, read_nighter};
use launcher_core::profile::ConfigError;
use tempfile::TempDir;

/// 整合包自带 nighter.json 的内容
fn pack_config() -> NighterConfig {
    NighterConfig {
        force_unlock: true,
        force_enable: true,
        level: 5,
        super_lords: vec![0, 1, 2, 3, 4, 5, 6],
    }
}

/// 在 dir 里写一份 nighter.json 再读取
fn read_bytes(dir: &Path, bytes: &[u8]) -> Result<NighterConfig, ConfigError> {
    let path = dir.join("nighter.json");
    fs::write(&path, bytes).unwrap();
    read_nighter(&path)
}

#[test]
fn nighter_reads_pack_file() {
    // 真实文件是 CRLF、无末尾换行，还带我们不读的 bypassOnlineCheck
    let config = read_nighter(&fixture("pack/DepthChange/nighter.json")).unwrap();
    assert_eq!(config, pack_config());
}

#[test]
fn nighter_tolerates_bom_and_crlf() {
    let tmp = TempDir::new().unwrap();
    // 记事本另存时可能加上 BOM
    let mut bytes = b"\xEF\xBB\xBF".to_vec();
    bytes.extend(fs::read(fixture("pack/DepthChange/nighter.json")).unwrap());
    assert_eq!(read_bytes(tmp.path(), &bytes).unwrap(), pack_config());

    let crlf = "{\r\n\t\"superNightLordList\": [],\r\n\t\"forceDeepNight\": {\"level\": 3, \"enable\": false},\r\n\t\"forceUnlockDeepNight\": false\r\n}\r\n";
    assert_eq!(
        read_bytes(tmp.path(), crlf.as_bytes()).unwrap(),
        NighterConfig {
            force_unlock: false,
            force_enable: false,
            level: 3,
            super_lords: vec![]
        }
    );
}

#[test]
fn nighter_errors_are_values() {
    let tmp = TempDir::new().unwrap();
    let err = read_nighter(&tmp.path().join("none.json")).unwrap_err();
    assert!(matches!(err, ConfigError::Io(_)));

    // 记事本存成 UTF-16 的文件：读到了，是内容不对
    let utf16: Vec<u8> = [0xFF, 0xFE]
        .into_iter()
        .chain("{}".encode_utf16().flat_map(u16::to_le_bytes))
        .collect();
    let ok = r#""forceUnlockDeepNight": true, "forceDeepNight": {"enable": true, "level": 5}"#;
    let cases: [(Vec<u8>, &str); 8] = [
        (utf16, "格式错误：文件不是 UTF-8 编码"),
        (b"{\n  \"forceUnlockDeepNight\": true,\n}".to_vec(), "格式错误：第 3 行："),
        (b"[]".to_vec(), "格式错误：缺少 forceUnlockDeepNight"),
        (format!(r#"{{{ok}}}"#).into_bytes(), "格式错误：缺少 superNightLordList"),
        (
            br#"{"forceUnlockDeepNight": true, "forceDeepNight": {"enable": true}, "superNightLordList": []}"#.to_vec(),
            "格式错误：缺少 forceDeepNight.level",
        ),
        (
            format!(r#"{{{ok}, "superNightLordList": [1, "2"]}}"#).into_bytes(),
            "格式错误：superNightLordList 应为 0~255 的整数数组",
        ),
        (
            br#"{"forceUnlockDeepNight": 1, "forceDeepNight": {"enable": true, "level": 5}, "superNightLordList": []}"#.to_vec(),
            "格式错误：forceUnlockDeepNight 应为 true 或 false",
        ),
        (
            br#"{"forceUnlockDeepNight": true, "forceDeepNight": {"enable": true, "level": 256}, "superNightLordList": []}"#.to_vec(),
            "格式错误：forceDeepNight.level 应为 0~255 的整数",
        ),
    ];
    for (bytes, want) in cases {
        let err = read_bytes(tmp.path(), &bytes).unwrap_err();
        assert!(matches!(err, ConfigError::Parse(_)), "{want}");
        assert!(err.to_string().starts_with(want), "{err} / {want}");
    }
}

/// 压成一行的语法错误不带 serde 的位置后缀
#[test]
fn nighter_syntax_error_has_no_position() {
    let tmp = TempDir::new().unwrap();
    let err = read_bytes(tmp.path(), b"{\n  \"a\": 1,\n}").unwrap_err();
    let text = err.to_string();
    assert!(text.contains("第 3 行"), "{text}");
    assert!(!text.contains("at line"), "{text}");
}

fn fixture_bytes() -> Vec<u8> {
    fs::read(fixture("pack/DepthChange/nighter.json")).unwrap()
}

#[test]
fn nighter_doc_reads_same_config() {
    assert_eq!(
        NighterDoc::parse(&fixture_bytes())
            .unwrap()
            .config()
            .unwrap(),
        pack_config()
    );
}

#[test]
fn nighter_untouched_roundtrip_identical() {
    let original = fixture_bytes();
    assert_eq!(NighterDoc::parse(&original).unwrap().to_bytes(), original);
}

#[test]
fn nighter_set_level_changes_one_line() {
    let original = fixture_bytes();
    let mut doc = NighterDoc::parse(&original).unwrap();
    doc.set_level(3).unwrap();
    let written = doc.to_bytes();
    let split = |b: &[u8]| {
        b.split(|&c| c == b'\n')
            .map(<[u8]>::to_vec)
            .collect::<Vec<_>>()
    };
    let (old, new) = (split(&original), split(&written));
    assert_eq!(old.len(), new.len());
    let changed: Vec<usize> = (0..old.len()).filter(|&i| old[i] != new[i]).collect();
    assert_eq!(changed.len(), 1);
    assert_eq!(new[changed[0]].trim_ascii(), b"\"level\": 3");
}

#[test]
fn nighter_keeps_order_unknown_fields() {
    let text = "{\n  \"z\": 1,\n  \"forceDeepNight\": {\n    \"level\": 2,\n    \"x\": true,\n    \"enable\": true\n  },\n  \"bypassOnlineCheck\": true,\n  \"forceUnlockDeepNight\": false,\n  \"superNightLordList\": []\n}";
    let mut doc = NighterDoc::parse(text.as_bytes()).unwrap();
    doc.set_force_unlock(true).unwrap();
    doc.set_force_enable(false).unwrap();
    let want = text
        .replace(
            "\"forceUnlockDeepNight\": false",
            "\"forceUnlockDeepNight\": true",
        )
        .replace("\"enable\": true", "\"enable\": false");
    assert_eq!(String::from_utf8(doc.to_bytes()).unwrap(), want);
}

#[test]
fn nighter_keeps_lf_bom_and_four_space_indent() {
    let text = b"\xEF\xBB\xBF{\n    \"forceUnlockDeepNight\": true,\n    \"forceDeepNight\": {\n        \"enable\": true,\n        \"level\": 5\n    },\n    \"superNightLordList\": [\n        1\n    ]\n}\n";
    let doc = NighterDoc::parse(text).unwrap();
    assert_eq!(doc.to_bytes(), text);
}

#[test]
fn nighter_empty_list_writes_empty_array() {
    let mut doc = NighterDoc::parse(&fixture_bytes()).unwrap();
    doc.set_super_lords(&[]).unwrap();
    assert_eq!(doc.config().unwrap().super_lords, Vec::<u8>::new());
    let text = String::from_utf8(doc.to_bytes()).unwrap();
    assert!(text.contains("\"superNightLordList\": []"), "{text}");
}

#[test]
fn nighter_set_on_missing_key_errors() {
    let mut doc = NighterDoc::parse(b"{}").unwrap();
    assert!(matches!(doc.set_level(3), Err(ConfigError::Parse(_))));
    assert!(matches!(
        doc.set_force_enable(true),
        Err(ConfigError::Parse(_))
    ));
    assert!(matches!(
        doc.set_force_unlock(true),
        Err(ConfigError::Parse(_))
    ));
    assert!(matches!(
        doc.set_super_lords(&[1]),
        Err(ConfigError::Parse(_))
    ));
}
