//! 启动流程：唤起阶段的判定（WakeState::decide）和给前端的 JSON 格式；run 的完整流程见 launch_run.rs

mod common;

use std::time::Instant;

use common::fake_os::secs;
use launcher_core::launch::{FailReason, LaunchView, Observation, Step, WakeDecision, WakeState};
use launcher_core::preflight::{Issue, IssueKind};
use serde::Serialize;

fn wait(slow: bool, game_loading: bool) -> WakeDecision {
    WakeDecision::Wait { slow, game_loading }
}

/// me3 还在运行，游戏进程 42 已在、还没有窗口
fn loading() -> Observation {
    Observation {
        game_pid: Some(42),
        ..Observation::default()
    }
}

/// me3 以 code 退出，还没有游戏进程
fn me3_exited(code: u32) -> Observation {
    Observation {
        me3_exit: Some(code),
        ..Observation::default()
    }
}

#[test]
fn window_appears_means_running() {
    let t0 = Instant::now();
    let mut wake = WakeState::new(t0);
    assert_eq!(wake.decide(&loading(), t0 + secs(3.0)), wait(false, true));
    let shown = Observation {
        game_window: true,
        ..loading()
    };
    assert_eq!(
        wake.decide(&shown, t0 + secs(10.0)),
        WakeDecision::Running { pid: 42 }
    );

    // 窗口可见最优先：me3 已非零退出也算进了游戏
    let mut wake = WakeState::new(t0);
    let me3_failed = Observation {
        me3_exit: Some(3),
        ..shown
    };
    assert_eq!(
        wake.decide(&me3_failed, t0 + secs(1.0)),
        WakeDecision::Running { pid: 42 }
    );
}

#[test]
fn me3_nonzero_exit_fails_immediately() {
    let t0 = Instant::now();
    let mut wake = WakeState::new(t0);
    let fail = WakeDecision::Fail(FailReason::Me3Exit { code: 3 });
    assert_eq!(wake.decide(&me3_exited(3), t0 + secs(0.5)), fail);

    // 已见过游戏进程：以游戏自己的结果为准，me3 的退出码不再算数
    let mut wake = WakeState::new(t0);
    assert_eq!(wake.decide(&loading(), t0 + secs(1.0)), wait(false, true));
    let both = Observation {
        me3_exit: Some(3),
        ..loading()
    };
    assert_eq!(wake.decide(&both, t0 + secs(1.5)), wait(false, true));
}

#[test]
fn me3_exit_zero_then_game_appears_ok() {
    let t0 = Instant::now();
    let mut wake = WakeState::new(t0);
    assert_eq!(
        wake.decide(&me3_exited(0), t0 + secs(1.0)),
        wait(false, false)
    );
    let game = Observation {
        me3_exit: Some(0),
        ..loading()
    };
    assert_eq!(wake.decide(&game, t0 + secs(10.0)), wait(false, true));
    // 游戏进程出现后不再按 me3 退出计时：过了 15 秒也不判失败
    assert_eq!(wake.decide(&game, t0 + secs(30.0)), wait(false, true));
    let shown = Observation {
        game_window: true,
        ..game
    };
    assert_eq!(
        wake.decide(&shown, t0 + secs(31.0)),
        WakeDecision::Running { pid: 42 }
    );
}

#[test]
fn me3_exit_zero_no_game_fails_after_15s() {
    let t0 = Instant::now();
    let mut wake = WakeState::new(t0);
    assert_eq!(
        wake.decide(&Observation::default(), t0 + secs(4.5)),
        wait(false, false)
    );
    // 5 秒时第一次看到 me3 退出，从这一刻起算；之后每轮都看到它退出，不重新计时
    let t1 = t0 + secs(5.0);
    assert_eq!(wake.decide(&me3_exited(0), t1), wait(false, false));
    assert_eq!(
        wake.decide(&me3_exited(0), t1 + secs(14.5)),
        wait(false, false)
    );
    let fail = WakeDecision::Fail(FailReason::GameNeverStarted);
    assert_eq!(wake.decide(&me3_exited(0), t1 + secs(15.0)), fail);
}

#[test]
fn game_exits_before_window_fails() {
    let t0 = Instant::now();
    let mut wake = WakeState::new(t0);
    assert_eq!(wake.decide(&loading(), t0 + secs(3.0)), wait(false, true));
    let exited = Observation {
        game_exit: Some(1),
        ..loading()
    };
    let fail = WakeDecision::Fail(FailReason::GameExitedEarly { code: Some(1) });
    assert_eq!(wake.decide(&exited, t0 + secs(3.5)), fail);

    // 没有句柄：见过的进程不在了就算退出，退出码未知
    let mut wake = WakeState::new(t0);
    assert_eq!(wake.decide(&loading(), t0 + secs(3.0)), wait(false, true));
    let fail = WakeDecision::Fail(FailReason::GameExitedEarly { code: None });
    assert_eq!(wake.decide(&Observation::default(), t0 + secs(3.5)), fail);
}

