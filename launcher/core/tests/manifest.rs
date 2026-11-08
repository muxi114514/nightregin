//! 整合包 manifest.json：只读 version / game / me3，其余键忽略；缺失或损坏为 None

use std::fs;

use launcher_core::gamever::GameVersion;
use launcher_core::manifest::{ManifestInfo, read_manifest_info};
use tempfile::TempDir;

#[test]
fn manifest_info_reads_fields_and_ignores_rest() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("manifest.json");
    let text = r#"{"version":"1.9.5","game":[{"fileVersion":"1.3.3.0","size":118835320}],"me3":"0.10.1","files":[]}"#;
    let want = ManifestInfo {
        version: Some("1.9.5".to_owned()),
        game: vec![GameVersion {
            file_version: [1, 3, 3, 0],
            size: 118835320,
        }],
        me3: Some("0.10.1".to_owned()),
        notes: Vec::new(),
    };
    fs::write(&path, text).unwrap();
    assert_eq!(read_manifest_info(&path), Some(want.clone()));

    // PowerShell 5.1 写出的 UTF-8 带 BOM
    let mut bom = b"\xEF\xBB\xBF".to_vec();
    bom.extend_from_slice(text.as_bytes());
    fs::write(&path, bom).unwrap();
    assert_eq!(read_manifest_info(&path), Some(want));

    // 三项都可以没有
    fs::write(&path, r#"{"files":[]}"#).unwrap();
    assert_eq!(read_manifest_info(&path), Some(ManifestInfo::default()));
}

#[test]
fn manifest_info_none_when_missing_or_broken() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("manifest.json");
    assert_eq!(read_manifest_info(&path), None);

    let broken = [
        "",
        "null",
        r#"{"version":"1.9.5",}"#,
        r#"{"version":195}"#,
        r#"{"game":{"fileVersion":"1.3.3.0","size":118835320}}"#,
        r#"{"game":[{"fileVersion":"1.3.3","size":118835320}]}"#,
        r#"{"game":[{"fileVersion":"1.3.3.0"}]}"#,
    ];
    for text in broken {
        fs::write(&path, text).unwrap();
        assert_eq!(read_manifest_info(&path), None, "{text}");
    }
}
