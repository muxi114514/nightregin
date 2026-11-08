//! launch_run 测试的后半段

use super::*;

#[test]
fn runner_av_code_without_exit_crash_dump_is_abnormal() {
    // 没有新的转储；启动前就有的已知崩溃转储不算这次的
    let tmp = TempDir::new().unwrap();
    write_dump(
        tmp.path(),
        "old_exit.dmp",
        &exit_crash(),
        epoch() - secs(3600.0),
    );
    let os = FakeOs::new(game_exits_with(AV));
    assert_eq!(
        start(
            LaunchJob::default(),
            &FakeEnv::with_dumps(tmp.path()),
            &os,
            never
        ),
        happy(true)
    );
}

#[test]
fn runner_dump_before_spawn_is_ignored() {
    // 点开始后、唤起前（备份那 30 秒里）写下的转储，不算这次游戏的
    let tmp = TempDir::new().unwrap();
    dump(tmp.path(), "before_spawn.dmp", epoch() + secs(15.0));
    let os = FakeOs::new(game_exits_with(0));
    let env = FakeEnv::with_dumps(tmp.path()).backup_taking(&os, secs(30.0));
    let (outcome, views) = start(LaunchJob::default(), &env, &os, never);
    let finished = LaunchView::Finished { abnormal: false };
    assert_eq!(outcome, LaunchOutcome::Finished { abnormal: false });
    // 剧本时刻是绝对虚拟时间：30 秒唤起时游戏已有窗口，首轮就进游戏，不发 LOADING
    assert_eq!(views, steps_then(4, [LaunchView::Running, finished]));
}

#[test]
fn game_vanishes_without_handle_fails_early() {
    // 打不开游戏进程的句柄，窗口出现前进程就不在了：退出码未知
    let script = Script {
        game_start: Some(secs(3.0)),
        game_exit: Some((secs(8.0), 1)),
        game_unopenable: true,
        ..Script::default()
    };
    let os = FakeOs::new(script.clone());
    let got = start(LaunchJob::default(), &FakeEnv::new(), &os, never);
    let unknown = FailReason::GameExitedEarly { code: None };
    assert_eq!(got, failed_at(Step::Wake, unknown, [LOADING]));
    // 打不开也只试一次，不每轮重开；8 秒退出后连续第 3 轮（9 秒）找不到才算不在
    assert_eq!(os.opens(), 1);
    assert_eq!(os.elapsed(), secs(9.0));

    // 对照：句柄打得开时同样提前失败，并带上读到的退出码
    let os = FakeOs::new(Script {
        game_unopenable: false,
        ..script
    });
    let got = start(LaunchJob::default(), &FakeEnv::new(), &os, never);
    let exited = FailReason::GameExitedEarly { code: Some(1) };
    assert_eq!(got, failed_at(Step::Wake, exited, [LOADING]));
}

#[test]
fn runner_running_ends_when_unopenable_game_vanishes() {
    // 进游戏后进程不在了：没有句柄读不到退出码（剧本里的 1 不算数），也没有新转储，算正常
    let tmp = TempDir::new().unwrap();
    let os = FakeOs::new(Script {
        game_unopenable: true,
        ..game_exits_with(1)
    });
    let got = start(
        LaunchJob::default(),
        &FakeEnv::with_dumps(tmp.path()),
        &os,
        never,
    );
    assert_eq!(got, happy(false));
    assert_eq!(os.opens(), 1);
    // 60 秒退出后连续 3 轮（60、60.5、61 秒）找不到才算不在
    assert_eq!(os.elapsed(), secs(61.0));

    // 对照：同样读不到退出码，但唤起后有新转储（空文件读不出异常信息，按真崩溃算），就算异常
    dump(tmp.path(), "nightreign.dmp", epoch() + secs(30.0));
    let os = FakeOs::new(Script {
        game_unopenable: true,
        ..game_exits_with(1)
    });
    let got = start(
        LaunchJob::default(),
        &FakeEnv::with_dumps(tmp.path()),
        &os,
        never,
    );
    assert_eq!(got, happy(true));
}

