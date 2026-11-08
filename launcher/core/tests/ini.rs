//! INI：逐字节往返、节名键名不区分大小写、全局节、值里的注释与空白

mod common;

use std::fs;

use common::fixture;
use launcher_core::ini::{IniDoc, read_ini};
use launcher_core::profile::ConfigError;

fn chance_setter() -> IniDoc {
    read_ini(&fixture("pack/dll/ChanceSetter.ini")).unwrap()
}

#[test]
fn ini_roundtrip_is_byte_identical() {
    // ChanceSetter 是 LF，两份 nrsc_settings 是 CRLF
    for rel in [
        "pack/dll/ChanceSetter.ini",
        "pack/SeamlessCoop/nrsc_settings.ini",
        "game_nrsc_settings.ini",
    ] {
        let bytes = fs::read(fixture(rel)).unwrap();
        assert_eq!(IniDoc::parse(&bytes).to_bytes(), bytes, "{rel}");
    }
}

#[test]
fn ini_roundtrip_keeps_bom_and_line_endings() {
    // BOM、末尾有无换行、CRLF 与 LF 混用、孤立的 CR、空文件都原样保留
    for bytes in [
        &b"\xEF\xBB\xBF[a]\r\nk = v\r\n"[..],
        b"\xEF\xBB\xBFk = v",
        b"[a]\nk = v",
        b"a = 1\r\nb = 2\nc = 3\r\n",
        b"a = 1\r\nb = 2\r",
        b"a = 1\rb = 2\n",
        b"",
        b"\xEF\xBB\xBF",
        b"\n",
        b"\r\n\r\n",
    ] {
        let shown = String::from_utf8_lossy(bytes);
        assert_eq!(IniDoc::parse(bytes).to_bytes(), bytes, "{shown:?}");
    }
    // BOM 不算进第一个键；CRLF 和混用换行时值里不带 \r
    assert_eq!(
        IniDoc::parse(b"\xEF\xBB\xBFk = v").get("", "k").as_deref(),
        Some("v")
    );
    let mixed = IniDoc::parse(b"a = 1\r\nb = 2\nc = 3\r\n");
    assert_eq!(mixed.get("", "a").as_deref(), Some("1"));
    assert_eq!(mixed.get("", "b").as_deref(), Some("2"));
    assert_eq!(mixed.get("", "c").as_deref(), Some("3"));
}

#[test]
fn ini_get_case_insensitive() {
    let cs = chance_setter();
    let nrsc = read_ini(&fixture("pack/SeamlessCoop/nrsc_settings.ini")).unwrap();
    assert_eq!(cs.get("save", "autoextension").as_deref(), Some(".CL_SAVE"));
    assert_eq!(
        nrsc.get("scaling", "HEALTH_SCALING").as_deref(),
        Some("120")
    );
    // 文件里 [save] 小写、[SAVE] 大写，反过来查也行；值本身的大小写不动
    assert_eq!(cs.get("SAVE", "Path").as_deref(), Some("auto"));
    assert_eq!(
        nrsc.get("save", "save_file_extension").as_deref(),
        Some("co2")
    );
    // 游戏目录那份是另一个文件
    let game = read_ini(&fixture("game_nrsc_settings.ini")).unwrap();
    assert_eq!(
        game.get("SCALING", "health_scaling").as_deref(),
        Some("100")
    );
}

#[test]
fn ini_global_section() {
    let cs = chance_setter();
    assert_eq!(cs.get("", "configVersion").as_deref(), Some("10"));
    // "" 只含第一个节之前的键，节里的键查不到，反之亦然
    assert_eq!(cs.get("", "enabled"), None);
    assert_eq!(cs.get("general", "configVersion"), None);
    assert_eq!(IniDoc::parse(b"[s]\nk = 1\n").get("", "k"), None);
}

#[test]
fn ini_dotted_section_and_aligned_keys() {
    let cs = chance_setter();
    assert_eq!(
        cs.get("everdarkChances.depth5", "Harmonia").as_deref(),
        Some("66")
    );
    assert_eq!(
        cs.get("nightlordChances.depth5", "Gladius").as_deref(),
        Some("10")
    );
    // 带点的节名各自独立，不并进前缀同名的节
    assert_eq!(
        cs.get("everdarkChances.depth1", "Harmonia").as_deref(),
        Some("25")
    );
    assert_eq!(cs.get("everdarkChances", "Harmonia"), None);
    assert_eq!(
        cs.get("everdarkChances", "enabled").as_deref(),
        Some("true")
    );
}

#[test]
fn ini_value_comments_and_whitespace() {
    let doc = IniDoc::parse(
        b"[s]\n\
          a = 1 ; note\n\
          b = C:\\x;y\n\
          c =\tv\t# note\n\
          d =   \n\
          e = ; only a comment\n\
          f = x = y\n\
          ;g = 1\n\
          # h = 2\n\
          [ t ]\n\
          k = 1\n",
    );
    let get = |key: &str| doc.get("s", key);
    // 空白之后的 ; 或 # 才是注释
    assert_eq!(get("a").as_deref(), Some("1"));
    assert_eq!(get("b").as_deref(), Some(r"C:\x;y"));
    assert_eq!(get("c").as_deref(), Some("v"));
    assert_eq!(get("d").as_deref(), Some(""));
    assert_eq!(get("e").as_deref(), Some(""));
    // 值从第一个 = 之后开始
    assert_eq!(get("f").as_deref(), Some("x = y"));
    // 注释行即使含 = 也不是键
    assert_eq!(get(";g"), None);
    assert_eq!(get("# h"), None);
    // 节名两端空白不算
    assert_eq!(doc.get("t", "k").as_deref(), Some("1"));
}

