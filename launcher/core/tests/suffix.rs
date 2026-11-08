//! 存档后缀：规范化、me3 / nrsc / ChanceSetter 三方取值、阻断与警告

mod common;

use common::fixture;
use launcher_core::ini::{IniDoc, read_ini};
use launcher_core::profile::Me3Profile;
use launcher_core::suffix::{
    CHANCE_SETTER_DEFAULT_EXT, CHANCE_SETTER_DEFAULT_PATH, NRSC_DEFAULT_EXT, SuffixCheck,
    chance_setter_ext, me3_ext, normalize_ext, nrsc_ext,
};

fn check(me3: Option<&str>, nrsc: &str, chance_setter: Option<&str>) -> SuffixCheck {
    SuffixCheck {
        me3: me3.map(str::to_owned),
        nrsc: nrsc.to_owned(),
        chance_setter: chance_setter.map(str::to_owned),
    }
}

#[test]
fn suffix_rules() {
    assert_eq!(normalize_ext(".CL_SAVE"), "cl_save");
    let ok = SuffixCheck {
        me3: Some("co2".into()),
        nrsc: "co2".into(),
        chance_setter: Some(".CL_SAVE".into()),
    };
    assert!(!ok.blocking() && ok.warning());
    let bad = SuffixCheck {
        me3: Some("co2".into()),
        nrsc: "sl2".into(),
        chance_setter: None,
    };
    assert!(bad.blocking() && !bad.warning());
}

#[test]
fn suffix_normalize_and_compare() {
    assert_eq!(normalize_ext(" \t.Co2 "), "co2");
    assert_eq!(normalize_ext("co2"), "co2");
    assert_eq!(normalize_ext(""), "");
    // 只去一个开头的点
    assert_eq!(normalize_ext("..co2"), ".co2");
    // 规范化后相同就不算不一致
    let same = check(Some("CO2"), " .co2", Some(".Co2"));
    assert!(!same.blocking() && !same.warning());
    // .me3 没有 savefile：没有可比的对象，既不阻断也不警告
    let no_me3 = check(None, "sl2", Some(".dbb"));
    assert!(!no_me3.blocking() && !no_me3.warning());
    // savefile 没有扩展名（空串）和任何非空后缀都不一致
    let bare = check(Some(""), "co2", Some("co2"));
    assert!(bare.blocking() && bare.warning());
}

#[test]
fn chance_setter_uses_path_extension_when_not_auto() {
    let doc = IniDoc::parse(b"[save]\npath = D:\\x\\NR0000.co2\nautoExtension = .CL_SAVE\n");
    assert_eq!(chance_setter_ext(&doc).as_deref(), Some("co2"));
    // 路径里有 GBK 中文也不影响扩展名；没有扩展名时为空串
    let gbk = IniDoc::parse(b"[save]\npath = D:\\\xC4\xE3\\NR0000.CO2\n");
    assert_eq!(chance_setter_ext(&gbk).as_deref(), Some("CO2"));
    let bare = IniDoc::parse(b"[save]\npath = D:\\x\\NR0000\n");
    assert_eq!(chance_setter_ext(&bare).as_deref(), Some(""));
}

#[test]
fn chance_setter_auto_uses_auto_extension() {
    // 真实整合包：path = auto，autoExtension = .CL_SAVE
    let cs = read_ini(&fixture("pack/dll/ChanceSetter.ini")).unwrap();
    assert_eq!(chance_setter_ext(&cs).as_deref(), Some(".CL_SAVE"));
    // ChanceSetter 比较 auto 时去空白、不分大小写
    let upper = IniDoc::parse(b"[SAVE]\nPATH = Auto\nautoextension = dbb\n");
    assert_eq!(chance_setter_ext(&upper).as_deref(), Some("dbb"));
}

