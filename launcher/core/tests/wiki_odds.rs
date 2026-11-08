//! 图鉴的几率：按 ChanceSetter 和 nighter 现算

mod common;

use common::fake_pack;
use launcher_core::ini::IniDoc;
use launcher_core::nighter::NighterConfig;
use launcher_core::paths::PackLayout;
use launcher_core::wiki::odds::{WikiOdds, odds_from, read_wiki_odds};

const NIGHTLORDS: [&str; 10] = [
    "Gladius",
    "Adel",
    "Gnoster",
    "Maris",
    "Libra",
    "Fulghor",
    "Caligo",
    "Heolstor",
    "Harmonia",
    "Straghess",
];
const EVERDARK: [&str; 8] = [
    "Gladius", "Adel", "Gnoster", "Maris", "Libra", "Fulghor", "Caligo", "Harmonia",
];
const MAPS: [&str; 6] = [
    "Standard",
    "TheCrater",
    "Mountaintop",
    "RottedWoods",
    "ShroudedCity",
    "GreatHollow",
];

fn depth_sections(name: &str, keys: &[&str], value: &str) -> String {
    let mut text = format!("[{name}]\nenabled = true\n");
    for depth in 1..=5 {
        text += &format!("[{name}.depth{depth}]\n");
        for key in keys {
            text += &format!("{key} = {value}\n");
        }
    }
    text
}

fn map_section() -> String {
    let mut text = "[mapChances]\nenabled = true\n".to_owned();
    for key in MAPS {
        text += &format!("{key} = 7\n");
    }
    text
}

fn full_ini() -> String {
    format!(
        "{}{}{}",
        depth_sections("nightlordChances", &NIGHTLORDS, "10"),
        depth_sections("everdarkChances", &EVERDARK, "25"),
        map_section()
    )
}

fn doc(text: &str) -> IniDoc {
    IniDoc::parse(text.as_bytes())
}

fn nighter(force_enable: bool, level: u8) -> NighterConfig {
    NighterConfig {
        force_unlock: false,
        force_enable,
        level,
        super_lords: Vec::new(),
    }
}

#[test]
fn wiki_odds_forced_level() {
    let odds = odds_from(Some(&doc(&full_ini())), Some(&nighter(true, 3)));
    assert_eq!(odds.level, Some(3));
    assert_eq!(odds.nightlords.unwrap()["Caligo"], [10.0; 5]);
}

#[test]
fn wiki_odds_not_forced() {
    let odds = odds_from(Some(&doc(&full_ini())), Some(&nighter(false, 3)));
    assert_eq!(odds.level, None);
}

#[test]
fn wiki_odds_level_out_of_range() {
    let cs = doc(&full_ini());
    assert_eq!(odds_from(Some(&cs), Some(&nighter(true, 6))).level, None);
    assert_eq!(odds_from(Some(&cs), Some(&nighter(true, 0))).level, None);
    assert_eq!(odds_from(Some(&cs), Some(&nighter(true, 5))).level, Some(5));
}

#[test]
fn wiki_odds_no_files() {
    assert_eq!(odds_from(None, None), WikiOdds::default());
}

#[test]
fn wiki_odds_disabled_section_is_none() {
    let off = |name: &str| {
        let on = format!("[{name}]\nenabled = true");
        let text = full_ini().replace(&on, &format!("[{name}]\nenabled = false"));
        odds_from(Some(&doc(&text)), None)
    };
    let a = off("nightlordChances");
    assert!(a.nightlords.is_none() && a.everdark.is_some() && a.maps.is_some());
    let b = off("everdarkChances");
    assert!(b.everdark.is_none() && b.nightlords.is_some() && b.maps.is_some());
    let c = off("mapChances");
    assert!(c.maps.is_none() && c.nightlords.is_some() && c.everdark.is_some());
}

#[test]
fn wiki_odds_missing_key_skips_that_lord() {
    let text = full_ini().replacen("[nightlordChances.depth3]\nGladius = 10\n", "[nightlordChances.depth3]\n", 1);
    let lords = odds_from(Some(&doc(&text)), None).nightlords.unwrap();
    assert!(!lords.contains_key("Gladius"));
    assert_eq!(lords.len(), 9);
}

#[test]
fn wiki_odds_non_numeric_skips() {
    let text = full_ini().replacen("Adel = 10", "Adel = abc", 1);
    let lords = odds_from(Some(&doc(&text)), None).nightlords.unwrap();
    assert!(!lords.contains_key("Adel"));
}

#[test]
fn wiki_odds_not_finite_skips() {
    let text = full_ini().replacen("Adel = 10", "Adel = NaN", 1);
    let lords = odds_from(Some(&doc(&text)), None).nightlords.unwrap();
    assert!(!lords.contains_key("Adel"));
}

#[test]
fn wiki_odds_keeps_decimals() {
    let text = full_ini().replacen("TheCrater = 7", "TheCrater = 12.5", 1);
    let maps = odds_from(Some(&doc(&text)), None).maps.unwrap();
    assert_eq!(maps["TheCrater"], 12.5);
}

#[test]
fn wiki_odds_fixture_counts() {
    let fp = fake_pack("pack");
    let pack = PackLayout {
        root: fp.pack_root.clone(),
    };
    let odds = read_wiki_odds(&pack);
    assert_eq!(odds.nightlords.unwrap().len(), 10);
    assert_eq!(odds.everdark.unwrap().len(), 8);
    assert_eq!(odds.maps.unwrap().len(), 6);
}
