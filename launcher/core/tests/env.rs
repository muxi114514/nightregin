//! PackEnv：开始前备份（账号、存档名、错误映射）、启动命令与转储目录，以及接上 run 的完整启动。
//! 真实存档（%APPDATA%\Nightreign）靠 save_root() 断言守住：每次 with_save_root 之后、
//! 第一次 backup() 之前都断言根目录已经换掉，换不成就在备份之前失败。假账号守不住：
//! 没登录时 resolve 会挑唯一的账号目录，本机那一个就是真实账号

mod common;

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use common::backup::{lock_exclusive, now};
use common::fake_os::{FakeOs, Script, game_exits_with};
use common::{FakePack, fake_pack};
use launcher_core::backup::{BACKUP_DIR_NAME, BackupEntry, BackupKind, BackupOutcome};
use launcher_core::config::service::ConfigFlush;
use launcher_core::config::{ConfigFile, ConfigWriteError};
use launcher_core::env::PackEnv;
use launcher_core::launch::{FailReason, LaunchEnv, LaunchJob, LaunchOutcome, LaunchView, run};
use launcher_core::os::{STEAM_EXE, me3_command};
use launcher_core::paths::{GameDir, PackLayout};
use launcher_core::preflight::{IssueKind, PackState, health_issues};
use launcher_core::settings::Settings;
use launcher_core::steam::STEAM_ID64_BASE;
use tempfile::TempDir;

/// 两个账号，A < B
const A: u64 = 76561198000000001;
const B: u64 = 76561198000000002;
/// common::backup::now() 时刻的 NR0000.co2 自动备份
const AUTO_NAME: &str = "NR0000_2026-11-06_214005_auto.co2.bak";
/// save_root 写的存档内容是 17 位账号号
const SAVE_SIZE: u64 = 17;

/// 这个账号登录时注册表里的 ActiveUser
fn active(account: u64) -> Option<u32> {
    Some(u32::try_from(account - STEAM_ID64_BASE).unwrap())
}

/// 假的写回：flush 的结果固定
struct FakeFlush(bool);

impl ConfigFlush for FakeFlush {
    fn flush(&self) -> Result<(), ConfigWriteError> {
        if self.0 {
            Ok(())
        } else {
            Err(ConfigWriteError::Malformed(
                ConfigFile::Nighter,
                "假错误".to_owned(),
            ))
        }
    }
}

fn no_flush() -> Arc<dyn ConfigFlush> {
    Arc::new(FakeFlush(true))
}

fn load(fp: &FakePack, settings: Settings) -> PackState {
    PackState::load(
        PackLayout {
            root: fp.pack_root.clone(),
        },
        settings,
        &|| None,
    )
}

/// Steam 在运行；存档根目录是 root
fn pack_env(fp: &FakePack, settings: Settings, active_user: Option<u32>, root: &Path) -> PackEnv {
    let os = FakeOs::new(Script {
        already_running: vec![STEAM_EXE],
        ..Script::default()
    });
    let env = PackEnv::new(load(fp, settings), os, no_flush(), active_user, now)
        .with_save_root(Some(root.to_path_buf()));
    // 根目录没换成就停在这里，轮不到 backup() 碰真实存档
    assert_eq!(env.save_root(), Some(root));
    env
}

/// <tmp>\Nightreign：每个账号目录里放一个 NR0000.co2，内容是账号号
fn save_root(tmp: &TempDir, accounts: &[u64]) -> PathBuf {
    let root = tmp.path().join("Nightreign");
    fs::create_dir_all(&root).unwrap();
    for id in accounts {
        let dir = root.join(id.to_string());
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("NR0000.co2"), id.to_string()).unwrap();
    }
    root
}

fn backups(root: &Path, account: u64) -> PathBuf {
    root.join(account.to_string()).join(BACKUP_DIR_NAME)
}

/// 账号 account 新建了一份名为 name 的自动备份
fn created(root: &Path, account: u64, name: &str, size: u64) -> Result<BackupOutcome, FailReason> {
    Ok(BackupOutcome::Created(BackupEntry {
        path: backups(root, account).join(name),
        kind: BackupKind::Auto,
        taken_at: now(),
        size,
    }))
}

#[test]
fn pack_env_backup_skips_when_disabled() {
    let fp = fake_pack("game");
    let tmp = TempDir::new().unwrap();
    let root = save_root(&tmp, &[A, B]);
    // 先看开关：没登录、两个账号目录时也不去确定账号，不报 AccountUnknown
    for active_user in [active(A), None, Some(0)] {
        let settings = Settings {
            backup_before_start: false,
            ..Settings::default()
        };
        let env = pack_env(&fp, settings, active_user, &root);
        assert_eq!(env.backup(), Ok(BackupOutcome::NoSave), "{active_user:?}");
    }
    // 存档在也不备份
    assert!(!backups(&root, A).exists() && !backups(&root, B).exists());
}

