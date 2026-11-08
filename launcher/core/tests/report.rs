//! 整理问题记录：选哪些文件、崩溃转储的取舍、打包结果里没有账号信息

mod common;

use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use common::backup::{lock_exclusive, now};
use common::minidump::{exit_crash, game_read_av, write_dump};
use launcher_core::paths::{GameDir, PackLayout};
use launcher_core::report::sanitize::Identities;
use launcher_core::report::select::{latest_me3_logs, recent_files, select_dumps};
use launcher_core::report::{ReportInputs, create, preview};
use tempfile::TempDir;

const USER: &str = "TestUserName";
const STEAM: &str = "76561198000000001";
const MB: u64 = 1024 * 1024;

fn ago(secs: u64) -> SystemTime {
    SystemTime::now() - Duration::from_secs(secs)
}

fn file_at(dir: &Path, name: &str, body: &[u8], modified: SystemTime) -> PathBuf {
    fs::create_dir_all(dir).unwrap();
    let path = dir.join(name);
    fs::write(&path, body).unwrap();
    File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_modified(modified)
        .unwrap();
    path
}

struct Fixture {
    tmp: TempDir,
    inputs: ReportInputs,
}

impl Fixture {
    fn pack(&self) -> PathBuf {
        self.inputs.pack.root.clone()
    }
    fn me3(&self) -> PathBuf {
        self.inputs.me3_log_dir.clone().unwrap()
    }
    fn dumps(&self) -> PathBuf {
        let dir = self.inputs.game.as_ref().unwrap().dump_reports();
        fs::create_dir_all(&dir).unwrap();
        dir
    }
}

fn fixture() -> Fixture {
    let tmp = TempDir::new().unwrap();
    let pack = tmp.path().join("pack");
    let log = format!("C:\\Users\\{USER}\\AppData {STEAM} line\n");
    let body = log.as_bytes();
    let t = ago(60);
    file_at(&pack.join(r"launcher\logs"), "launcher.log", body, t);
    file_at(&pack.join(r"StatAlloc\logs"), "nr_stat_alloc.log", body, t);
    file_at(&pack.join("logs"), "a.log", body, t);
    file_at(&pack.join(r"dll\logs"), "b.log", body, t);
    file_at(&pack.join("DepthChange"), "nighter.json", body, t);
    file_at(&pack.join("dll"), "ChanceSetter.ini", body, t);
    file_at(&pack.join("SeamlessCoop"), "nrsc_settings.ini", body, t);
    file_at(&pack.join("StatAlloc"), "nr_stat_alloc.toml", body, t);
    let me3 = tmp.path().join("me3logs");
    file_at(&me3, "old.log", body, ago(3000));
    file_at(&me3, "mid.log", body, ago(2000));
    file_at(&me3, "new.log", body, ago(1000));
    let desktop = tmp.path().join("desktop");
    fs::create_dir_all(&desktop).unwrap();
    let inputs = ReportInputs {
        pack: PackLayout { root: pack },
        game: Some(GameDir {
            root: tmp.path().join("game"),
        }),
        me3_log_dir: Some(me3),
        desktop,
        identities: Identities {
            names: vec![USER.as_bytes().to_vec()],
            steam_ids: vec![STEAM.parse().unwrap()],
        },
        versions: format!("整合包 1.9.5\n路径 {USER}\n"),
    };
    Fixture { tmp, inputs }
}

fn unzip(path: &Path) -> Vec<(String, Vec<u8>)> {
    let mut archive = zip::ZipArchive::new(File::open(path).unwrap()).unwrap();
    (0..archive.len())
        .map(|i| {
            let mut entry = archive.by_index(i).unwrap();
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes).unwrap();
            (entry.name().unwrap().into_owned(), bytes)
        })
        .collect()
}

fn entry<'a>(all: &'a [(String, Vec<u8>)], name: &str) -> &'a [u8] {
    &all.iter()
        .find(|(n, _)| n == name)
        .unwrap_or_else(|| panic!("缺 {name}"))
        .1
}

