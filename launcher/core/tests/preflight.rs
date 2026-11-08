//! 首页问题（health_issues）、预检（preflight）与首页状态（home_state），都在临时目录的假整合包上验证

mod common;

use std::env;
use std::fs;
use std::os::windows::fs::OpenOptionsExt;
use std::path::PathBuf;
use std::sync::Arc;

use common::fake_os::{FakeOs, RUNNING_PID, Script};
use common::{FakePack, fake_pack};
use launcher_core::gamever::{GameVersion, dotted, read_exe_version};
use launcher_core::launch::LaunchJob;
use launcher_core::os::{GAME_EXE, STEAM_EXE};
use launcher_core::paths::{GameDir, PackLayout};
use launcher_core::preflight::IssueKind::{
    ChanceSetterSuffix, EverdarkMismatch, GameAlreadyRunning, GameNotFound, GameVersionMismatch,
    LastExitAbnormal, Me3FilesMissing, PackNotFound, SaveBusy, SteamNotRunning, SuffixMismatch,
};
use launcher_core::preflight::{
    DeepNight, HomeState, Issue, IssueKind, PackState, health_issues, home_state, preflight,
    suffix_check,
};
use launcher_core::settings::Settings;
use launcher_core::suffix::SuffixCheck;
use serde_json::json;

/// manifest 里写的游戏版本
const V133: GameVersion = GameVersion {
    file_version: [1, 3, 3, 0],
    size: 118835320,
};
/// 与 .me3 的 co2 不一致的无缝联机配置
const NRSC_SL2: &str = "[SAVE]\r\nsave_file_extension = sl2\r\n";

/// 读假整合包的现状，不问 Steam
fn load(fp: &FakePack, settings: Settings) -> PackState {
    PackState::load(
        PackLayout {
            root: fp.pack_root.clone(),
        },
        settings,
        &|| None,
    )
}

fn kinds(issues: &[Issue]) -> Vec<IssueKind> {
    issues.iter().map(|issue| issue.kind).collect()
}

fn issue(kind: IssueKind, blocking: bool, detail: &str) -> Issue {
    Issue {
        kind,
        blocking,
        detail: detail.to_owned(),
    }
}

/// 只有 Steam 在运行
fn steam_running() -> Arc<FakeOs> {
    FakeOs::new(Script {
        already_running: vec![STEAM_EXE],
        ..Script::default()
    })
}

/// Steam 在运行、游戏没在运行时，按默认选项预检
fn check(s: &PackState) -> Result<(), Issue> {
    preflight(s, &*steam_running(), &LaunchJob::default())
}

/// 开发模式下勾了「仍然启动」
fn ignore_version() -> LaunchJob {
    LaunchJob {
        ignore_version: true,
        ..LaunchJob::default()
    }
}

fn write(fp: &FakePack, rel: &str, text: &str) {
    fs::write(fp.pack_root.join(rel), text).unwrap();
}

fn remove(fp: &FakePack, rel: &str) {
    fs::remove_file(fp.pack_root.join(rel)).unwrap();
}

/// manifest.json：整合包版本 1.9.5，适配 game 里的游戏版本
fn write_manifest(fp: &FakePack, game: &[GameVersion]) {
    write(
        fp,
        "manifest.json",
        &json!({ "version": "1.9.5", "game": game }).to_string(),
    );
}

/// 假游戏 exe 是空文件、读不出版本：换成带版本资源的 kernel32.dll，交出它的版本
fn versioned_exe(fp: &FakePack) -> GameVersion {
    let exe = fp.game_root.join(r"Game\nightreign.exe");
    let system = PathBuf::from(env::var_os("SystemRoot").unwrap()).join(r"System32\kernel32.dll");
    fs::copy(system, &exe).unwrap();
    read_exe_version(&exe).unwrap()
}

#[test]
fn health_orders_issues_by_priority() {
    // 找不到游戏，且 ChanceSetter 后缀不一致（真实 fixture 的 .CL_SAVE）
    let fp = fake_pack("game");
    fs::remove_file(fp.game_root.join(r"Game\nightreign.exe")).unwrap();
    let s = load(&fp, Settings::default());
    assert_eq!(
        kinds(&health_issues(&s)),
        [GameNotFound, ChanceSetterSuffix]
    );

    // 能同时出现的五项（找到了游戏才查版本）：按枚举的声明顺序排
    let fp = fake_pack("game");
    write_manifest(&fp, &[V133]);
    remove(&fp, r"bin\me3.exe");
    write(&fp, r"SeamlessCoop\nrsc_settings.ini", NRSC_SL2);
    let s = load(
        &fp,
        Settings {
            last_exit_abnormal: true,
            ..Settings::default()
        },
    );
    let issues = health_issues(&s);
    let all = [
        GameVersionMismatch,
        Me3FilesMissing,
        SuffixMismatch,
        ChanceSetterSuffix,
        LastExitAbnormal,
    ];
    assert_eq!(kinds(&issues), all);
    let blocking: Vec<bool> = issues.iter().map(|issue| issue.blocking).collect();
    assert_eq!(blocking, [true, true, true, false, false]);
}

