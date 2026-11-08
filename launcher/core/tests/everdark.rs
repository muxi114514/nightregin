//! 永夜王：ChanceSetter 是控制方，nighter 只管深夜层数；一致性判断、一键改正、存档后缀对齐

mod common;

use std::fs;

use common::{FakePack, fake_pack};
use launcher_core::config::ConfigFile;
use launcher_core::config::everdark::{
    LordOdds, apply_factory, apply_lords, everdark_ok, fix_everdark, read_lords,
};
use launcher_core::config::store::ConfigStore;
use launcher_core::ini::IniDoc;
use launcher_core::nighter::read_nighter;
use launcher_core::paths::PackLayout;
use launcher_core::suffix::{SuffixCheck, align_suffix, suffix_view};

const CS_LORDS: [&str; 8] = [
    "Gladius", "Adel", "Gnoster", "Maris", "Libra", "Fulghor", "Caligo", "Harmonia",
];

fn setup() -> (FakePack, ConfigStore) {
    let fp = fake_pack("pack");
    let store = ConfigStore::new(PackLayout {
        root: fp.pack_root.clone(),
    });
    (fp, store)
}

fn cs_path(store: &ConfigStore) -> std::path::PathBuf {
    ConfigFile::ChanceSetter.path(store.pack())
}

fn cs_doc(store: &ConfigStore) -> IniDoc {
    IniDoc::parse(&fs::read(cs_path(store)).unwrap())
}

fn factory_doc(store: &ConfigStore) -> IniDoc {
    IniDoc::parse(&fs::read(ConfigFile::ChanceSetter.default_path(store.pack())).unwrap())
}

fn nighter_lords(store: &ConfigStore) -> Vec<u8> {
    read_nighter(&ConfigFile::Nighter.path(store.pack()))
        .unwrap()
        .super_lords
}

fn nightlord_block(text: &str) -> String {
    let start = text.find("[nightlordChances").unwrap();
    let end = text.find("[everdarkChances]").unwrap();
    text[start..end].to_owned()
}

fn value(doc: &IniDoc, depth: u8, lord: &str) -> String {
    doc.get(&format!("everdarkChances.depth{depth}"), lord)
        .unwrap()
}

/// 指定名单的每一层都是 100，其余都是 0
fn assert_odds(doc: &IniDoc, on: &[usize]) {
    for d in 1..=5 {
        for (i, lord) in CS_LORDS.iter().enumerate() {
            let want = if on.contains(&i) { "100" } else { "0" };
            assert_eq!(value(doc, d, lord), want, "depth{d} {lord}");
        }
    }
}

#[test]
fn picked_writes_100_and_0_and_enables() {
    let (_fp, store) = setup();
    let before = fs::read_to_string(cs_path(&store)).unwrap();
    apply_lords(&store, &[6]).unwrap();
    let doc = cs_doc(&store);
    assert_eq!(
        doc.get("everdarkChances", "enabled").as_deref(),
        Some("true")
    );
    assert_odds(&doc, &[6]);
    let after = fs::read_to_string(cs_path(&store)).unwrap();
    assert_eq!(nightlord_block(&after), nightlord_block(&before));
}

#[test]
fn harmonia_is_the_eighth_cell() {
    let (_fp, store) = setup();
    apply_lords(&store, &[7]).unwrap();
    assert_odds(&cs_doc(&store), &[7]);
}

#[test]
fn all_writes_every_lord_100() {
    let (_fp, store) = setup();
    apply_lords(&store, &[0, 1, 2, 3, 4, 5, 6, 7]).unwrap();
    assert_odds(&cs_doc(&store), &[0, 1, 2, 3, 4, 5, 6, 7]);
}

#[test]
fn none_writes_every_lord_0() {
    let (_fp, store) = setup();
    apply_lords(&store, &[]).unwrap();
    let doc = cs_doc(&store);
    assert_eq!(
        doc.get("everdarkChances", "enabled").as_deref(),
        Some("true")
    );
    assert_odds(&doc, &[]);
}

#[test]
fn lords_never_touch_nighter() {
    let (_fp, store) = setup();
    let path = ConfigFile::Nighter.path(store.pack());
    let before = fs::read(&path).unwrap();
    apply_lords(&store, &[1]).unwrap();
    apply_factory(&store).unwrap();
    assert_eq!(fs::read(&path).unwrap(), before);
}

#[test]
fn factory_restores_original_odds() {
    let (_fp, store) = setup();
    let original = fs::read(cs_path(&store)).unwrap();
    apply_lords(&store, &[2]).unwrap();
    assert_ne!(fs::read(cs_path(&store)).unwrap(), original);
    apply_factory(&store).unwrap();
    assert_eq!(fs::read(cs_path(&store)).unwrap(), original);
}

#[test]
fn factory_without_defaults_reports_missing() {
    let (fp, store) = setup();
    fs::remove_file(fp.pack_root.join(r"launcher\defaults\ChanceSetter.ini")).unwrap();
    let err = apply_factory(&store).unwrap_err();
    assert!(err.is_permanent());
    assert!(err.to_string().contains("出厂配置"), "{err}");
}

#[test]
fn missing_key_writes_nothing() {
    let (_fp, store) = setup();
    let path = cs_path(&store);
    let broken = fs::read_to_string(&path)
        .unwrap()
        .replacen("Harmonia = 25", "Other = 25", 1);
    fs::write(&path, &broken).unwrap();
    assert!(apply_lords(&store, &[]).is_err());
    assert_eq!(fs::read_to_string(&path).unwrap(), broken);
}

