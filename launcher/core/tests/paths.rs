//! 找整合包（discover）、找游戏目录（find_game_dir）与各文件位置

mod common;

use std::cell::Cell;
use std::fs;
use std::path::{Path, PathBuf};

use common::{fake_pack, fixture};
use launcher_core::paths::{GameDir, PROFILE_FILE, PackLayout, PathsError, find_game_dir};
use launcher_core::profile::Me3Profile;
use tempfile::TempDir;

/// 在 dir 里放一份真实的 褪色燃烧.me3
fn put_profile(dir: &Path) {
    fs::create_dir_all(dir).unwrap();
    fs::copy(fixture("pack").join(PROFILE_FILE), dir.join(PROFILE_FILE)).unwrap();
}

/// 在 root 下建 Game\nightreign.exe（空文件），返回 root
fn make_game_root(root: &Path) -> PathBuf {
    fs::create_dir_all(root.join("Game")).unwrap();
    fs::write(root.join(r"Game\nightreign.exe"), b"").unwrap();
    root.to_path_buf()
}

#[test]
fn discover_finds_profile_in_exe_dir() {
    let tmp = TempDir::new().unwrap();
    let exe_dir = tmp.path().join("pack");
    put_profile(&exe_dir);
    assert_eq!(
        PackLayout::discover(&exe_dir, None),
        Ok(PackLayout {
            root: exe_dir.clone()
        })
    );
}

#[test]
fn discover_prefers_env_override() {
    let tmp = TempDir::new().unwrap();
    let exe_dir = tmp.path().join("exe");
    let env_dir = tmp.path().join("env");
    put_profile(&exe_dir);
    put_profile(&env_dir);
    assert_eq!(
        PackLayout::discover(&exe_dir, Some(&env_dir)).unwrap().root,
        env_dir
    );
    // 环境变量指的目录里没有 .me3 时退回 exe 所在目录
    let empty = tmp.path().join("empty");
    fs::create_dir_all(&empty).unwrap();
    assert_eq!(
        PackLayout::discover(&exe_dir, Some(&empty)).unwrap().root,
        exe_dir
    );
}

#[test]
fn discover_fails_outside_pack() {
    // 模拟把 exe 复制到桌面：目录里没有 .me3
    let tmp = TempDir::new().unwrap();
    let exe_dir = tmp.path().join("Desktop");
    fs::create_dir_all(&exe_dir).unwrap();
    assert_eq!(
        PackLayout::discover(&exe_dir, None),
        Err(PathsError::PackNotFound {
            searched: vec![exe_dir.clone()]
        })
    );
    // 给了环境变量也找不到：按查找顺序列出两个目录
    let env_dir = tmp.path().join("env");
    assert_eq!(
        PackLayout::discover(&exe_dir, Some(&env_dir)),
        Err(PathsError::PackNotFound {
            searched: vec![env_dir.clone(), exe_dir.clone()]
        })
    );
}

#[test]
fn discover_makes_relative_override_absolute() {
    // cargo 让集成测试在本 crate 根目录下运行，相对路径 tests\fixtures\pack 里有 .me3
    let cwd = std::env::current_dir().unwrap();
    let tmp = TempDir::new().unwrap();
    let exe_dir = tmp.path().join("Desktop");
    fs::create_dir_all(&exe_dir).unwrap();
    let pack = PackLayout::discover(&exe_dir, Some(Path::new(r"tests\fixtures\pack"))).unwrap();
    assert!(pack.root.is_absolute());
    assert_eq!(pack.root, cwd.join(r"tests\fixtures\pack"));
    assert!(pack.profile().is_absolute());
    // 找不到时列出的也是转换后的绝对路径
    assert_eq!(
        PackLayout::discover(&exe_dir, Some(Path::new("no-such-pack"))),
        Err(PathsError::PackNotFound {
            searched: vec![cwd.join("no-such-pack"), exe_dir.clone()]
        })
    );
}

#[test]
fn game_dir_found_by_walking_up() {
    let fp = fake_pack("game");
    assert_eq!(
        find_game_dir(&fp.pack_root, None, &|| None),
        Some(GameDir {
            root: fp.game_root.clone()
        })
    );

    // 往上找到的优先于手选目录和 Steam，且找到后不再问 Steam
    let other = make_game_root(&fp.tmp.path().join("other"));
    let steam_calls = Cell::new(0);
    let steam = || {
        steam_calls.set(steam_calls.get() + 1);
        Some(other.clone())
    };
    assert_eq!(
        find_game_dir(&fp.pack_root, Some(&other), &steam),
        Some(GameDir {
            root: fp.game_root.clone()
        })
    );
    assert_eq!(steam_calls.get(), 0);

    // 最多往上 5 级：第 5 级找得到，第 6 级不再找
    let game = make_game_root(&fp.tmp.path().join("deep"));
    let five = game.join(r"1\2\3\4\5");
    let six = five.join("6");
    fs::create_dir_all(&six).unwrap();
    assert_eq!(
        find_game_dir(&five, None, &|| None),
        Some(GameDir { root: game.clone() })
    );
    assert_eq!(find_game_dir(&six, None, &|| None), None);
}