#[test]
fn health_warns_chance_setter_suffix() {
    // 真实整合包：me3 与 nrsc 都是 co2，ChanceSetter 用 .CL_SAVE
    let fp = fake_pack("game");
    let s = load(&fp, Settings::default());
    let want = SuffixCheck {
        me3: Some("co2".into()),
        nrsc: "co2".into(),
        chance_setter: Some(".CL_SAVE".into()),
    };
    assert_eq!(suffix_check(&s), want);
    // 只警告不阻断，detail 是 ChanceSetter 的后缀原文
    assert_eq!(
        health_issues(&s),
        [issue(ChanceSetterSuffix, false, ".CL_SAVE")]
    );
    assert_eq!(check(&s), Ok(()));
    // ChanceSetter.ini 不在：无从比较，不警告
    remove(&fp, r"dll\ChanceSetter.ini");
    assert_eq!(
        health_issues(&load(&fp, Settings::default())),
        Vec::<Issue>::new()
    );
}

#[test]
fn health_reports_missing_me3_files() {
    let fp = fake_pack("game");
    remove(&fp, r"bin\me3_mod_host.dll");
    remove(&fp, r"SeamlessCoop\nrsc.dll");
    remove(&fp, r"dll\ChanceSetter.dll");
    fs::remove_dir(fp.pack_root.join("mod")).unwrap();
    let s = load(&fp, Settings::default());
    // me3 自己的文件在前，再按 .me3 的顺序；路径相对整合包根目录，只列前 3 个（第 4 个 mod 不列）
    let detail = r"bin\me3_mod_host.dll、SeamlessCoop\nrsc.dll、dll\ChanceSetter.dll";
    assert_eq!(
        health_issues(&s).first(),
        Some(&issue(Me3FilesMissing, true, detail))
    );
    assert_eq!(check(&s).map_err(|i| i.kind), Err(Me3FilesMissing));

    // 只缺一个 package（目录）
    let fp = fake_pack("game");
    fs::remove_dir(fp.pack_root.join("mod")).unwrap();
    let issues = health_issues(&load(&fp, Settings::default()));
    assert_eq!(issues.first(), Some(&issue(Me3FilesMissing, true, "mod")));

    // 只缺 bin\me3-launcher.exe
    let fp = fake_pack("game");
    remove(&fp, r"bin\me3-launcher.exe");
    let issues = health_issues(&load(&fp, Settings::default()));
    let launcher = issue(Me3FilesMissing, true, r"bin\me3-launcher.exe");
    assert_eq!(issues.first(), Some(&launcher));

    // .me3 里写的绝对路径不在整合包下：列完整路径
    let fp = fake_pack("game");
    let outside = fp.tmp.path().join(r"别处 目录\extra.dll");
    let native = format!("[[natives]]\npath = '{}'\n", outside.display());
    write(
        &fp,
        "褪色燃烧.me3",
        &format!("savefile = \"NR0000.co2\"\n{native}"),
    );
    let issues = health_issues(&load(&fp, Settings::default()));
    let full = outside.display().to_string();
    assert_eq!(issues.first(), Some(&issue(Me3FilesMissing, true, &full)));
}

