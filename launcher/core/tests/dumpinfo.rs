//! 崩溃转储分类：退出时的已知崩溃、各种真崩溃、截断 / 垃圾 / 不是 minidump 的输入，以及按时间汇总

mod common;

use std::io::Cursor;
use std::time::{Duration, SystemTime};

use common::minidump::{
    AV, EXECUTE, EXIT_READ_AT, Fault, GAME_BASE, MODULES, NTDLL_BASE, READ, WRITE,
    exec_av_no_module, exit_crash, game_read_av, minidump, minidump_in, ntdll_heap_corruption,
    with_params, without_module_list, write_dump,
};
use launcher_core::dumpinfo::{DumpError, DumpInfo, DumpKind, DumpSummary, FaultModule, classify};
use launcher_core::launch::abnormal_since;
use tempfile::TempDir;

fn parse(bytes: &[u8]) -> Result<DumpInfo, DumpError> {
    DumpInfo::parse(&mut Cursor::new(bytes))
}

/// 解析成功时的分类；解析失败也是真崩溃
fn kind(bytes: &[u8]) -> DumpKind {
    parse(bytes).map_or(DumpKind::RealCrash, |info| info.kind())
}

#[test]
fn exit_crash_is_classified() {
    let info = parse(&exit_crash()).unwrap();
    let ntdll = FaultModule {
        name: "ntdll.dll".into(),
        base: NTDLL_BASE,
        size: MODULES[1].2,
    };
    let want = DumpInfo {
        code: AV,
        flags: 0,
        address: NTDLL_BASE + 0x1cc5d,
        access_kind: Some(READ),
        access_address: Some(EXIT_READ_AT),
        module: Some(ntdll),
    };
    assert_eq!(info, want);
    assert_eq!(info.kind(), DumpKind::ExitCrash);
    // 模块名不分大小写；偏移不看（系统更新会变）
    let upper = [
        MODULES[0],
        (r"C:\Windows\System32\NTDLL.DLL", NTDLL_BASE, 0x0021_8000),
    ];
    let moved = Fault {
        code: AV,
        access: READ,
        address: NTDLL_BASE + 0x9_9999,
    };
    assert_eq!(kind(&minidump_in(moved, &upper)), DumpKind::ExitCrash);
}

#[test]
fn real_crash_shapes() {
    let game = parse(&game_read_av()).unwrap();
    assert_eq!(
        game.module.as_ref().map(|m| m.name.as_str()),
        Some("nightreign.exe")
    );
    assert_eq!(game.kind(), DumpKind::RealCrash, "游戏里的读访问违例");

    let exec = parse(&exec_av_no_module()).unwrap();
    assert_eq!(
        (exec.module.clone(), exec.access_kind),
        (None, Some(EXECUTE))
    );
    assert_eq!(exec.kind(), DumpKind::RealCrash, "不在任何模块里的执行违例");

    let heap = parse(&ntdll_heap_corruption()).unwrap();
    assert_eq!(
        heap.module.as_ref().map(|m| m.name.as_str()),
        Some("ntdll.dll")
    );
    assert_eq!(heap.kind(), DumpKind::RealCrash, "ntdll 里不是访问违例");

    // ntdll 里的写 / 执行违例、只是名字像 ntdll 的模块：都不算
    for access in [WRITE, EXECUTE] {
        let fault = Fault {
            code: AV,
            access,
            address: NTDLL_BASE + 0x1cc5d,
        };
        assert_eq!(
            kind(&minidump(fault)),
            DumpKind::RealCrash,
            "访问类型 {access}"
        );
    }
    let lookalike = [
        MODULES[0],
        (r"C:\mods\my_ntdll.dll", NTDLL_BASE, 0x0021_8000),
    ];
    let fault = Fault {
        code: AV,
        access: READ,
        address: NTDLL_BASE + 0x1cc5d,
    };
    assert_eq!(kind(&minidump_in(fault, &lookalike)), DumpKind::RealCrash);
}

/// 读得出来但认不出模块（没有模块列表）或访问类型（参数个数为 0）：算真崩溃
#[test]
fn missing_module_list_or_params_is_real_crash() {
    let info = parse(&without_module_list(exit_crash())).unwrap();
    assert_eq!(
        (info.code, info.access_kind, info.module.clone()),
        (AV, Some(READ), None)
    );
    assert_eq!(info.kind(), DumpKind::RealCrash, "没有模块列表");

    let info = parse(&with_params(exit_crash(), 0)).unwrap();
    assert_eq!((info.access_kind, info.access_address), (None, None));
    assert_eq!(
        info.module.as_ref().map(|m| m.name.as_str()),
        Some("ntdll.dll")
    );
    assert_eq!(info.kind(), DumpKind::RealCrash, "参数个数为 0");
}

