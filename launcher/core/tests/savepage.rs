//! 存档页：账号选择、列表、手动备份、按名字恢复、从原版存档复制

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use common::backup::{later, now};
use common::fake_os::{FakeOs, Script};
use launcher_core::os::GAME_EXE;
use launcher_core::savepage::{
    BackupNow, RestoreRefused, SaveContext, backup_now, restore_by_name, saves_view,
};
use launcher_core::saves::{SaveLocation, VanillaCopy, copy_from_vanilla, save_busy, vanilla_save};
use launcher_core::suffix::SuffixView;
use tempfile::TempDir;

const A: u64 = 76561198000000001;
const B: u64 = 76561198000000002;
const SAVE: &str = "NR0000.co2";

fn suffix() -> SuffixView {
    SuffixView {
        me3: Some("co2".into()),
        nrsc: "co2".into(),
        chance_setter: Some(".co2".into()),
        consistent: true,
        blocking: false,
        path_manual: false,
    }
}

fn idle() -> Arc<FakeOs> {
    FakeOs::new(Script::default())
}

fn running() -> Arc<FakeOs> {
    FakeOs::new(Script {
        already_running: vec![GAME_EXE],
        ..Script::default()
    })
}

fn account_dir(root: &Path, id: u64) -> PathBuf {
    let dir = root.join(id.to_string());
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn with_save(root: &Path, id: u64) -> PathBuf {
    let file = account_dir(root, id).join(SAVE);
    fs::write(&file, b"save").unwrap();
    file
}

fn ctx<'a>(
    root: &'a Path,
    os: &'a FakeOs,
    active: Option<u64>,
    remembered: Option<u64>,
) -> SaveContext<'a> {
    SaveContext {
        root: Some(root),
        savefile: SAVE,
        active,
        remembered,
        os,
    }
}

fn location(root: &Path, id: u64) -> SaveLocation {
    let dir = root.join(id.to_string());
    SaveLocation {
        account: id,
        file: dir.join(SAVE),
        dir,
    }
}