#[test]
fn game_dir_uses_configured_then_steam() {
    let tmp = TempDir::new().unwrap();
    // 埋深 6 级，往上 5 级都在临时目录内，结果不受 TEMP 位置影响
    let pack_root = tmp.path().join(r"a\b\c\d\e\pack");
    fs::create_dir_all(&pack_root).unwrap();
    let chosen = make_game_root(&tmp.path().join("chosen"));
    let library = make_game_root(&tmp.path().join("library"));
    let not_game = tmp.path().join("not_game");
    fs::create_dir_all(&not_game).unwrap();
    let steam = || Some(library.clone());

    // 手选目录有效：优先于 Steam
    assert_eq!(
        find_game_dir(&pack_root, Some(&chosen), &steam),
        Some(GameDir {
            root: chosen.clone()
        })
    );
    // 手选目录无效或没选：用 Steam
    assert_eq!(
        find_game_dir(&pack_root, Some(&not_game), &steam),
        Some(GameDir {
            root: library.clone()
        })
    );
    assert_eq!(
        find_game_dir(&pack_root, None, &steam),
        Some(GameDir {
            root: library.clone()
        })
    );
    // 都没有 → None；Steam 给的目录里没有 nightreign.exe 也不算
    assert_eq!(find_game_dir(&pack_root, Some(&not_game), &|| None), None);
    assert_eq!(
        find_game_dir(&pack_root, None, &|| Some(not_game.clone())),
        None
    );
}

#[test]
fn discover_and_game_dir_with_spaces_and_cjk() {
    let fp = fake_pack("ELDEN RING 黑夜 君临");
    let pack = PackLayout::discover(&fp.pack_root, None).unwrap();
    assert_eq!(pack.root, fp.pack_root);
    let game = find_game_dir(&pack.root, None, &|| None).unwrap();
    assert_eq!(game.root, fp.game_root);
    assert!(game.exe().is_file());
}

#[test]
fn env_override_with_spaces_and_cjk() {
    let fp = fake_pack("ELDEN RING 黑夜 君临");
    let elsewhere = fp.tmp.path().join("别处 目录");
    fs::create_dir_all(&elsewhere).unwrap();
    assert_eq!(
        PackLayout::discover(&elsewhere, Some(&fp.pack_root))
            .unwrap()
            .root,
        fp.pack_root
    );
}

/// 假整合包按 .me3 补齐了文件：配置读得出来，也没有缺失
#[test]
fn fake_pack_has_no_missing_files() {
    let fp = fake_pack("ELDEN RING 黑夜 君临");
    let pack = PackLayout::discover(&fp.pack_root, None).unwrap();
    assert!(pack.me3_exe().is_file());
    let profile = Me3Profile::load(&pack.profile()).unwrap();
    assert!(profile.missing_files().is_empty());
}

#[test]
fn layout_paths_match_pack_structure() {
    let r = Path::new(r"D:\G R\me3\MMV\me3");
    let pack = PackLayout {
        root: r.to_path_buf(),
    };
    assert_eq!(pack.profile(), r.join("褪色燃烧.me3"));
    assert_eq!(pack.me3_exe(), r.join(r"bin\me3.exe"));
    assert_eq!(pack.launcher_dir(), r.join("launcher"));
    assert_eq!(pack.settings_file(), r.join(r"launcher\settings.json"));
    assert_eq!(pack.logs_dir(), r.join(r"launcher\logs"));
    assert_eq!(pack.manifest(), r.join("manifest.json"));
    assert_eq!(pack.nighter_json(), r.join(r"DepthChange\nighter.json"));
    assert_eq!(pack.chance_setter_ini(), r.join(r"dll\ChanceSetter.ini"));
    assert_eq!(pack.nrsc_ini(), r.join(r"SeamlessCoop\nrsc_settings.ini"));

    let g = Path::new(r"D:\G R");
    let game = GameDir {
        root: g.to_path_buf(),
    };
    assert_eq!(game.exe(), g.join(r"Game\nightreign.exe"));
    assert_eq!(
        game.dump_reports(),
        g.join(r"Game\SeamlessCoop\crashdumps\reports")
    );
}