/// 截断、垃圾、不是 minidump：都读不出来，算真崩溃，且不 panic
#[test]
fn truncated_garbage_and_non_mdmp_are_real_crashes() {
    let full = exit_crash();
    for len in 0..full.len() {
        assert!(parse(&full[..len]).is_err(), "截到 {len} 字节");
    }

    let mut not_mdmp = full.clone();
    not_mdmp[..4].copy_from_slice(b"MZ\x90\x00");
    assert!(matches!(parse(&not_mdmp), Err(DumpError::NotMinidump)));

    // 大数和越界：流的个数、目录位置、模块列表长度、异常流长度和位置、模块个数、ntdll 的模块名位置和长度。
    // 合成转储的布局：目录在 32，第 2、3 项是模块列表和异常流；模块列表在 240，第 2 个模块是 ntdll
    let ntdll_name_rva = 240 + 4 + 108 + 20;
    let name_at = u32::from_le_bytes(full[ntdll_name_rva..ntdll_name_rva + 4].try_into().unwrap());
    let patches = [
        (8, u32::MAX),
        (12, u32::MAX - 3),
        (48, 8),
        (60, 8),
        (64, u32::MAX),
        (240, u32::MAX),
        (ntdll_name_rva, u32::MAX),
        (name_at as usize, u32::MAX),
    ];
    for (at, v) in patches {
        let mut bad = full.clone();
        bad[at..at + 4].copy_from_slice(&v.to_le_bytes());
        assert!(parse(&bad).is_err(), "偏移 {at} 改成 {v:#x}");
    }

    // 伪随机字节，带不带 MDMP 头都试
    let mut seed = 0x2545_F491_4F6C_DD1Du64;
    for len in [1, 4, 16, 64, 1000, 5000] {
        let mut junk: Vec<u8> = (0..len)
            .map(|_| {
                seed ^= seed << 13;
                seed ^= seed >> 7;
                seed ^= seed << 17;
                seed as u8
            })
            .collect();
        assert_eq!(kind(&junk), DumpKind::RealCrash, "{len} 字节垃圾");
        if len >= 4 {
            junk[..4].copy_from_slice(b"MDMP");
            assert_eq!(
                kind(&junk),
                DumpKind::RealCrash,
                "MDMP 头 + {} 字节垃圾",
                len - 4
            );
        }
    }

    // 逐字节改坏一个有效转储：结果随位置不同，但都不能 panic
    for at in 0..full.len() {
        let mut bad = full.clone();
        bad[at] ^= 0xFF;
        let _ = parse(&bad);
    }
}

#[test]
fn unreadable_file_is_real_crash() {
    let tmp = TempDir::new().unwrap();
    assert_eq!(
        classify(&tmp.path().join("missing.dmp")),
        DumpKind::RealCrash
    );
    write_dump(tmp.path(), "empty.dmp", &[], SystemTime::now());
    assert_eq!(classify(&tmp.path().join("empty.dmp")), DumpKind::RealCrash);
    write_dump(tmp.path(), "exit.dmp", &exit_crash(), SystemTime::now());
    assert_eq!(classify(&tmp.path().join("exit.dmp")), DumpKind::ExitCrash);
}

/// 启动时补查：只有退出时的已知崩溃不算异常；since 之前的转储不算
#[test]
fn abnormal_since_ignores_exit_crashes() {
    let tmp = TempDir::new().unwrap();
    let since = SystemTime::UNIX_EPOCH + Duration::from_secs(1_793_000_000);
    let after = since + Duration::from_secs(60);
    write_dump(tmp.path(), "exit1.dmp", &exit_crash(), after);
    write_dump(tmp.path(), "exit2.dmp", &exit_crash(), after);
    write_dump(
        tmp.path(),
        "old_real.dmp",
        &game_read_av(),
        since - Duration::from_secs(3600),
    );
    assert!(!abnormal_since(Some(tmp.path()), since));
    let summary = DumpSummary {
        exit_crashes: 2,
        real_crashes: 0,
    };
    assert_eq!(DumpSummary::since(Some(tmp.path()), since), summary);

    write_dump(tmp.path(), "real.dmp", &exec_av_no_module(), after);
    assert!(abnormal_since(Some(tmp.path()), since));
    let summary = DumpSummary {
        exit_crashes: 2,
        real_crashes: 1,
    };
    assert_eq!(DumpSummary::since(Some(tmp.path()), since), summary);
    assert_eq!(DumpSummary::since(None, since), DumpSummary::default());
}

#[test]
fn game_module_is_found_by_address() {
    // 两个模块都在列表里时按地址区间找，不是取第一个或最后一个
    let info = parse(&game_read_av()).unwrap();
    assert_eq!(
        info.module.map(|m| (m.base, m.size)),
        Some((GAME_BASE, MODULES[0].2))
    );
}