#[test]
fn manual_backup_creates_row() {
    let tmp = TempDir::new().unwrap();
    with_save(tmp.path(), A);
    let os = idle();
    match backup_now(&ctx(tmp.path(), &os, Some(A), None), A, false, now()).unwrap() {
        BackupNow::Created { row } => {
            assert_eq!(row.kind, "manual");
            assert_eq!(row.taken_at, "2026-11-06 21:40:05");
            assert_eq!(row.size, 4);
            assert_eq!(row.name, "NR0000_2026-11-06_214005_manual.co2.bak");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn manual_backup_needs_confirm_while_running() {
    let tmp = TempDir::new().unwrap();
    with_save(tmp.path(), A);
    let os = running();
    let c = ctx(tmp.path(), &os, Some(A), None);
    assert!(matches!(
        backup_now(&c, A, false, now()).unwrap(),
        BackupNow::NeedsConfirm
    ));
    assert!(matches!(
        backup_now(&c, A, true, now()).unwrap(),
        BackupNow::Created { .. }
    ));
}

#[test]
fn manual_backup_without_save_is_nosave() {
    let tmp = TempDir::new().unwrap();
    account_dir(tmp.path(), A);
    let os = idle();
    assert!(matches!(
        backup_now(&ctx(tmp.path(), &os, Some(A), None), A, false, now()).unwrap(),
        BackupNow::NoSave
    ));
}

#[test]
fn restore_refused_while_game_running() {
    let tmp = TempDir::new().unwrap();
    let file = with_save(tmp.path(), A);
    let os = idle();
    let BackupNow::Created { row } =
        backup_now(&ctx(tmp.path(), &os, Some(A), None), A, false, now()).unwrap()
    else {
        panic!()
    };
    fs::write(&file, b"changed").unwrap();
    let os = running();
    let err =
        restore_by_name(&ctx(tmp.path(), &os, Some(A), None), A, &row.name, later(1)).unwrap_err();
    assert!(matches!(err, RestoreRefused::GameRunning));
    assert_eq!(fs::read(&file).unwrap(), b"changed");
}

#[test]
fn restore_by_name_returns_prerestore_row() {
    let tmp = TempDir::new().unwrap();
    let file = with_save(tmp.path(), A);
    let os = idle();
    let c = ctx(tmp.path(), &os, Some(A), None);
    let BackupNow::Created { row } = backup_now(&c, A, false, now()).unwrap() else {
        panic!()
    };
    fs::write(&file, b"changed").unwrap();
    let pre = restore_by_name(&c, A, &row.name, later(1))
        .unwrap()
        .unwrap();
    assert_eq!(pre.kind, "prerestore");
    assert_eq!(fs::read(&file).unwrap(), b"save");
}

#[test]
fn restore_name_outside_list_is_not_found() {
    let tmp = TempDir::new().unwrap();
    with_save(tmp.path(), A);
    let os = idle();
    let c = ctx(tmp.path(), &os, Some(A), None);
    for name in [r"..\x.bak", "NR0000.co2", ""] {
        assert!(matches!(
            restore_by_name(&c, A, name, later(1)),
            Err(RestoreRefused::NotFound)
        ));
    }
}

#[test]
fn vanilla_copied_and_original_untouched() {
    let tmp = TempDir::new().unwrap();
    let dir = account_dir(tmp.path(), A);
    let sl2 = dir.join("NR0000.sl2");
    fs::write(&sl2, b"vanilla").unwrap();
    let save = location(tmp.path(), A);
    assert_eq!(vanilla_save(&save), sl2);
    assert_eq!(copy_from_vanilla(&save).unwrap(), VanillaCopy::Copied);
    assert_eq!(fs::read(&save.file).unwrap(), b"vanilla");
    assert_eq!(fs::read(&sl2).unwrap(), b"vanilla");
    assert_eq!(fs::read_dir(&dir).unwrap().count(), 2);
}

#[test]
fn vanilla_never_overwrites_save() {
    let tmp = TempDir::new().unwrap();
    let file = with_save(tmp.path(), A);
    fs::write(file.with_extension("sl2"), b"vanilla").unwrap();
    assert_eq!(
        copy_from_vanilla(&location(tmp.path(), A)).unwrap(),
        VanillaCopy::AlreadyHasSave
    );
    assert_eq!(fs::read(&file).unwrap(), b"save");
}

#[test]
fn vanilla_missing_reports_no_vanilla() {
    let tmp = TempDir::new().unwrap();
    account_dir(tmp.path(), A);
    assert_eq!(
        copy_from_vanilla(&location(tmp.path(), A)).unwrap(),
        VanillaCopy::NoVanilla
    );
}

#[test]
fn view_can_copy_only_without_co2_with_sl2() {
    let tmp = TempDir::new().unwrap();
    let dir = account_dir(tmp.path(), A);
    let os = idle();
    let c = ctx(tmp.path(), &os, Some(A), None);
    assert!(!saves_view(&c, None, suffix()).can_copy_vanilla);
    fs::write(dir.join("NR0000.sl2"), b"v").unwrap();
    assert!(saves_view(&c, None, suffix()).can_copy_vanilla);
    fs::write(dir.join(SAVE), b"s").unwrap();
    assert!(!saves_view(&c, None, suffix()).can_copy_vanilla);
}

#[test]
fn view_explicit_account_wins() {
    let tmp = TempDir::new().unwrap();
    with_save(tmp.path(), A);
    with_save(tmp.path(), B);
    let os = idle();
    let view = saves_view(&ctx(tmp.path(), &os, Some(A), None), Some(B), suffix());
    assert_eq!(view.current.as_deref(), Some(B.to_string().as_str()));
    assert_eq!(view.active.as_deref(), Some(A.to_string().as_str()));
    assert_eq!(view.accounts, [A.to_string(), B.to_string()]);
    assert_eq!(
        view.save_file,
        Some(
            tmp.path()
                .join(B.to_string())
                .join(SAVE)
                .display()
                .to_string()
        )
    );
}

#[test]
fn view_ambiguous_has_no_current() {
    let tmp = TempDir::new().unwrap();
    with_save(tmp.path(), A);
    with_save(tmp.path(), B);
    let os = idle();
    let view = saves_view(&ctx(tmp.path(), &os, None, None), None, suffix());
    assert_eq!(view.current, None);
    assert_eq!(view.accounts.len(), 2);
    assert!(view.backups.is_empty());
}

#[test]
fn view_lists_backups_newest_first_and_game_state() {
    let tmp = TempDir::new().unwrap();
    with_save(tmp.path(), A);
    let os = idle();
    let c = ctx(tmp.path(), &os, Some(A), None);
    backup_now(&c, A, false, now()).unwrap();
    fs::write(tmp.path().join(A.to_string()).join(SAVE), b"changed").unwrap();
    backup_now(&c, A, false, later(1)).unwrap();
    let view = saves_view(&c, None, suffix());
    assert_eq!(view.backups.len(), 2);
    assert_eq!(view.backups[0].taken_at, "2026-11-06 21:41:05");
    assert!(!view.game_running);
    let os = running();
    assert!(saves_view(&ctx(tmp.path(), &os, Some(A), None), None, suffix()).game_running);
}

#[test]
fn view_without_root_is_empty() {
    let os = idle();
    let c = SaveContext {
        root: None,
        savefile: SAVE,
        active: Some(A),
        remembered: None,
        os: &*os,
    };
    let view = saves_view(&c, None, suffix());
    assert_eq!(view.current, None);
    assert!(view.accounts.is_empty());
}

#[test]
fn view_json_shape() {
    let tmp = TempDir::new().unwrap();
    with_save(tmp.path(), A);
    let os = idle();
    let c = ctx(tmp.path(), &os, Some(A), None);
    backup_now(&c, A, false, now()).unwrap();
    let v = serde_json::to_value(saves_view(&c, None, suffix())).unwrap();
    assert_eq!(v["canCopyVanilla"], false);
    assert_eq!(v["backups"][0]["takenAt"], "2026-11-06 21:40:05");
    assert_eq!(v["suffix"]["chanceSetter"], ".co2");
    let n = serde_json::to_value(BackupNow::NeedsConfirm).unwrap();
    assert_eq!(n["kind"], "needsConfirm");
    assert_eq!(serde_json::to_value(VanillaCopy::Copied).unwrap(), "copied");
}

#[test]
fn save_busy_detects_exclusive_handle() {
    let tmp = TempDir::new().unwrap();
    let file = with_save(tmp.path(), A);
    let save = location(tmp.path(), A);
    assert!(!save_busy(&save));
    let held = common::backup::lock_exclusive(&file);
    assert!(save_busy(&save));
    drop(held);
}

#[test]
fn manual_backup_unchanged_when_same_as_latest_manual() {
    let tmp = TempDir::new().unwrap();
    with_save(tmp.path(), A);
    let os = idle();
    let c = ctx(tmp.path(), &os, Some(A), None);
    assert!(matches!(
        backup_now(&c, A, false, now()).unwrap(),
        BackupNow::Created { .. }
    ));
    assert!(matches!(
        backup_now(&c, A, false, later(1)).unwrap(),
        BackupNow::Unchanged { .. }
    ));
    assert_eq!(saves_view(&c, None, suffix()).backups.len(), 1);
}