#[test]
fn pack_env_backup_uses_active_account() {
    let fp = fake_pack("game");
    let tmp = TempDir::new().unwrap();
    let root = save_root(&tmp, &[A, B]);
    // 记住的是 A、登录的是 B：用 B
    let remembered = Settings {
        save_account: Some(A),
        ..Settings::default()
    };
    let env = pack_env(&fp, remembered.clone(), active(B), &root);
    assert_eq!(env.backup(), created(&root, B, AUTO_NAME, SAVE_SIZE));
    assert!(!backups(&root, A).exists());
    // Steam 开着但没登录（ActiveUser 为 0）：用记住的 A
    let env = pack_env(&fp, remembered, Some(0), &root);
    assert_eq!(env.backup(), created(&root, A, AUTO_NAME, SAVE_SIZE));
}

#[test]
fn pack_env_backup_account_unknown() {
    let fp = fake_pack("game");
    let tmp = TempDir::new().unwrap();
    let root = save_root(&tmp, &[B, A]);
    // 两个账号目录，没登录（含 ActiveUser 为 0）也没记住：不猜，升序列出、转成字符串
    let accounts = vec![
        "76561198000000001".to_owned(),
        "76561198000000002".to_owned(),
    ];
    let unknown = Err(FailReason::AccountUnknown { accounts });
    for active_user in [None, Some(0)] {
        let env = pack_env(&fp, Settings::default(), active_user, &root);
        assert_eq!(env.backup(), unknown, "{active_user:?}");
    }
    assert!(!backups(&root, A).exists() && !backups(&root, B).exists());
}

#[test]
fn pack_env_backup_uses_profile_savefile() {
    let fp = fake_pack("game");
    let tmp = TempDir::new().unwrap();
    let root = save_root(&tmp, &[A]);
    fs::write(root.join(A.to_string()).join("NR0001.dbb"), b"dbb").unwrap();
    let profile = fp.pack_root.join("褪色燃烧.me3");
    fs::write(&profile, "savefile = \"NR0001.dbb\"\n").unwrap();
    let env = pack_env(&fp, Settings::default(), active(A), &root);
    let name = "NR0001_2026-11-06_214005_auto.dbb.bak";
    assert_eq!(env.backup(), created(&root, A, name, 3));
    // .me3 没写 savefile：用 NR0000.co2
    fs::write(&profile, "profileVersion = \"v1\"\n").unwrap();
    let env = pack_env(&fp, Settings::default(), active(A), &root);
    assert_eq!(env.backup(), created(&root, A, AUTO_NAME, SAVE_SIZE));
}

#[test]
fn pack_env_backup_nosave_without_account_or_root() {
    let fp = fake_pack("game");
    let tmp = TempDir::new().unwrap();
    // 新玩家：根目录下还没有账号目录，也没登录
    let root = save_root(&tmp, &[]);
    let env = pack_env(&fp, Settings::default(), None, &root);
    assert_eq!(env.backup(), Ok(BackupOutcome::NoSave));
    // 登录了，但这个账号还没进过游戏
    let env = pack_env(&fp, Settings::default(), active(A), &root);
    assert_eq!(env.backup(), Ok(BackupOutcome::NoSave));
    // 没有 APPDATA：找不到存档根目录
    let env = pack_env(&fp, Settings::default(), active(A), &root).with_save_root(None);
    assert_eq!(env.save_root(), None);
    assert_eq!(env.backup(), Ok(BackupOutcome::NoSave));
}

#[test]
fn pack_env_backup_maps_errors() {
    let fp = fake_pack("game");
    let tmp = TempDir::new().unwrap();
    let root = save_root(&tmp, &[A]);
    let env = pack_env(&fp, Settings::default(), active(A), &root);
    let save = root.join(A.to_string()).join("NR0000.co2");
    {
        // 别的程序独占打开着存档
        let _lock = lock_exclusive(&save);
        assert_eq!(env.backup(), Err(FailReason::SaveBusy));
    }
    // 存档名是个目录：打开时拒绝访问。message 只带 io 错误原文，「备份失败：」由界面加
    fs::remove_file(&save).unwrap();
    fs::create_dir(&save).unwrap();
    let message = io::Error::from_raw_os_error(5).to_string();
    assert_eq!(env.backup(), Err(FailReason::Backup { message }));
}