#[test]
fn read_factory_odds() {
    let (_fp, store) = setup();
    let view = read_lords(&cs_doc(&store), Some(&factory_doc(&store)));
    assert_eq!(view.odds, LordOdds::Factory);
    assert!(view.list.is_empty());
    assert!(view.has_factory);
}

#[test]
fn read_picked_list() {
    let (_fp, store) = setup();
    apply_lords(&store, &[1, 7]).unwrap();
    let view = read_lords(&cs_doc(&store), Some(&factory_doc(&store)));
    assert_eq!(view.odds, LordOdds::Picked);
    assert_eq!(view.list, [1, 7]);
}

#[test]
fn read_none_and_all_are_picked() {
    let (_fp, store) = setup();
    apply_lords(&store, &[]).unwrap();
    let none = read_lords(&cs_doc(&store), None);
    assert_eq!((none.odds, none.list.len()), (LordOdds::Picked, 0));
    apply_lords(&store, &[0, 1, 2, 3, 4, 5, 6, 7]).unwrap();
    let all = read_lords(&cs_doc(&store), None);
    assert_eq!((all.odds, all.list.len()), (LordOdds::Picked, 8));
    assert!(!all.has_factory);
}

#[test]
fn read_hand_edited_odds_is_custom() {
    let (_fp, store) = setup();
    let view = read_lords(&cs_doc(&store), None);
    assert_eq!(view.odds, LordOdds::Custom);
    assert!(view.list.is_empty());
}

#[test]
fn read_disabled_is_custom() {
    let (_fp, store) = setup();
    apply_lords(&store, &[1]).unwrap();
    let mut doc = cs_doc(&store);
    doc.set("everdarkChances", "enabled", "false").unwrap();
    assert_eq!(read_lords(&doc, None).odds, LordOdds::Custom);
}

#[test]
fn everdark_ok_compares_nighter_with_factory() {
    assert!(everdark_ok(&[0, 1], &[1, 0]));
    assert!(everdark_ok(&[], &[]));
    assert!(!everdark_ok(&[0], &[]));
    assert!(!everdark_ok(&[], &[0, 1]));
}

#[test]
fn fix_everdark_restores_factory_list() {
    let (fp, store) = setup();
    let factory = fp.pack_root.join(r"launcher\defaults\nighter.json");
    let text = fs::read_to_string(&factory).unwrap();
    let start = text.find("\"superNightLordList\"").unwrap();
    let end = text[start..].find(']').unwrap() + start + 1;
    fs::write(
        &factory,
        format!(
            "{}\"superNightLordList\": []{}",
            &text[..start],
            &text[end..]
        ),
    )
    .unwrap();
    assert_eq!(nighter_lords(&store).len(), 7);
    assert!(fix_everdark(&store).unwrap().wrote);
    assert_eq!(nighter_lords(&store), Vec::<u8>::new());
}

fn nrsc_doc(store: &ConfigStore) -> IniDoc {
    IniDoc::parse(&fs::read(ConfigFile::Nrsc.path(store.pack())).unwrap())
}

fn set_ini(store: &ConfigStore, file: ConfigFile, section: &str, key: &str, value: &str) {
    let path = file.path(store.pack());
    let mut doc = IniDoc::parse(&fs::read(&path).unwrap());
    doc.set(section, key, value).unwrap();
    fs::write(&path, doc.to_bytes()).unwrap();
}

#[test]
fn align_writes_both_with_their_own_dot_style() {
    let (_fp, store) = setup();
    set_ini(
        &store,
        ConfigFile::Nrsc,
        "SAVE",
        "save_file_extension",
        "sl2",
    );
    assert!(align_suffix(&store, "co2").unwrap().wrote);
    assert_eq!(
        nrsc_doc(&store)
            .get("SAVE", "save_file_extension")
            .as_deref(),
        Some("co2")
    );
    assert_eq!(
        cs_doc(&store).get("save", "autoExtension").as_deref(),
        Some(".co2")
    );
}

#[test]
fn align_leaves_manual_path_and_flags_it() {
    let (_fp, store) = setup();
    set_ini(
        &store,
        ConfigFile::ChanceSetter,
        "save",
        "path",
        r"D:\x\NR0000.sl2",
    );
    let before = cs_doc(&store).get("save", "autoExtension");
    align_suffix(&store, "co2").unwrap();
    assert_eq!(cs_doc(&store).get("save", "autoExtension"), before);
    let check = SuffixCheck {
        me3: Some("co2".into()),
        nrsc: "co2".into(),
        chance_setter: Some("sl2".into()),
    };
    let view = suffix_view(&check, Some(&cs_doc(&store)));
    assert!(view.path_manual && !view.consistent && !view.blocking);
}

#[test]
fn align_noop_when_consistent_ignoring_case_and_dot() {
    let (_fp, store) = setup();
    set_ini(
        &store,
        ConfigFile::Nrsc,
        "SAVE",
        "save_file_extension",
        ".CO2",
    );
    set_ini(
        &store,
        ConfigFile::ChanceSetter,
        "save",
        "autoExtension",
        "CO2",
    );
    assert!(!align_suffix(&store, ".co2").unwrap().wrote);
    assert_eq!(
        nrsc_doc(&store)
            .get("SAVE", "save_file_extension")
            .as_deref(),
        Some(".CO2")
    );
}

#[test]
fn suffix_view_flags() {
    let check = SuffixCheck {
        me3: Some("co2".into()),
        nrsc: "sl2".into(),
        chance_setter: Some(".co2".into()),
    };
    let view = suffix_view(&check, None);
    assert!(view.blocking && !view.consistent && !view.path_manual);
    let ok = SuffixCheck {
        nrsc: "co2".into(),
        ..check
    };
    let view = suffix_view(&ok, None);
    assert!(view.consistent && !view.blocking);
}