#[test]
fn health_reports_unparsable_profile() {
    let fp = fake_pack("game");
    // 无缝联机改成 sl2：.me3 读得出来时这是阻断问题
    write(&fp, r"SeamlessCoop\nrsc_settings.ini", NRSC_SL2);
    write(&fp, "褪色燃烧.me3", "savefile = 1\n");
    let s = load(&fp, Settings::default());
    assert_eq!(
        s.profile.as_ref().err().map(String::as_str),
        Some("格式错误：savefile 应为字符串")
    );
    // 算 me3 文件缺失，detail 带上解析错误；me3 一方的后缀未知，不报两项后缀问题
    let broken = issue(
        Me3FilesMissing,
        true,
        "褪色燃烧.me3 格式错误：savefile 应为字符串",
    );
    assert_eq!(health_issues(&s), std::slice::from_ref(&broken));
    assert_eq!(suffix_check(&s).me3, None);
    assert_eq!(check(&s), Err(broken));
    // me3 自己的文件同时缺失：一并列出
    remove(&fp, r"bin\me3.exe");
    let both = r"bin\me3.exe、褪色燃烧.me3 格式错误：savefile 应为字符串";
    let s = load(&fp, Settings::default());
    assert_eq!(health_issues(&s), [issue(Me3FilesMissing, true, both)]);
    // bin 下三个文件全缺：只列得下 3 项，解析错误仍占最后一个名额，不会被截掉
    remove(&fp, r"bin\me3-launcher.exe");
    remove(&fp, r"bin\me3_mod_host.dll");
    let capped = r"bin\me3.exe、bin\me3-launcher.exe、褪色燃烧.me3 格式错误：savefile 应为字符串";
    let s = load(&fp, Settings::default());
    assert_eq!(health_issues(&s), [issue(Me3FilesMissing, true, capped)]);
}

#[test]
fn empty_suffix_reads_as_no_suffix() {
    // savefile 没有扩展名：.me3 一方的后缀是空串，横带上写「（无后缀）」；其余后缀照旧用原文
    let fp = fake_pack("game");
    write(&fp, "褪色燃烧.me3", "savefile = \"NR0000\"\n");
    let want = [
        issue(
            SuffixMismatch,
            true,
            "褪色燃烧.me3 是 （无后缀），无缝联机是 co2",
        ),
        issue(ChanceSetterSuffix, false, ".CL_SAVE"),
    ];
    assert_eq!(health_issues(&load(&fp, Settings::default())), want);
    // 无缝联机、ChanceSetter 一方为空时同样
    let fp = fake_pack("game");
    write(
        &fp,
        r"SeamlessCoop\nrsc_settings.ini",
        "[SAVE]\nsave_file_extension =\n",
    );
    write(
        &fp,
        r"dll\ChanceSetter.ini",
        "[save]\npath = D:\\x\\NR0000\n",
    );
    let want = [
        issue(
            SuffixMismatch,
            true,
            "褪色燃烧.me3 是 co2，无缝联机是 （无后缀）",
        ),
        issue(ChanceSetterSuffix, false, "（无后缀）"),
    ];
    assert_eq!(health_issues(&load(&fp, Settings::default())), want);
}

#[test]
fn last_exit_abnormal_flag_becomes_issue() {
    let fp = fake_pack("game");
    let s = load(
        &fp,
        Settings {
            last_exit_abnormal: true,
            ..Settings::default()
        },
    );
    assert_eq!(
        health_issues(&s).last(),
        Some(&issue(LastExitAbnormal, false, ""))
    );
    // 不阻断开始游戏
    assert_eq!(check(&s), Ok(()));
    let cleared = health_issues(&load(&fp, Settings::default()));
    assert!(!kinds(&cleared).contains(&LastExitAbnormal));
}

#[test]
fn preflight_blocks_when_steam_not_running() {
    let fp = fake_pack("game");
    let s = load(&fp, Settings::default());
    let job = LaunchJob::default();
    let no_steam = Err(issue(SteamNotRunning, true, ""));
    assert_eq!(
        preflight(&s, &*FakeOs::new(Script::default()), &job),
        no_steam
    );
    // 最先查 Steam：游戏也在运行时仍报 Steam
    let game_only = FakeOs::new(Script {
        already_running: vec![GAME_EXE],
        ..Script::default()
    });
    assert_eq!(preflight(&s, &*game_only, &job), no_steam);
    assert_eq!(preflight(&s, &*steam_running(), &job), Ok(()));
}

#[test]
fn preflight_blocks_when_game_running() {
    let fp = fake_pack("game");
    // 整合包也有阻断问题
    remove(&fp, r"bin\me3.exe");
    let s = load(&fp, Settings::default());
    let both = vec![STEAM_EXE, GAME_EXE];
    let both = FakeOs::new(Script {
        already_running: both,
        ..Script::default()
    });
    let running = issue(GameAlreadyRunning, true, &format!("pid {RUNNING_PID}"));
    assert_eq!(preflight(&s, &*both, &LaunchJob::default()), Err(running));
    // 游戏退出后才轮到整合包的问题
    assert_eq!(check(&s).map_err(|i| i.kind), Err(Me3FilesMissing));
}

