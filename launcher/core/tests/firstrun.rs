//! 首次运行要走哪些步

mod common;

use std::fs;
use std::path::Path;

use common::{FakePack, fake_pack};
use launcher_core::firstrun::first_run_plan;
use launcher_core::paths::PackLayout;
use launcher_core::preflight::{IssueKind, PackState};
use launcher_core::settings::Settings;
use tempfile::TempDir;

const A: u64 = 76561198000000001;
const B: u64 = 76561198000000002;

fn state(fp: &FakePack, settings: Settings) -> PackState {
    PackState::load(
        PackLayout {
            root: fp.pack_root.clone(),
        },
        settings,
        &|| None,
    )
}

fn account(root: &Path, id: u64) -> std::path::PathBuf {
    let dir = root.join(id.to_string());
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn plan_game_missing() {
    let fp = fake_pack("game");
    fs::remove_file(fp.game_root.join(r"Game\nightreign.exe")).unwrap();
    let plan = first_run_plan(&state(&fp, Settings::default()), None, None);
    assert!(!plan.game_found);
}

#[test]
fn plan_active_account_needs_no_choice() {
    let fp = fake_pack("game");
    let tmp = TempDir::new().unwrap();
    account(tmp.path(), A);
    account(tmp.path(), B);
    let plan = first_run_plan(&state(&fp, Settings::default()), Some(tmp.path()), Some(B));
    assert!(plan.game_found);
    assert!(!plan.need_account_choice);
    assert_eq!(plan.active_account.as_deref(), Some(B.to_string().as_str()));
    assert_eq!(plan.accounts, [A.to_string(), B.to_string()]);
}

#[test]
fn plan_two_accounts_without_active_need_choice() {
    let fp = fake_pack("game");
    let tmp = TempDir::new().unwrap();
    account(tmp.path(), A);
    account(tmp.path(), B);
    let plan = first_run_plan(&state(&fp, Settings::default()), Some(tmp.path()), None);
    assert!(plan.need_account_choice);
    assert_eq!(plan.active_account, None);
}

#[test]
fn plan_remembered_account_needs_no_choice() {
    let fp = fake_pack("game");
    let tmp = TempDir::new().unwrap();
    account(tmp.path(), A);
    account(tmp.path(), B);
    let settings = Settings {
        save_account: Some(A),
        ..Settings::default()
    };
    assert!(!first_run_plan(&state(&fp, settings), Some(tmp.path()), None).need_account_choice);
}

#[test]
fn plan_single_account_auto() {
    let fp = fake_pack("game");
    let tmp = TempDir::new().unwrap();
    account(tmp.path(), A);
    let plan = first_run_plan(&state(&fp, Settings::default()), Some(tmp.path()), None);
    assert!(!plan.need_account_choice);
}

#[test]
fn plan_new_player_no_accounts_ok() {
    let fp = fake_pack("game");
    let tmp = TempDir::new().unwrap();
    let plan = first_run_plan(&state(&fp, Settings::default()), Some(tmp.path()), None);
    assert!(!plan.need_account_choice && !plan.can_copy_vanilla && plan.accounts.is_empty());
}

#[test]
fn plan_can_copy_vanilla() {
    let fp = fake_pack("game");
    let tmp = TempDir::new().unwrap();
    let dir = account(tmp.path(), A);
    fs::write(dir.join("NR0000.sl2"), b"v").unwrap();
    assert!(
        first_run_plan(&state(&fp, Settings::default()), Some(tmp.path()), Some(A))
            .can_copy_vanilla
    );
    fs::write(dir.join("NR0000.co2"), b"s").unwrap();
    assert!(
        !first_run_plan(&state(&fp, Settings::default()), Some(tmp.path()), Some(A))
            .can_copy_vanilla
    );
}

#[test]
fn plan_lists_only_fixable_issues() {
    let fp = fake_pack("game");
    fs::write(
        fp.pack_root.join(r"launcher\defaults\nighter.json"),
        r#"{"forceUnlockDeepNight":true,"forceDeepNight":{"enable":true,"level":5},"superNightLordList":[]}"#,
    )
    .unwrap();
    let settings = Settings {
        last_exit_abnormal: true,
        ..Settings::default()
    };
    let plan = first_run_plan(&state(&fp, settings), None, None);
    let kinds: Vec<IssueKind> = plan.issues.iter().map(|i| i.kind).collect();
    assert_eq!(
        kinds,
        [IssueKind::ChanceSetterSuffix, IssueKind::EverdarkMismatch]
    );
}

#[test]
fn plan_done_when_flag_set() {
    let fp = fake_pack("game");
    let settings = Settings {
        first_run_done: true,
        ..Settings::default()
    };
    assert!(first_run_plan(&state(&fp, settings), None, None).done);
    assert!(!first_run_plan(&state(&fp, Settings::default()), None, None).done);
}

#[test]
fn plan_json_shape() {
    let fp = fake_pack("game");
    let v =
        serde_json::to_value(first_run_plan(&state(&fp, Settings::default()), None, None)).unwrap();
    for key in [
        "done",
        "gameFound",
        "accounts",
        "activeAccount",
        "needAccountChoice",
        "canCopyVanilla",
        "issues",
    ] {
        assert!(v.get(key).is_some(), "{key}");
    }
}
