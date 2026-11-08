//! 游戏设置的读写：读失败只影响对应的行、手改的值照实显示、取值校验、写回只动目标行

mod common;

use std::fs;

use common::{FakePack, fake_pack};
use launcher_core::config::ConfigFile;
use launcher_core::config::everdark::{LordOdds, LordsView};
use launcher_core::config::game::{
    Field, GameSettings, SettingChange, apply_changes, overlay, read_game_settings,
};
use launcher_core::config::store::ConfigStore;
use launcher_core::ini::IniDoc;
use launcher_core::paths::PackLayout;
use serde_json::json;

fn setup() -> (FakePack, ConfigStore) {
    let fp = fake_pack("pack");
    let store = ConfigStore::new(PackLayout {
        root: fp.pack_root.clone(),
    });
    (fp, store)
}

fn ok<T>(value: T) -> Field<T> {
    Field::Ok { value }
}

fn is_failed<T>(f: &Field<T>, file: ConfigFile) -> bool {
    matches!(f, Field::Failed { file: got, .. } if *got == file)
}

/// 把 nrsc 的一个键改成给定的原文
fn edit_nrsc(store: &ConfigStore, key: &str, raw: &str) {
    let path = ConfigFile::Nrsc.path(store.pack());
    let mut doc = IniDoc::parse(&fs::read(&path).unwrap());
    doc.set("SCALING", key, raw).unwrap();
    fs::write(&path, doc.to_bytes()).unwrap();
}

#[test]
fn reads_pack_defaults() {
    let (_fp, store) = setup();
    let s = read_game_settings(&store);
    assert_eq!(s.force_deep_night, ok(true));
    assert_eq!(s.deep_night_level, ok(5));
    assert_eq!(s.unlock_deep_night, ok(true));
    assert_eq!(
        s.lords,
        ok(LordsView {
            list: vec![],
            odds: LordOdds::Factory,
            has_factory: true
        })
    );
    assert_eq!(
        (s.health_scaling, s.damage_scaling, s.posture_scaling),
        (ok(120), ok(60), ok(100))
    );
    assert!(!s.game_running);
    assert_eq!(s.lord_picks, None);
}

#[test]
fn one_bad_file_fails_only_its_rows() {
    let (_fp, store) = setup();
    fs::write(ConfigFile::Nighter.path(store.pack()), b"garbage").unwrap();
    let s = read_game_settings(&store);
    assert!(is_failed(&s.force_deep_night, ConfigFile::Nighter));
    assert!(is_failed(&s.deep_night_level, ConfigFile::Nighter));
    assert!(is_failed(&s.unlock_deep_night, ConfigFile::Nighter));
    // nighter 只管层数：它坏了不影响永夜王
    assert!(matches!(s.lords, Field::Ok { .. }));
    assert_eq!(
        (s.health_scaling, s.damage_scaling, s.posture_scaling),
        (ok(120), ok(60), ok(100))
    );
}

#[test]
fn missing_nrsc_fails_scaling_rows() {
    let (_fp, store) = setup();
    fs::remove_file(ConfigFile::Nrsc.path(store.pack())).unwrap();
    let s = read_game_settings(&store);
    assert!(is_failed(&s.health_scaling, ConfigFile::Nrsc));
    assert_eq!(s.deep_night_level, ok(5));
}

#[test]
fn non_integer_scaling_fails_that_row() {
    let (_fp, store) = setup();
    edit_nrsc(&store, "damage_scaling", "abc");
    let s = read_game_settings(&store);
    assert!(is_failed(&s.damage_scaling, ConfigFile::Nrsc));
    assert_eq!(s.health_scaling, ok(120));
}

#[test]
fn hand_edited_out_of_range_is_shown() {
    let (_fp, store) = setup();
    edit_nrsc(&store, "health_scaling", "500");
    let nighter = ConfigFile::Nighter.path(store.pack());
    let text = fs::read_to_string(&nighter)
        .unwrap()
        .replace("\"level\": 5", "\"level\": 7");
    fs::write(&nighter, text).unwrap();
    let s = read_game_settings(&store);
    assert_eq!(s.health_scaling, ok(500));
    assert_eq!(s.deep_night_level, ok(7));
}

#[test]
fn validated_rejects_out_of_range() {
    for bad in [
        SettingChange::DeepNightLevel(0),
        SettingChange::DeepNightLevel(6),
        SettingChange::HealthScaling(5),
        SettingChange::DamageScaling(305),
        SettingChange::PostureScaling(125),
        SettingChange::Lords(vec![8]),
    ] {
        assert!(bad.clone().validated().is_err(), "{bad:?}");
    }
    for good in [
        SettingChange::DeepNightLevel(1),
        SettingChange::DeepNightLevel(5),
        SettingChange::HealthScaling(10),
        SettingChange::PostureScaling(300),
        SettingChange::Lords(vec![]),
        SettingChange::ForceDeepNight(false),
    ] {
        assert_eq!(good.clone().validated().as_ref(), Ok(&good));
    }
}

#[test]
fn validated_sorts_and_dedupes_lords() {
    assert_eq!(
        SettingChange::Lords(vec![6, 1, 6, 0]).validated(),
        Ok(SettingChange::Lords(vec![0, 1, 6]))
    );
}

#[test]
fn scaling_write_touches_one_line() {
    let (_fp, store) = setup();
    let path = ConfigFile::Nrsc.path(store.pack());
    let before = fs::read_to_string(&path).unwrap();
    let report = apply_changes(&store, &[SettingChange::HealthScaling(200)]).unwrap();
    assert!(report.wrote);
    let after = fs::read_to_string(&path).unwrap();
    assert_eq!(
        after,
        before.replace("health_scaling = 120", "health_scaling = 200")
    );
}