#[test]
fn ini_duplicates_take_first() {
    // 重复的键或节取第一个（ChanceSetter 用的 GetPrivateProfileString 也是这样）
    let doc = IniDoc::parse(b"[a]\nk = 1\nk = 2\n[b]\nk = 3\n[A]\nk = 4\nonly = 5\n");
    assert_eq!(doc.get("a", "k").as_deref(), Some("1"));
    assert_eq!(doc.get("b", "k").as_deref(), Some("3"));
    // 只看第一个同名节（和 GetPrivateProfileString 一致）
    assert_eq!(doc.get("a", "only"), None);
}

#[test]
fn ini_non_utf8_value_is_lossy() {
    // 用 GBK 保存的中文路径：读得出来，非 UTF-8 部分换成替换符，扩展名不受影响
    let doc = IniDoc::parse(b"[save]\r\npath = D:\\\xC4\xE3\\NR0000.co2\r\n");
    assert_eq!(
        doc.get("save", "path").as_deref(),
        Some("D:\\\u{FFFD}\u{FFFD}\\NR0000.co2")
    );
}

#[test]
fn ini_read_missing_file_is_io_error() {
    let err = read_ini(&fixture("no_such.ini")).unwrap_err();
    assert!(matches!(err, ConfigError::Io(_)));
    assert!(err.to_string().starts_with("读取失败："));
}

fn lines(bytes: &[u8]) -> Vec<&[u8]> {
    bytes.split(|&b| b == b'\n').collect()
}

#[test]
fn set_changes_only_target_line() {
    let original = fs::read(fixture("pack/SeamlessCoop/nrsc_settings.ini")).unwrap();
    let mut doc = IniDoc::parse(&original);
    assert!(doc.set("SCALING", "health_scaling", "200").unwrap());
    let written = doc.to_bytes();
    let (old, new) = (lines(&original), lines(&written));
    assert_eq!(old.len(), new.len());
    let changed: Vec<usize> = (0..old.len()).filter(|&i| old[i] != new[i]).collect();
    assert_eq!(changed.len(), 1);
    assert_eq!(new[changed[0]].trim_ascii(), b"health_scaling = 200");
}

#[test]
fn set_keeps_alignment_crlf_bom() {
    let mut doc = IniDoc::parse(b"\xEF\xBB\xBF[a]\r\nGladius  = 25\r\nAdel     = 25\r\n");
    assert!(doc.set("a", "gladius", "100").unwrap());
    assert_eq!(
        doc.to_bytes(),
        b"\xEF\xBB\xBF[a]\r\nGladius  = 100\r\nAdel     = 25\r\n"
    );
}

#[test]
fn set_keeps_inline_comment() {
    let mut doc = IniDoc::parse(b"[s]\na = 1   ; note\n");
    assert!(doc.set("s", "a", "2").unwrap());
    assert_eq!(doc.to_bytes(), b"[s]\na = 2   ; note\n");
}

#[test]
fn set_preserves_non_utf8_elsewhere() {
    let original = b"; \xC4\xE3\xBA\xC3\r\n[s]\r\na = 1\r\n";
    let mut doc = IniDoc::parse(original);
    assert!(doc.set("s", "a", "2").unwrap());
    assert_eq!(doc.to_bytes(), b"; \xC4\xE3\xBA\xC3\r\n[s]\r\na = 2\r\n");
}

#[test]
fn set_on_empty_value() {
    for (before, after) in [
        (&b"[s]\na =\n"[..], &b"[s]\na = 2\n"[..]),
        (b"[s]\na =   \n", b"[s]\na = 2\n"),
        (b"[s]\na = ; note\n", b"[s]\na = 2 ; note\n"),
    ] {
        let mut doc = IniDoc::parse(before);
        assert!(doc.set("s", "a", "2").unwrap());
        assert_eq!(doc.to_bytes(), after);
    }
}

#[test]
fn set_same_value_returns_false() {
    let original = b"[s]\na = 1\n";
    let mut doc = IniDoc::parse(original);
    assert!(!doc.set("s", "a", "1").unwrap());
    assert_eq!(doc.to_bytes(), original);
}

#[test]
fn set_missing_key_errors_and_changes_nothing() {
    let original = b"[s]\na = 1\n";
    let mut doc = IniDoc::parse(original);
    let err = doc.set("s", "b", "2").unwrap_err();
    assert_eq!((err.section.as_str(), err.key.as_str()), ("s", "b"));
    assert!(doc.set("t", "a", "2").is_err());
    assert_eq!(doc.to_bytes(), original);
}

#[test]
fn first_duplicate_section_wins() {
    let mut doc = IniDoc::parse(b"[a]\nk = 1\n[b]\nk = 3\n[A]\nk = 4\nonly = 5\n");
    assert_eq!(doc.get("a", "k").as_deref(), Some("1"));
    assert_eq!(doc.get("a", "only"), None);
    assert!(doc.set("a", "only", "9").is_err());
    assert!(doc.set("A", "k", "7").unwrap());
    assert_eq!(
        doc.to_bytes(),
        b"[a]\nk = 7\n[b]\nk = 3\n[A]\nk = 4\nonly = 5\n"
    );
}