#[test]
fn slow_after_120s_without_process() {
    let t0 = Instant::now();
    let mut wake = WakeState::new(t0);
    let me3_running = Observation::default();
    assert_eq!(
        wake.decide(&me3_running, t0 + secs(119.5)),
        wait(false, false)
    );
    assert_eq!(
        wake.decide(&me3_running, t0 + secs(120.0)),
        wait(true, false)
    );
}

#[test]
fn slow_after_120s_with_process_no_window() {
    let t0 = Instant::now();
    let mut wake = WakeState::new(t0);
    assert_eq!(wake.decide(&loading(), t0 + secs(5.0)), wait(false, true));
    assert_eq!(wake.decide(&loading(), t0 + secs(119.5)), wait(false, true));
    assert_eq!(wake.decide(&loading(), t0 + secs(120.0)), wait(true, true));
}

fn json(value: &impl Serialize) -> String {
    serde_json::to_string(value).unwrap()
}

#[test]
fn launch_view_json_shape() {
    // 给前端的格式
    assert_eq!(
        serde_json::to_string(&LaunchView::Waking {
            slow: true,
            game_loading: false
        })
        .unwrap(),
        r#"{"phase":"waking","slow":true,"gameLoading":false}"#
    );
    assert_eq!(
        serde_json::to_string(&FailReason::Me3Exit { code: 3 }).unwrap(),
        r#"{"kind":"me3Exit","code":3}"#
    );
}

#[test]
fn every_variant_json_shape() {
    // 前端按 phase / kind 分支取字段：每个变体都钉住，改名或改 serde 属性都会被发现
    let failed = LaunchView::Failed {
        step: Step::Backup,
        reason: FailReason::SaveBusy,
    };
    let views = [
        (LaunchView::Idle, r#"{"phase":"idle"}"#),
        (
            LaunchView::Step {
                step: Step::Preflight,
            },
            r#"{"phase":"step","step":"preflight"}"#,
        ),
        (
            LaunchView::Step { step: Step::Backup },
            r#"{"phase":"step","step":"backup"}"#,
        ),
        (
            LaunchView::Step {
                step: Step::LoadConfig,
            },
            r#"{"phase":"step","step":"loadConfig"}"#,
        ),
        (
            LaunchView::Step { step: Step::Wake },
            r#"{"phase":"step","step":"wake"}"#,
        ),
        (
            LaunchView::Waking {
                slow: false,
                game_loading: true,
            },
            r#"{"phase":"waking","slow":false,"gameLoading":true}"#,
        ),
        (LaunchView::Running, r#"{"phase":"running"}"#),
        (
            failed,
            r#"{"phase":"failed","step":"backup","reason":{"kind":"saveBusy"}}"#,
        ),
        (LaunchView::Cancelled, r#"{"phase":"cancelled"}"#),
        (
            LaunchView::Finished { abnormal: true },
            r#"{"phase":"finished","abnormal":true}"#,
        ),
    ];
    for (view, want) in views {
        assert_eq!(json(&view), want);
    }

    let issue = Issue {
        kind: IssueKind::SaveBusy,
        blocking: true,
        detail: "NR0000.co2".into(),
    };
    let accounts = vec![
        "76561198000000001".to_owned(),
        "76561198000000002".to_owned(),
    ];
    let msg = || "拒绝访问".to_owned();
    let reasons = [
        (
            FailReason::Preflight { issue },
            concat!(
                r#"{"kind":"preflight","issue":"#,
                r#"{"kind":"saveBusy","blocking":true,"detail":"NR0000.co2"}}"#
            ),
        ),
        (FailReason::SaveBusy, r#"{"kind":"saveBusy"}"#),
        (
            FailReason::AccountUnknown { accounts },
            r#"{"kind":"accountUnknown","accounts":["76561198000000001","76561198000000002"]}"#,
        ),
        (
            FailReason::Backup { message: msg() },
            r#"{"kind":"backup","message":"拒绝访问"}"#,
        ),
        (
            FailReason::ConfigWrite { message: msg() },
            r#"{"kind":"configWrite","message":"拒绝访问"}"#,
        ),
        (
            FailReason::SpawnFailed { message: msg() },
            r#"{"kind":"spawnFailed","message":"拒绝访问"}"#,
        ),
        (
            FailReason::Me3Exit { code: 0xC000_0005 },
            r#"{"kind":"me3Exit","code":3221225477}"#,
        ),
        (
            FailReason::GameNeverStarted,
            r#"{"kind":"gameNeverStarted"}"#,
        ),
        (
            FailReason::GameExitedEarly { code: Some(1) },
            r#"{"kind":"gameExitedEarly","code":1}"#,
        ),
        (
            FailReason::GameExitedEarly { code: None },
            r#"{"kind":"gameExitedEarly","code":null}"#,
        ),
    ];
    for (reason, want) in reasons {
        assert_eq!(json(&reason), want);
    }
}