fn run(fx: &Fixture) -> PathBuf {
    create(&fx.inputs, SystemTime::now(), now()).unwrap()
}

fn names(all: &[(String, Vec<u8>)]) -> Vec<&str> {
    all.iter().map(|(n, _)| n.as_str()).collect()
}

#[test]
fn selects_latest_two_me3_logs() {
    let fx = fixture();
    let picked: Vec<String> = latest_me3_logs(&fx.me3())
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(picked, ["new.log", "mid.log"]);
}

#[test]
fn old_mod_logs_excluded() {
    let tmp = TempDir::new().unwrap();
    file_at(tmp.path(), "fresh.log", b"x", ago(71 * 3600));
    file_at(tmp.path(), "old.log", b"x", ago(73 * 3600));
    let picked = recent_files(tmp.path(), SystemTime::now());
    assert_eq!(picked, [tmp.path().join("fresh.log")]);
}

#[test]
fn dumps_outside_span_excluded() {
    let tmp = TempDir::new().unwrap();
    write_dump(tmp.path(), "old.dmp", &game_read_av(), ago(5000));
    write_dump(tmp.path(), "new.dmp", &game_read_av(), ago(100));
    let picked = select_dumps(tmp.path(), ago(1000));
    assert_eq!(picked.len(), 1);
    assert_eq!(picked[0].name, "new.dmp");
}

#[test]
fn prefers_real_crash_over_exit_crash() {
    let tmp = TempDir::new().unwrap();
    write_dump(tmp.path(), "exit1.dmp", &exit_crash(), ago(10));
    write_dump(tmp.path(), "exit2.dmp", &exit_crash(), ago(20));
    write_dump(tmp.path(), "real.dmp", &game_read_av(), ago(500));
    let picked = select_dumps(tmp.path(), ago(1000));
    assert_eq!(picked[0].name, "real.dmp");
    assert!(picked[0].real_crash);
    assert_eq!(picked[1].name, "exit1.dmp");
    assert!(!picked[1].real_crash);
    assert_eq!(picked.len(), 2);
}

fn sized(dir: &Path, name: &str, size: u64, modified: SystemTime) {
    let path = file_at(dir, name, b"", modified);
    let f = File::options().write(true).open(&path).unwrap();
    f.set_len(size).unwrap();
    f.set_modified(modified).unwrap();
}

#[test]
fn over_20mb_keeps_first_only() {
    let tmp = TempDir::new().unwrap();
    sized(tmp.path(), "a.dmp", 12 * MB, ago(10));
    sized(tmp.path(), "b.dmp", 12 * MB, ago(20));
    let picked = select_dumps(tmp.path(), ago(1000));
    assert_eq!(picked.len(), 1);
    assert_eq!(picked[0].name, "a.dmp");
}

#[test]
fn single_dump_over_limit_is_listed_not_packed() {
    let fx = fixture();
    sized(&fx.dumps(), "huge.dmp", 21 * MB, ago(10));
    assert!(preview(&fx.inputs, SystemTime::now()).dumps.is_empty());
    let all = unzip(&run(&fx));
    assert!(names(&all).iter().all(|n| !n.starts_with("dumps/")));
    let list = String::from_utf8(entry(&all, "清单.txt").to_vec()).unwrap();
    assert!(list.contains("huge.dmp"), "{list}");
}

#[test]
fn preview_lists_dumps_and_total() {
    let fx = fixture();
    write_dump(&fx.dumps(), "real.dmp", &game_read_av(), ago(10));
    let p = preview(&fx.inputs, SystemTime::now());
    assert_eq!(p.dumps.len(), 1);
    assert_eq!(p.dumps[0].name, "real.dmp");
    assert_eq!(p.dump_bytes, p.dumps[0].size);
}