#[test]
fn nighter_changes_are_applied_together() {
    let (_fp, store) = setup();
    apply_changes(
        &store,
        &[
            SettingChange::DeepNightLevel(3),
            SettingChange::ForceDeepNight(false),
            SettingChange::UnlockDeepNight(false),
        ],
    )
    .unwrap();
    let s = read_game_settings(&store);
    assert_eq!(s.deep_night_level, ok(3));
    assert_eq!(s.force_deep_night, ok(false));
    assert_eq!(s.unlock_deep_night, ok(false));
}

#[test]
fn bypass_online_check_preserved() {
    let (_fp, store) = setup();
    apply_changes(&store, &[SettingChange::DeepNightLevel(3)]).unwrap();
    let text = fs::read_to_string(ConfigFile::Nighter.path(store.pack())).unwrap();
    assert!(text.contains("\"bypassOnlineCheck\": true"), "{text}");
}

#[test]
fn overlay_replaces_failed_and_old_values() {
    let (_fp, store) = setup();
    fs::write(ConfigFile::Nighter.path(store.pack()), b"garbage").unwrap();
    let mut s: GameSettings = read_game_settings(&store);
    overlay(
        &mut s,
        &[
            SettingChange::DeepNightLevel(2),
            SettingChange::HealthScaling(90),
        ],
    );
    assert_eq!(s.deep_night_level, ok(2));
    assert_eq!(s.health_scaling, ok(90));
    assert!(is_failed(&s.force_deep_night, ConfigFile::Nighter));
}

#[test]
fn json_shapes_match_frontend() {
    let (_fp, store) = setup();
    fs::write(ConfigFile::Nighter.path(store.pack()), b"garbage").unwrap();
    let v = serde_json::to_value(read_game_settings(&store)).unwrap();
    assert_eq!(v["healthScaling"], json!({"state": "ok", "value": 120}));
    assert_eq!(v["deepNightLevel"]["state"], "failed");
    assert_eq!(v["deepNightLevel"]["file"], "nighter");
    assert_eq!(v["gameRunning"], false);
    assert_eq!(v["lordPicks"], json!(null));
    let c: SettingChange =
        serde_json::from_value(json!({"key": "deepNightLevel", "value": 3})).unwrap();
    assert_eq!(c, SettingChange::DeepNightLevel(3));
    let c: SettingChange =
        serde_json::from_value(json!({"key": "lords", "value": [1, 2]})).unwrap();
    assert_eq!(c, SettingChange::Lords(vec![1, 2]));
}

#[test]
fn missing_chance_setter_fails_lords_row() {
    let (_fp, store) = setup();
    fs::remove_file(ConfigFile::ChanceSetter.path(store.pack())).unwrap();
    let s = read_game_settings(&store);
    assert!(is_failed(&s.lords, ConfigFile::ChanceSetter));
    assert_eq!(s.deep_night_level, ok(5));
}

#[test]
fn chance_setter_missing_key_fails_lords_row() {
    let (_fp, store) = setup();
    let path = ConfigFile::ChanceSetter.path(store.pack());
    let text = fs::read_to_string(&path)
        .unwrap()
        .replacen("Harmonia = 25", "Other = 25", 1);
    fs::write(&path, text).unwrap();
    assert!(is_failed(
        &read_game_settings(&store).lords,
        ConfigFile::ChanceSetter
    ));
}

/// 手改掉 enabled 也一样：永夜王的写入要用到它
#[test]
fn chance_setter_without_enabled_fails_lords_row() {
    let (_fp, store) = setup();
    let path = ConfigFile::ChanceSetter.path(store.pack());
    let text = fs::read_to_string(&path).unwrap().replacen(
        "enabled = true\r\n\r\n[everdarkChances.depth1]",
        "\r\n[everdarkChances.depth1]",
        1,
    );
    let text = text.replacen(
        "enabled = true\n\n[everdarkChances.depth1]",
        "\n[everdarkChances.depth1]",
        1,
    );
    fs::write(&path, text).unwrap();
    assert!(is_failed(
        &read_game_settings(&store).lords,
        ConfigFile::ChanceSetter
    ));
}

#[test]
fn lords_changes_replace_each_other() {
    assert!(SettingChange::Lords(vec![1]).same_key(&SettingChange::LordsFactory));
    assert!(SettingChange::LordsFactory.same_key(&SettingChange::Lords(vec![])));
    assert!(!SettingChange::Lords(vec![1]).same_key(&SettingChange::DeepNightLevel(1)));
}

#[test]
fn lords_factory_applies_and_overlays() {
    let (_fp, store) = setup();
    apply_changes(&store, &[SettingChange::Lords(vec![2])]).unwrap();
    let mut s = read_game_settings(&store);
    assert!(
        matches!(&s.lords, Field::Ok { value } if value.odds == LordOdds::Picked && value.list == [2])
    );
    overlay(&mut s, &[SettingChange::LordsFactory]);
    assert!(
        matches!(&s.lords, Field::Ok { value } if value.odds == LordOdds::Factory && value.list.is_empty())
    );
    apply_changes(&store, &[SettingChange::LordsFactory]).unwrap();
    let s = read_game_settings(&store);
    assert!(matches!(&s.lords, Field::Ok { value } if value.odds == LordOdds::Factory));
}

#[test]
fn lords_factory_json_shape() {
    let c: SettingChange = serde_json::from_value(json!({"key": "lordsFactory"})).unwrap();
    assert_eq!(c, SettingChange::LordsFactory);
    let (_fp, store) = setup();
    let v = serde_json::to_value(read_game_settings(&store)).unwrap();
    assert_eq!(
        v["lords"]["value"],
        json!({"list": [], "odds": "factory", "hasFactory": true})
    );
}
