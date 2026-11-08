//! preflight 测试的后半段

use super::*;

#[test]
fn preflight_skips_version_check_without_manifest() {
    // 假游戏 exe 读不出版本，但没有 manifest 就不查
    let fp = fake_pack("game");
    let s = load(&fp, Settings::default());
    assert!(s.manifest.is_none());
    assert_eq!(check(&s), Ok(()));
    // manifest 没列游戏版本，或读不出来：同样不查
    for text in [
        r#"{"version":"1.9.5"}"#,
        r#"{"version":"1.9.5","game":[]}"#,
        "{",
    ] {
        write(&fp, "manifest.json", text);
        assert_eq!(check(&load(&fp, Settings::default())), Ok(()), "{text}");
    }
}

#[test]
fn pack_state_uses_chosen_dir_then_steam() {
    // 整合包不在游戏目录下：先认玩家手选的目录，再问 Steam
    let fp = fake_pack("game");
    let elsewhere = fp.tmp.path().join("Steam Library 游戏");
    fs::create_dir_all(elsewhere.join("Game")).unwrap();
    let exe = r"Game\nightreign.exe";
    fs::rename(fp.game_root.join(exe), elsewhere.join(exe)).unwrap();
    let pack = || PackLayout {
        root: fp.pack_root.clone(),
    };
    let want = Some(GameDir {
        root: elsewhere.clone(),
    });
    let chosen = Settings {
        game_dir: Some(elsewhere.clone()),
        ..Settings::default()
    };
    assert_eq!(PackState::load(pack(), chosen, &|| None).game, want);
    let steam = || Some(elsewhere.clone());
    assert_eq!(
        PackState::load(pack(), Settings::default(), &steam).game,
        want
    );
    assert_eq!(
        PackState::load(pack(), Settings::default(), &|| None).game,
        None
    );
}

#[test]
fn home_state_version_and_deep_night() {
    let fp = fake_pack("game");
    let home = home_state(&load(&fp, Settings::default()));
    assert_eq!(home.version, None);
    assert_eq!(
        home.deep_night,
        Some(DeepNight {
            enabled: true,
            level: 5
        })
    );

    // 版本取 manifest；深夜开关取 forceDeepNight.enable，不看 forceUnlockDeepNight
    write_manifest(&fp, &[]);
    let nighter = r#"{
        "forceUnlockDeepNight": true,
        "forceDeepNight": { "enable": false, "level": 3 },
        "superNightLordList": [0, 1, 2, 3, 4, 5, 6]
    }"#;
    write(&fp, r"DepthChange\nighter.json", nighter);
    let s = load(
        &fp,
        Settings {
            low_effects: true,
            ..Settings::default()
        },
    );
    let want = HomeState {
        version: Some("1.9.5".into()),
        deep_night: Some(DeepNight {
            enabled: false,
            level: 3,
        }),
        issues: vec![issue(ChanceSetterSuffix, false, ".CL_SAVE")],
        low_effects: true,
        dev_mode: false,
    };
    assert_eq!(home_state(&s), want);
    let dev = home_state(&load(
        &fp,
        Settings {
            dev_mode: true,
            ..Settings::default()
        },
    ));
    assert_eq!((dev.low_effects, dev.dev_mode), (false, true));

    // nighter.json 读不出来或不在：没有深夜状态
    write(&fp, r"DepthChange\nighter.json", "{");
    assert_eq!(home_state(&load(&fp, Settings::default())).deep_night, None);
    remove(&fp, r"DepthChange\nighter.json");
    assert_eq!(home_state(&load(&fp, Settings::default())).deep_night, None);
}

#[test]
fn home_state_json_shape() {
    // 前端的 HomeState / Issue 类型按这个形状取字段
    let fp = fake_pack("game");
    let home = home_state(&load(&fp, Settings::default()));
    let want = json!({
        "version": null,
        "deepNight": { "enabled": true, "level": 5 },
        "issues": [{ "kind": "chanceSetterSuffix", "blocking": false, "detail": ".CL_SAVE" }],
        "lowEffects": false,
        "devMode": false,
    });
    assert_eq!(serde_json::to_value(&home).unwrap(), want);

    // IssueKind 的全部取值；声明顺序就是首页横带的优先级
    let all = [
        PackNotFound,
        GameNotFound,
        GameVersionMismatch,
        Me3FilesMissing,
        SaveBusy,
        SuffixMismatch,
        ChanceSetterSuffix,
        LastExitAbnormal,
        SteamNotRunning,
        GameAlreadyRunning,
    ];
    assert!(all.is_sorted());
    let names = json!([
        "packNotFound",
        "gameNotFound",
        "gameVersionMismatch",
        "me3FilesMissing",
        "saveBusy",
        "suffixMismatch",
        "chanceSetterSuffix",
        "lastExitAbnormal",
        "steamNotRunning",
        "gameAlreadyRunning",
    ]);
    assert_eq!(serde_json::to_value(all).unwrap(), names);
}

/// nighter 的永夜名单不是出厂值：只提示，不拦下，排在存档后缀之后
#[test]
fn health_lists_everdark_after_suffix() {
    let fp = fake_pack("game");
    write(
        &fp,
        r"launcher\defaults\nighter.json",
        r#"{"forceUnlockDeepNight":true,"forceDeepNight":{"enable":true,"level":5},"superNightLordList":[]}"#,
    );
    let issues = health_issues(&load(&fp, Settings::default()));
    let kinds: Vec<IssueKind> = issues.iter().map(|i| i.kind).collect();
    let (suffix, everdark) = (
        kinds.iter().position(|k| *k == ChanceSetterSuffix).unwrap(),
        kinds.iter().position(|k| *k == EverdarkMismatch).unwrap(),
    );
    assert!(suffix < everdark);
    let issue = issues.iter().find(|i| i.kind == EverdarkMismatch).unwrap();
    assert!(!issue.blocking);
}

#[test]
fn everdark_not_reported_when_nighter_unreadable() {
    let fp = fake_pack("game");
    write(&fp, r"DepthChange\nighter.json", "garbage");
    let issues = health_issues(&load(&fp, Settings::default()));
    assert!(issues.iter().all(|i| i.kind != EverdarkMismatch));
}

#[test]
fn everdark_not_reported_without_factory() {
    let fp = fake_pack("game");
    remove(&fp, r"launcher\defaults\nighter.json");
    write(
        &fp,
        r"DepthChange\nighter.json",
        r#"{"forceUnlockDeepNight":true,"forceDeepNight":{"enable":true,"level":5},"superNightLordList":[1]}"#,
    );
    let issues = health_issues(&load(&fp, Settings::default()));
    assert!(issues.iter().all(|i| i.kind != EverdarkMismatch));
}

#[test]
fn home_reports_save_busy() {
    let fp = fake_pack("game");
    let tmp = tempfile::TempDir::new().unwrap();
    let account: u64 = 76561198000000001;
    let dir = tmp.path().join(account.to_string());
    fs::create_dir_all(&dir).unwrap();
    let file = dir.join("NR0000.co2");
    fs::write(&file, b"x").unwrap();

    let state = load(&fp, Settings::default()).probe_save(Some(tmp.path()), Some(account));
    assert!(health_issues(&state).iter().all(|i| i.kind != SaveBusy));

    let held = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&file)
        .unwrap();
    let state = load(&fp, Settings::default()).probe_save(Some(tmp.path()), Some(account));
    assert!(health_issues(&state).iter().any(|i| i.kind == SaveBusy));
    drop(held);
}