#[test]
fn preflight_blocks_on_me3_vs_nrsc_suffix() {
    let fp = fake_pack("game");
    write(&fp, r"SeamlessCoop\nrsc_settings.ini", NRSC_SL2);
    let s = load(&fp, Settings::default());
    let mismatch = issue(SuffixMismatch, true, "褪色燃烧.me3 是 co2，无缝联机是 sl2");
    assert_eq!(check(&s), Err(mismatch));
    // 开发模式下的「仍然启动」只放过游戏版本
    let dev = load(
        &fp,
        Settings {
            dev_mode: true,
            ..Settings::default()
        },
    );
    let got = preflight(&dev, &*steam_running(), &ignore_version()).map_err(|i| i.kind);
    assert_eq!(got, Err(SuffixMismatch));
    // nrsc_settings.ini 不在：无缝联机用默认的 co2，与 .me3 一致
    remove(&fp, r"SeamlessCoop\nrsc_settings.ini");
    assert_eq!(check(&load(&fp, Settings::default())), Ok(()));
}

#[test]
fn preflight_version_mismatch_unless_dev_override() {
    let fp = fake_pack("game");
    let actual = versioned_exe(&fp);
    let older = GameVersion {
        file_version: [1, 2, 0, 0],
        size: 1,
    };
    write_manifest(&fp, &[V133, older]);
    let shown = format!("{} / {}", dotted(actual.file_version), actual.size);
    let detail = format!("实际 {shown}，需要 1.3.3.0 / 118835320、1.2.0.0 / 1");
    let mismatch = issue(GameVersionMismatch, true, &detail);
    let os = steam_running();
    let plain = LaunchJob::default();
    let dev_mode = || Settings {
        dev_mode: true,
        ..Settings::default()
    };
    let s = load(&fp, Settings::default());
    assert_eq!(preflight(&s, &*os, &plain), Err(mismatch.clone()));
    // 「仍然启动」要在开发模式下才生效
    assert_eq!(
        preflight(&s, &*os, &ignore_version()),
        Err(mismatch.clone())
    );
    let dev = load(&fp, dev_mode());
    assert_eq!(preflight(&dev, &*os, &plain), Err(mismatch.clone()));
    assert_eq!(preflight(&dev, &*os, &ignore_version()), Ok(()));
    // 首页照样提示；放过版本后，后面的阻断问题照样拦下
    assert_eq!(health_issues(&dev).first(), Some(&mismatch));
    remove(&fp, r"bin\me3.exe");
    let got = preflight(&load(&fp, dev_mode()), &*os, &ignore_version()).map_err(|i| i.kind);
    assert_eq!(got, Err(Me3FilesMissing));

    // manifest 列了实际版本就相符；版本号相同、大小不同仍不符
    let fp = fake_pack("game");
    let actual = versioned_exe(&fp);
    write_manifest(&fp, &[V133, actual]);
    assert_eq!(check(&load(&fp, Settings::default())), Ok(()));
    write_manifest(
        &fp,
        &[GameVersion {
            size: actual.size + 1,
            ..actual
        }],
    );
    let got = check(&load(&fp, Settings::default())).map_err(|i| i.kind);
    assert_eq!(got, Err(GameVersionMismatch));
    // 大小相同、版本号不同也不符：两样都要比
    write_manifest(
        &fp,
        &[GameVersion {
            file_version: [9, 9, 9, 9],
            ..actual
        }],
    );
    let got = check(&load(&fp, Settings::default())).map_err(|i| i.kind);
    assert_eq!(got, Err(GameVersionMismatch));
}

#[test]
fn unreadable_exe_version_is_mismatch() {
    // 假游戏 exe 是空文件，没有版本资源
    let fp = fake_pack("game");
    write_manifest(&fp, &[V133]);
    let why = read_exe_version(&fp.game_root.join(r"Game\nightreign.exe")).unwrap_err();
    let detail = format!("读不出实际版本：{why}，需要 1.3.3.0 / 118835320");
    let mismatch = issue(GameVersionMismatch, true, &detail);
    assert_eq!(check(&load(&fp, Settings::default())), Err(mismatch));
    // 开发模式下的「仍然启动」同样放行
    let dev = load(
        &fp,
        Settings {
            dev_mode: true,
            ..Settings::default()
        },
    );
    assert_eq!(
        preflight(&dev, &*steam_running(), &ignore_version()),
        Ok(())
    );
}

#[path = "preflight/more.rs"]
mod more;