#[test]
fn runner_snapshot_miss_does_not_end_run() {
    // 打不开句柄时只能看进程快照。快照偶尔失败：5 秒（载入中）一次；20、20.5 秒连续两次，
    // 21 秒找到、重新计数，21.5 秒又一次。都不算游戏退出
    let misses = [5.0, 20.0, 20.5, 21.5].map(secs).to_vec();
    let script = Script {
        game_unopenable: true,
        snapshot_misses: misses,
        ..game_exits_with(0)
    };
    let os = FakeOs::new(script);
    assert_eq!(
        start(LaunchJob::default(), &FakeEnv::new(), &os, never),
        happy(false)
    );
    // 一直等到 60 秒真正退出后的连续第 3 轮（61 秒）
    assert_eq!(os.elapsed(), secs(61.0));
}

#[test]
fn runner_cancel_while_running_kills() {
    let os = FakeOs::new(game_exits_with(0));
    let at_running = |view: &LaunchView| *view == LaunchView::Running;
    let got = start(LaunchJob::default(), &FakeEnv::new(), &os, at_running);
    assert_eq!(got, cancelled_after(4, [LOADING, LaunchView::Running]));
    assert_eq!(os.killed(), [ME3_EXE, ME3_LAUNCHER_EXE, GAME_EXE]);
}

#[test]
fn runner_step_failures() {
    // 失败记在出错的那一步：之后的步骤不再进行，也不唤起进程
    let issue = Issue {
        kind: IssueKind::GameVersionMismatch,
        blocking: true,
        detail: "1.02.3".into(),
    };
    let config = FailReason::ConfigWrite {
        message: "拒绝访问".into(),
    };
    let no_command = FailReason::SpawnFailed {
        message: "找不到 me3.exe".into(),
    };
    let cases = [
        (
            Step::Preflight,
            FakeEnv {
                preflight: Err(issue.clone()),
                ..FakeEnv::new()
            },
            FailReason::Preflight { issue },
        ),
        (
            Step::LoadConfig,
            FakeEnv {
                flush: Err(config.clone()),
                ..FakeEnv::new()
            },
            config,
        ),
        (
            Step::Wake,
            FakeEnv {
                command: Err(no_command.clone()),
                ..FakeEnv::new()
            },
            no_command,
        ),
    ];
    for (step, env, reason) in cases {
        let os = FakeOs::new(game_exits_with(0));
        assert_eq!(
            start(LaunchJob::default(), &env, &os, never),
            failed_at(step, reason, [])
        );
        assert!(os.spawned().is_empty());
    }

    // spawn 失败：记在 Wake 步，带上系统给的错误信息
    let script = Script {
        spawn_error: Some(io::ErrorKind::NotFound),
        ..game_exits_with(0)
    };
    let os = FakeOs::new(script);
    let message = io::Error::from(io::ErrorKind::NotFound).to_string();
    let got = start(LaunchJob::default(), &FakeEnv::new(), &os, never);
    assert_eq!(
        got,
        failed_at(Step::Wake, FailReason::SpawnFailed { message }, [])
    );
    assert_eq!(os.spawned(), [me3_command()]);

    // me3 非零退出、还没有游戏进程：也记在 Wake 步
    let os = FakeOs::new(Script {
        me3_exit: Some((secs(1.0), 3)),
        ..Script::default()
    });
    let got = start(LaunchJob::default(), &FakeEnv::new(), &os, never);
    assert_eq!(
        got,
        failed_at(Step::Wake, FailReason::Me3Exit { code: 3 }, [])
    );
}

#[test]
fn runner_me3_exit_zero_no_game_fails_after_15s() {
    // me3 在 1 秒以 0 退出、游戏进程一直不出现：每轮都要看到 me3 已退出，满 15 秒（16 秒）判失败
    let os = FakeOs::new(Script {
        me3_exit: Some((secs(1.0), 0)),
        ..Script::default()
    });
    let got = start(LaunchJob::default(), &FakeEnv::new(), &os, never);
    assert_eq!(got, failed_at(Step::Wake, FailReason::GameNeverStarted, []));
    assert_eq!(os.elapsed(), secs(16.0));
}

#[test]
fn runner_me3_exit_zero_then_game_runs() {
    // me3 在 1 秒以 0 退出，游戏随后照常出现、出窗口、以 0 退出：不判失败
    let os = FakeOs::new(Script {
        me3_exit: Some((secs(1.0), 0)),
        ..game_exits_with(0)
    });
    assert_eq!(
        start(LaunchJob::default(), &FakeEnv::new(), &os, never),
        happy(false)
    );
    assert_eq!(os.elapsed(), Duration::from_secs(60));
}