#[test]
fn pack_env_command_and_dumps() {
    let fp = fake_pack("game");
    let tmp = TempDir::new().unwrap();
    let env = pack_env(&fp, Settings::default(), None, tmp.path());
    let pack = PackLayout {
        root: fp.pack_root.clone(),
    };
    let game = GameDir {
        root: fp.game_root.clone(),
    };
    assert_eq!(env.command(), Ok(me3_command(&pack, &game)));
    assert_eq!(env.dumps_dir(), Some(game.dump_reports()));
    // 计划 1 还没有要写的配置
    assert_eq!(env.flush_config(), Ok(()));

    // 找不到游戏：预检本该先拦下；万一走到这里，交出与首页相同的「找不到游戏」
    fs::remove_file(game.exe()).unwrap();
    let env = pack_env(&fp, Settings::default(), None, tmp.path());
    let not_found = health_issues(&load(&fp, Settings::default())).remove(0);
    assert_eq!(
        (not_found.kind, not_found.blocking),
        (IssueKind::GameNotFound, true)
    );
    assert_eq!(
        env.command(),
        Err(FailReason::Preflight { issue: not_found })
    );
    assert_eq!(env.dumps_dir(), None);
}

#[test]
fn pack_env_preflight_uses_its_os_and_job() {
    let fp = fake_pack("game");
    // 假游戏 exe 读不出版本，manifest 又列了版本：版本不符
    let manifest = r#"{"game":[{"fileVersion":"1.3.3.0","size":118835320}]}"#;
    fs::write(fp.pack_root.join("manifest.json"), manifest).unwrap();
    let tmp = TempDir::new().unwrap();
    let dev = Settings {
        dev_mode: true,
        ..Settings::default()
    };
    let ignore = LaunchJob {
        ignore_version: true,
        ..LaunchJob::default()
    };
    let env = pack_env(&fp, dev.clone(), None, tmp.path());
    let got = env.preflight(&LaunchJob::default()).map_err(|i| i.kind);
    assert_eq!(got, Err(IssueKind::GameVersionMismatch));
    assert_eq!(env.preflight(&ignore), Ok(()));
    // 查的是自己的 Os：这里 Steam 没在运行
    let no_steam = PackEnv::new(
        load(&fp, dev),
        FakeOs::new(Script::default()),
        no_flush(),
        None,
        now,
    )
    .with_save_root(None);
    assert_eq!(no_steam.save_root(), None);
    let got = no_steam.preflight(&ignore).map_err(|i| i.kind);
    assert_eq!(got, Err(IssueKind::SteamNotRunning));
}

#[test]
fn pack_env_runs_launch_end_to_end() {
    // 路径含空格和中文；Steam 在运行，游戏 3 秒出现、10 秒出窗口、60 秒以 0 退出
    let fp = fake_pack("ELDEN RING 黑夜 君临");
    let tmp = TempDir::new().unwrap();
    let root = save_root(&tmp, &[A]);
    let os = FakeOs::new(Script {
        already_running: vec![STEAM_EXE],
        ..game_exits_with(0)
    });
    let env = PackEnv::new(
        load(&fp, Settings::default()),
        os.clone(),
        no_flush(),
        active(A),
        now,
    )
    .with_save_root(Some(root.clone()));
    assert_eq!(env.save_root(), Some(root.as_path()));
    let cancel = AtomicBool::new(false);
    let outcome = run(
        &LaunchJob::default(),
        &env,
        &*os,
        &cancel,
        &mut |_: LaunchView| {},
    );
    assert_eq!(outcome, LaunchOutcome::Finished { abnormal: false });
    let pack = PackLayout {
        root: fp.pack_root.clone(),
    };
    let game = GameDir {
        root: fp.game_root.clone(),
    };
    assert_eq!(os.spawned(), [me3_command(&pack, &game)]);
    // 唤起前备份了当前账号的存档
    assert!(backups(&root, A).join(AUTO_NAME).is_file());
}

#[test]
fn pack_env_flush_failure_is_config_write() {
    let fp = fake_pack("game");
    let os = FakeOs::new(Script::default());
    let env = PackEnv::new(
        load(&fp, Settings::default()),
        os,
        Arc::new(FakeFlush(false)),
        None,
        now,
    );
    match env.flush_config() {
        Err(FailReason::ConfigWrite { message }) => {
            assert!(message.contains("假错误"), "{message}")
        }
        other => panic!("{other:?}"),
    }
}