#[test]
fn chance_setter_missing_keys_use_dll_defaults() {
    assert_eq!(
        (CHANCE_SETTER_DEFAULT_PATH, CHANCE_SETTER_DEFAULT_EXT),
        ("auto", ".dbb")
    );
    // auto 下缺 autoExtension → .dbb
    let auto_only = IniDoc::parse(b"[save]\npath = auto\n");
    assert_eq!(chance_setter_ext(&auto_only).as_deref(), Some(".dbb"));
    // 缺 path 算 auto
    let ext_only = IniDoc::parse(b"[save]\nautoExtension = .co2\n");
    assert_eq!(chance_setter_ext(&ext_only).as_deref(), Some(".co2"));
    // 整个 [save] 节缺失，或文件为空 → auto + .dbb
    let no_save = IniDoc::parse(b"[general]\nenabled = true\n");
    assert_eq!(chance_setter_ext(&no_save).as_deref(), Some(".dbb"));
    assert_eq!(
        chance_setter_ext(&IniDoc::parse(b"")).as_deref(),
        Some(".dbb")
    );
}

#[test]
fn nrsc_ext_defaults_to_co2_when_missing() {
    assert_eq!(NRSC_DEFAULT_EXT, "co2");
    assert_eq!(nrsc_ext(None), "co2");
    // 文件在但没有这个键，同样取默认
    assert_eq!(nrsc_ext(Some(&IniDoc::parse(b"[SAVE]\r\n"))), "co2");
    // 有键就按原文返回，不规范化
    let custom = IniDoc::parse(b"[save]\nSAVE_FILE_EXTENSION = Dbb\n");
    assert_eq!(nrsc_ext(Some(&custom)), "Dbb");
    for rel in [
        "pack/SeamlessCoop/nrsc_settings.ini",
        "game_nrsc_settings.ini",
    ] {
        let doc = read_ini(&fixture(rel)).unwrap();
        assert_eq!(nrsc_ext(Some(&doc)), "co2", "{rel}");
    }
}

#[test]
fn me3_ext_reads_savefile_extension() {
    let profile = Me3Profile::load(&fixture("pack/褪色燃烧.me3")).unwrap();
    assert_eq!(me3_ext(&profile).as_deref(), Some("co2"));
    let none = Me3Profile {
        savefile: None,
        ..profile.clone()
    };
    assert_eq!(me3_ext(&none), None);
    let bare = Me3Profile {
        savefile: Some("NR0000".into()),
        ..profile
    };
    assert_eq!(me3_ext(&bare).as_deref(), Some(""));
}

#[test]
fn suffix_check_for_real_pack() {
    // 真实整合包：me3 与 nrsc 都是 co2，ChanceSetter 用 .CL_SAVE → 只警告不阻断
    let profile = Me3Profile::load(&fixture("pack/褪色燃烧.me3")).unwrap();
    let nrsc = read_ini(&fixture("pack/SeamlessCoop/nrsc_settings.ini")).unwrap();
    let cs = read_ini(&fixture("pack/dll/ChanceSetter.ini")).unwrap();
    let real = SuffixCheck {
        me3: me3_ext(&profile),
        nrsc: nrsc_ext(Some(&nrsc)),
        chance_setter: chance_setter_ext(&cs),
    };
    assert_eq!(real, check(Some("co2"), "co2", Some(".CL_SAVE")));
    assert!(!real.blocking() && real.warning());
    // 交给前端时键用 camelCase，值保留原文
    assert_eq!(
        serde_json::to_value(&real).unwrap(),
        serde_json::json!({ "me3": "co2", "nrsc": "co2", "chanceSetter": ".CL_SAVE" })
    );
}

#[test]
fn suffix_strips_paired_quotes() {
    assert_eq!(normalize_ext("\".co2\""), "co2");
    assert!(!check(Some("co2"), "co2", Some("\".co2\"")).warning());
    // path 带引号也按路径取扩展名
    let doc = IniDoc::parse(b"[save]\npath = \"D:\\x\\NR0000.co2\"\n");
    assert_eq!(chance_setter_ext(&doc).as_deref(), Some("co2"));
    let auto = IniDoc::parse(b"[save]\npath = \"auto\"\nautoExtension = .co2\n");
    assert_eq!(chance_setter_ext(&auto).as_deref(), Some(".co2"));
}

/// 无缝联机对空后缀的处理未知：按不一致拦下，一键对齐能修
#[test]
fn nrsc_empty_value_blocks() {
    let doc = IniDoc::parse(b"[SAVE]\nsave_file_extension =\n");
    assert_eq!(nrsc_ext(Some(&doc)), "");
    assert!(check(Some("co2"), &nrsc_ext(Some(&doc)), None).blocking());
}
