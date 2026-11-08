//! 读 exe 的文件版本与大小；版本号的点分写法

use std::fs;
use std::io;
use std::path::Path;

use launcher_core::gamever::{GameVersion, dotted, parse_dotted, read_exe_version};
use tempfile::TempDir;

#[test]
fn exe_version_of_system_binary() {
    let exe = Path::new(r"C:\Windows\System32\kernel32.dll");
    let version = read_exe_version(exe).unwrap();
    // 测试进程没有声明支持 Win10 的清单，系统把系统文件的 10.0 报成 6.2，构建号照实给
    let [major, _, build, _] = version.file_version;
    assert!(major >= 6 && build >= 10240, "{version:?}");
    assert_eq!(version.size, fs::metadata(exe).unwrap().len());
}

#[test]
fn exe_version_errors_are_values() {
    let tmp = TempDir::new().unwrap();
    let missing = read_exe_version(&tmp.path().join("nightreign.exe")).unwrap_err();
    assert_eq!(missing.kind(), io::ErrorKind::NotFound);
    // 不是 PE 文件，没有版本资源
    let text = tmp.path().join("readme.txt");
    fs::write(&text, b"not an exe").unwrap();
    assert!(read_exe_version(&text).is_err());
}

#[test]
fn dotted_roundtrip() {
    assert_eq!(parse_dotted("1.3.3.0"), Some([1, 3, 3, 0]));
    assert_eq!(dotted([1, 3, 3, 0]), "1.3.3.0");
    assert_eq!(
        parse_dotted(&dotted([65535, 0, 10, 2])),
        Some([65535, 0, 10, 2])
    );
    let bad = [
        "",
        "1.3.3",
        "1.3.3.0.0",
        "1..3.0",
        "1.3.3.x",
        "65536.0.0.0",
        "+1.3.3.0",
        " 1.3.3.0",
    ];
    for s in bad {
        assert_eq!(parse_dotted(s), None, "{s}");
    }

    // manifest 里的写法
    let version = GameVersion {
        file_version: [1, 3, 3, 0],
        size: 118835320,
    };
    let json = r#"{"fileVersion":"1.3.3.0","size":118835320}"#;
    assert_eq!(serde_json::to_string(&version).unwrap(), json);
    assert_eq!(serde_json::from_str::<GameVersion>(json).unwrap(), version);
    assert!(serde_json::from_str::<GameVersion>(r#"{"fileVersion":"1.3.3","size":1}"#).is_err());
}
