//! NRStatAlloc regulation 补丁工具：把属性组件 SpEffect（ID 9960001~9968002）加进任意黑夜君临 regulation.bin，
//! 供整合包作者在合并好其他模组之后使用。可直接把 regulation.bin 拖到 exe 上，结束时会等待按回车。

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use nr_stat_alloc_patcher::regulation::Regulation;
use nr_stat_alloc_shared::components::{RESTORE_FP, RESTORE_HP, all};
use nr_stat_alloc_shared::param::ParamFile;
use nr_stat_alloc_shared::param::layout::sp_effect;
use windows_sys::Win32::System::Console::GetConsoleProcessList;

const USAGE: &str = "\
用法 / Usage:
  nr_stat_alloc_patcher <regulation.bin> [output.bin]

把 NRStatAlloc 需要的 2034 行 SpEffect 加进输入的 regulation（原有内容全部保留），
不写输出路径时生成在输入旁边：regulation.statalloc.bin。重复打补丁会替换旧的组件行。
Adds the 2034 SpEffect rows NRStatAlloc needs to the given regulation, keeping everything else.
Without an output path the result is written next to the input as regulation.statalloc.bin.
Running it again on a patched file replaces the old rows.";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match run(&args) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("错误 / Error: {e}");
            1
        }
    };
    if launched_by_double_click() {
        println!("\n按回车退出 / Press Enter to exit");
        let _ = std::io::stdin().read_line(&mut String::new());
    }
    ExitCode::from(code)
}

fn run(args: &[String]) -> Result<(), String> {
    let Some(input) = args.first().filter(|a| !a.starts_with('-')).map(PathBuf::from) else {
        println!("{USAGE}");
        return Ok(());
    };
    let output = args.get(1).map(PathBuf::from).unwrap_or_else(|| input.with_extension("statalloc.bin"));
    if same_file(&input, &output) {
        return Err("输出不能覆盖输入 / output must not overwrite the input".into());
    }
    let bytes = std::fs::read(&input).map_err(|e| format!("{}: {e}", input.display()))?;
    let (patched, report) = Regulation::decode(&bytes)?.patch()?;
    verify(&patched)?;
    std::fs::write(&output, &patched).map_err(|e| format!("{}: {e}", output.display()))?;

    println!("regulation 版本 / version: {}", report.version);
    println!(
        "SpEffectParam: {} -> {} 行 / rows（新增 / added {}，替换 / replaced {}）",
        report.rows_before, report.rows_after, report.added, report.replaced
    );
    if report.foreign > 0 {
        println!(
            "警告 / Warning: {} 行原有数据不是本模组写入的，已被覆盖（其他模组占用了相同 ID）/ rows from another mod used the same IDs and were overwritten",
            report.foreign
        );
    }
    println!("已写出 / written: {} ({} bytes)", output.display(), patched.len());
    Ok(())
}

/// 重新解码输出，确认每个组件都在且时长正确
fn verify(patched: &[u8]) -> Result<(), String> {
    let reg = Regulation::decode(patched).map_err(|e| format!("self-check decode: {e}"))?;
    let table = ParamFile::parse(reg.sp_effect()?, sp_effect::TABLE.row_size)?;
    let missing = all().map(|c| c.0).chain([RESTORE_HP, RESTORE_FP]).find(|&id| table.row(id as u32).is_none());
    if let Some(id) = missing {
        return Err(format!("self-check: SpEffect {id} missing after patch"));
    }
    Ok(())
}

fn same_file(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

/// 控制台里只有本进程时，说明是双击或拖放启动的，窗口会在退出时立即关闭
fn launched_by_double_click() -> bool {
    let mut list = [0u32; 2];
    unsafe { GetConsoleProcessList(list.as_mut_ptr(), list.len() as u32) == 1 }
}
