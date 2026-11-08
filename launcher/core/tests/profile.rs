//! 读 .me3：真实整合包配置、enabled / optional、path 与 source 别名、两种表数组写法、错误

mod common;

use std::fs;
use std::path::Path;

use common::fixture;
use launcher_core::profile::{ConfigError, Me3Entry, Me3Profile};
use tempfile::TempDir;

/// 在 dir 里写一份 test.me3 再读取
fn load_text(dir: &Path, text: &str) -> Result<Me3Profile, ConfigError> {
    let path = dir.join("test.me3");
    fs::write(&path, text).unwrap();
    Me3Profile::load(&path)
}

#[test]
fn profile_reads_real_pack_profile() {
    let p = Me3Profile::load(&fixture("pack/褪色燃烧.me3")).unwrap();
    assert_eq!(p.savefile.as_deref(), Some("NR0000.co2"));
    assert!(
        p.natives
            .iter()
            .any(|e| e.path.ends_with("SeamlessCoop/nrsc.dll"))
    );
    assert!(
        p.natives
            .iter()
            .any(|e| e.path.ends_with("StatAlloc/nr_stat_alloc.dll"))
    );
    assert!(
        p.packages
            .iter()
            .any(|e| e.path.ends_with("StatAlloc/pkg_mmv"))
    );
    assert!(p.packages.iter().any(|e| e.path.ends_with("mod")));
    // 单引号路径照常读；注释掉的 RevenantFix 不算
    assert!(
        p.natives
            .iter()
            .any(|e| e.path.ends_with("dll/custom_drop_fxrs.dll"))
    );
    assert!(
        !p.natives
            .iter()
            .any(|e| e.path.ends_with("RevenantFix/RevenantFix.dll"))
    );
    assert_eq!((p.natives.len(), p.packages.len()), (9, 3));
    // 路径都相对 .me3 所在目录，转成了绝对路径
    assert_eq!(p.dir, fixture("pack"));
    assert!(
        p.natives
            .iter()
            .chain(&p.packages)
            .all(|e| e.path.starts_with(&p.dir))
    );
    assert!(p.natives.iter().chain(&p.packages).all(|e| !e.optional));
}

#[test]
fn profile_skips_disabled_and_reports_missing() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    fs::create_dir_all(dir.join("dll")).unwrap();
    fs::write(dir.join(r"dll\present.dll"), b"").unwrap();
    fs::create_dir_all(dir.join("pkg_present")).unwrap();
    let p = load_text(
        dir,
        r#"
[[natives]]
path = "dll/present.dll"

[[natives]]
path = "dll/required.dll"
load_after = [{ id = "present.dll", optional = true }]

[[natives]]
path = "dll/optional.dll"
optional = true

[[natives]]
path = "dll/disabled.dll"
enabled = false

[[packages]]
id = "present"
path = "pkg_present"

[[packages]]
id = "gone"
path = "pkg_gone"

[[packages]]
id = "off"
path = "pkg_off"
enabled = false
"#,
    )
    .unwrap();

    // enabled = false 的不收；load_after 里的 optional 不是条目自己的
    let names: Vec<&str> = p
        .natives
        .iter()
        .chain(&p.packages)
        .map(|e| e.path.file_name().unwrap().to_str().unwrap())
        .collect();
    assert_eq!(
        names,
        [
            "present.dll",
            "required.dll",
            "optional.dll",
            "pkg_present",
            "pkg_gone"
        ]
    );
    let optional: Vec<bool> = p.natives.iter().map(|e| e.optional).collect();
    assert_eq!(optional, [false, false, true]);
    // 缺失的必需 native 和 package 按配置顺序报出；optional 的缺了不报
    assert_eq!(
        p.missing_files(),
        vec![dir.join(r"dll\required.dll"), dir.join("pkg_gone")]
    );
}

#[test]
fn profile_accepts_path_or_source_alias() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    let p = load_text(
        dir,
        r#"
[[packages]]
id = "a"
path = "pkg_a"

[[packages]]
id = "b"
source = './sub/pkg_b'
"#,
    )
    .unwrap();
    assert_eq!(
        p.packages,
        vec![
            Me3Entry {
                path: dir.join("pkg_a"),
                optional: false
            },
            Me3Entry {
                path: dir.join(r"sub\pkg_b"),
                optional: false
            },
        ]
    );
}

#[test]
fn profile_reads_inline_table_arrays() {
    // natives = [{ … }] 与 [[natives]] 在 TOML 里等价，me3 两种都认
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    let p = load_text(
        dir,
        r#"
natives = [{ path = "a.dll", optional = true }, { path = "b.dll", enabled = false }]
packages = [{ id = "p", source = "pkg" }]
"#,
    )
    .unwrap();
    assert_eq!(
        p.natives,
        vec![Me3Entry {
            path: dir.join("a.dll"),
            optional: true
        }]
    );
    assert_eq!(
        p.packages,
        vec![Me3Entry {
            path: dir.join("pkg"),
            optional: false
        }]
    );
}

#[test]
fn profile_errors_are_values() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    let missing = Me3Profile::load(&dir.join("none.me3"));
    assert!(matches!(missing, Err(ConfigError::Io(_))));
    assert!(missing.unwrap_err().to_string().starts_with("读取失败："));

    // 语法错误、条目缺路径（native 不认 source）、类型不对：都是格式错误
    for text in [
        "natives = [",
        "[[natives]]\nsource = \"a.dll\"",
        "[[natives]]\npath = \"a.dll\"\nenabled = \"no\"",
        "natives = \"a.dll\"",
        "natives = [\"a.dll\"]",
        "savefile = 1",
    ] {
        let err = load_text(dir, text).unwrap_err();
        assert!(matches!(err, ConfigError::Parse(_)), "{text}");
        assert!(err.to_string().starts_with("格式错误："), "{text}");
    }
    // 报错带位置：语法错误给行号，条目错误给第几项
    let err = load_text(dir, "savefile = \"a\"\n\nnatives = [").unwrap_err();
    assert!(err.to_string().starts_with("格式错误：第 3 行："), "{err}");
    let err = load_text(
        dir,
        "[[natives]]\npath = \"a.dll\"\n\n[[natives]]\nsource = \"b.dll\"",
    )
    .unwrap_err();
    assert_eq!(err.to_string(), "格式错误：natives 第 2 项：缺少 path");
}

#[test]
fn me3_non_utf8_reports_parse() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("gbk.me3");
    fs::write(&path, b"savefile = \"\xC4\xE3.co2\"\n").unwrap();
    let err = Me3Profile::load(&path).unwrap_err();
    assert!(matches!(err, ConfigError::Parse(_)));
    assert!(err.to_string().contains("文件不是 UTF-8 编码"), "{err}");
}

#[test]
fn profile_tolerates_bom() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("bom.me3");
    fs::write(&path, b"\xEF\xBB\xBFsavefile = \"NR0000.co2\"\n").unwrap();
    assert_eq!(
        Me3Profile::load(&path).unwrap().savefile.as_deref(),
        Some("NR0000.co2")
    );
}

#[test]
fn profile_entry_paths_are_absolute() {
    let tmp = TempDir::new().unwrap();
    let text = "[[natives]]\npath = 'dll\\a.dll'\n[[packages]]\npath = 'mod'\n";
    let profile = load_text(tmp.path(), text).unwrap();
    assert!(profile.natives[0].path.is_absolute());
    assert_eq!(profile.packages[0].path, tmp.path().join("mod"));
}