#[test]
fn zip_has_expected_entries() {
    let fx = fixture();
    write_dump(&fx.dumps(), "real.dmp", &game_read_av(), ago(10));
    let path = run(&fx);
    assert_eq!(
        path.file_name().unwrap().to_string_lossy(),
        "黄金时代问题记录_2026-11-06_2140.zip"
    );
    let all = unzip(&path);
    let got = names(&all);
    for want in [
        "me3/new.log",
        "me3/mid.log",
        "launcher/launcher.log",
        "StatAlloc/nr_stat_alloc.log",
        "logs/a.log",
        "dll_logs/b.log",
        "config/nighter.json",
        "config/ChanceSetter.ini",
        "config/nrsc_settings.ini",
        "config/nr_stat_alloc.toml",
        "dumps/real.dmp",
        "版本.txt",
        "清单.txt",
    ] {
        assert!(got.contains(&want), "缺 {want}：{got:?}");
    }
    assert!(!got.contains(&"me3/old.log"));
}

#[test]
fn zip_has_no_identity() {
    let fx = fixture();
    write_dump(
        &fx.dumps(),
        &format!("{USER}.dmp"),
        &game_read_av(),
        ago(10),
    );
    file_at(
        &fx.pack().join("logs"),
        &format!("{STEAM}.log"),
        b"x",
        ago(5),
    );
    let all = unzip(&run(&fx));
    let contains =
        |hay: &[u8], needle: &str| hay.windows(needle.len()).any(|w| w == needle.as_bytes());
    for (name, bytes) in &all {
        assert!(!name.contains(USER) && !name.contains(STEAM), "{name}");
        if !name.starts_with("dumps/") {
            assert!(!contains(bytes, USER), "{name}");
            assert!(!contains(bytes, STEAM), "{name}");
        }
    }
    assert!(contains(entry(&all, "launcher/launcher.log"), "<用户名>"));
}

#[test]
fn locked_file_is_skipped_and_listed() {
    let fx = fixture();
    let held = lock_exclusive(&fx.pack().join(r"logs\a.log"));
    let all = unzip(&run(&fx));
    drop(held);
    assert!(!names(&all).contains(&"logs/a.log"));
    assert!(names(&all).contains(&"launcher/launcher.log"));
    let list = String::from_utf8(entry(&all, "清单.txt").to_vec()).unwrap();
    assert!(list.contains("a.log"), "{list}");
}

#[test]
fn big_log_keeps_tail() {
    let fx = fixture();
    let path = fx.pack().join(r"logs\big.log");
    let mut body = vec![b'x'; 9 * MB as usize];
    body.extend_from_slice(b"THE-END");
    fs::write(&path, body).unwrap();
    let all = unzip(&run(&fx));
    let got = entry(&all, "logs/big.log");
    assert!(got.len() <= 8 * MB as usize);
    assert!(got.ends_with(b"THE-END"));
    let list = String::from_utf8(entry(&all, "清单.txt").to_vec()).unwrap();
    assert!(list.contains("big.log"), "{list}");
}

#[test]
fn name_collision_gets_suffix_no_tmp_left() {
    let fx = fixture();
    let first = run(&fx);
    let second = run(&fx);
    let third = run(&fx);
    assert_ne!(first, second);
    assert_eq!(
        second.file_name().unwrap().to_string_lossy(),
        "黄金时代问题记录_2026-11-06_2140 (2).zip"
    );
    assert_eq!(
        third.file_name().unwrap().to_string_lossy(),
        "黄金时代问题记录_2026-11-06_2140 (3).zip"
    );
    assert_eq!(fs::read_dir(&fx.inputs.desktop).unwrap().count(), 3);
    let _ = &fx.tmp;
}

#[test]
fn missing_game_and_me3_dir_still_works() {
    let mut fx = fixture();
    fx.inputs.game = None;
    fx.inputs.me3_log_dir = None;
    let all = unzip(&run(&fx));
    assert!(names(&all).contains(&"launcher/launcher.log"));
}
