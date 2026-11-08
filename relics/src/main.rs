//! 自定义遗物词条、护符、锻体、武器词条（数值与新增词条）与停用夜晚首领（参数表部分），MMV 空城整合包与原版独立版共用：
//!   mmv_relics <输入 regulation> <输出> [--mmv-map-icons] [--tiered-buffs]
//! 整合包接在 mmv_rebalance 之后，独立版直接以原版 regulation 为输入，之后都接 nr_stat_alloc_patcher；行定义见 content/。重复运行结果不变。

mod content;
mod insert;
mod layout;
mod play_area;
mod pool;
mod spec;
mod tiered;

use std::process::ExitCode;

use nr_stat_alloc_patcher::param_write;
use nr_stat_alloc_patcher::regulation::Regulation;

use spec::{Patch, RowSpec};

/// MMV 系 pkg 专用：湖沼 / 锻造村改用普通地图能显示的图标行（见 content::map_icons）
const MAP_ICONS_FLAG: &str = "--mmv-map-icons";
/// 金 / 紫 / 蓝三档锻体与角色专属锻体（见 tiered），目前只给不带空城的 MMV
const TIERED_FLAG: &str = "--tiered-buffs";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("错误: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let map_icons = args.iter().any(|a| a == MAP_ICONS_FLAG);
    let tiered = args.iter().any(|a| a == TIERED_FLAG);
    args.retain(|a| a != MAP_ICONS_FLAG && a != TIERED_FLAG);
    let [input, output] = <[String; 2]>::try_from(args)
        .map_err(|_| format!("用法: mmv_relics <输入 regulation> <输出> [{MAP_ICONS_FLAG}] [{TIERED_FLAG}]"))?;
    let reg = Regulation::decode(&std::fs::read(&input).map_err(|e| format!("{input}: {e}"))?)?;
    let specs: Vec<&RowSpec> = content::all(map_icons, tiered).collect();
    let patches = content::patches(tiered);
    let mut seen = std::collections::HashSet::new();
    if let Some(dup) = specs.iter().find(|s| !seen.insert((s.table.file, s.id))) {
        return Err(format!("{} {} 在行定义里重复", dup.table.file, dup.id));
    }

    let mut replacements = Vec::new();
    // 实际写入的行定义（跳过本整合包没有的可选行），自检按它核对
    let mut applied = Vec::new();
    // 各表的冲突一次报全，方便适配新的整合包
    let mut errors = Vec::new();
    for table in layout::TABLES {
        let mine: Vec<&RowSpec> = specs.iter().copied().filter(|s| std::ptr::eq(s.table, table)).collect();
        let my_patches: Vec<&Patch> = patches.iter().filter(|p| std::ptr::eq(p.table, table)).collect();
        if mine.is_empty() && my_patches.is_empty() {
            continue;
        }
        let file = insert::open(&reg, table)?;
        let mine: Vec<&RowSpec> = mine.into_iter().filter(|s| !content::is_optional(s) || file.row(s.id).is_some()).collect();
        applied.extend(mine.iter().copied());
        let mut report = insert::Report::default();
        match insert::apply(&file, &mine, &my_patches, &content::in_place(table.file), &mut report) {
            Ok(bytes) => {
                println!(
                    "{}: 新增 {} 行，替换 {} 行，原地改写原版行 {} 行，按开关改字段 {} 行",
                    table.file, report.added, report.replaced, report.in_place, report.patched
                );
                replacements.push((table.file, bytes));
            }
            Err(e) => errors.push(format!("{}: {e}", table.file)),
        }
    }
    if !errors.is_empty() {
        return Err(errors.join("\n"));
    }
    replacements.push((layout::ITEM_TABLE.file, item_tables(&reg, tiered)?));
    if tiered {
        let tables = insert::open(&reg, &layout::ATTACH_EFFECT_TABLE)?;
        let mut rows = param_write::rows_of(&tables);
        let added = tiered::passives::add_to_tables(&mut rows).map_err(|e| format!("{}: {e}", layout::ATTACH_EFFECT_TABLE.file))?;
        println!("{}: 新武器词条追加 {added} 行", layout::ATTACH_EFFECT_TABLE.file);
        replacements.push((layout::ATTACH_EFFECT_TABLE.file, param_write::write_ordered(&tables, &rows)?));
        let mut loot = tiered::loot::Report::default();
        replacements.extend(tiered::loot::apply(&reg, &mut loot)?);
        println!(
            "掉落行：改写 {} 行，必出金 {} 行，指定蓝紫混合 {} 行，深夜红皮 {} 行，入侵据点 {} 行，罪人入侵补锻体选项 {} 行，宝箱怪选项 {} 行",
            loot.rows, loot.gold, loot.fixed, loot.variant, loot.condemned, loot.invader, loot.mimic
        );
    }
    let areas = insert::open(&reg, &layout::PLAY_AREA)?;
    let mut swapped = play_area::Report::default();
    let bytes = play_area::apply(&areas, content::night_bosses::SWAPS, &mut swapped).map_err(|e| format!("{}: {e}", layout::PLAY_AREA.file))?;
    println!("{}: 停用的夜晚首领换掉 {} 格", layout::PLAY_AREA.file, swapped.replaced);
    replacements.push((layout::PLAY_AREA.file, bytes));
    let out = reg.encode_with(&replacements)?;
    verify(&out, &applied, patches, tiered)?;
    std::fs::write(&output, &out).map_err(|e| format!("{output}: {e}"))?;
    println!("已写出 {output} ({} bytes)", out.len());
    Ok(())
}

/// 锻体候选池：先按 POOL_MOVES 增删，开了三档再生成新池并改嵌套引用
fn item_tables(reg: &Regulation, tiered: bool) -> Result<Vec<u8>, String> {
    let items = insert::open(reg, &layout::ITEM_TABLE)?;
    let mut rows = param_write::rows_of(&items);
    let mut moved = pool::Report::default();
    pool::apply(&mut rows, content::permanent_buffs::POOL_MOVES, &mut moved).map_err(|e| format!("{}: {e}", layout::ITEM_TABLE.file))?;
    println!("{}: 锻体候选移出 {} 行，移入 {} 行", layout::ITEM_TABLE.file, moved.removed, moved.added);
    if tiered {
        let mut built = tiered::pools::Report::default();
        tiered::pools::apply(&mut rows, &mut built).map_err(|e| format!("{}: {e}", layout::ITEM_TABLE.file))?;
        println!(
            "{}: 蓝池 {} 条、紫池 {} 条、金池 {} 条、角色专属 {} 条，外层表嵌套引用改指 {} 行",
            layout::ITEM_TABLE.file, built.blue, built.purple, built.gold, built.role, built.nested
        );
    }
    param_write::write_ordered(&items, &rows)
}

/// 重新解码输出，逐行核对改过的字段、锻体池、掉落与夜晚首领
fn verify(out: &[u8], specs: &[&RowSpec], patches: &[Patch], tiered: bool) -> Result<(), String> {
    let reg = Regulation::decode(out).map_err(|e| format!("self-check: {e}"))?;
    let items = insert::open(&reg, &layout::ITEM_TABLE)?;
    pool::verify(&items, content::permanent_buffs::POOL_MOVES)?;
    if tiered {
        tiered::pools::verify(&items, &insert::open(&reg, &layout::EQUIP_GOODS)?, &insert::open(&reg, &layout::PERMANENT_BUFF)?)?;
        tiered::loot::verify(&reg)?;
        tiered::passives::verify(&insert::open(&reg, &layout::ATTACH_EFFECT_TABLE)?)?;
    }
    play_area::verify(&insert::open(&reg, &layout::PLAY_AREA)?, content::night_bosses::SWAPS)?;
    // 行定义里被按开关改掉的字段以改后的值为准，不按原定义核对
    let patched: std::collections::HashSet<(&str, u32, usize)> =
        patches.iter().flat_map(|p| p.sets.iter().map(move |s| (p.table.file, p.id, s.offset()))).collect();
    let from_specs = specs.iter().map(|s| (s.table, s.id, s.sets.iter().filter(|set| !patched.contains(&(s.table.file, s.id, set.offset()))).collect::<Vec<_>>()));
    let from_patches = patches.iter().map(|p| (p.table, p.id, p.sets.iter().collect::<Vec<_>>()));
    for (table, id, sets) in from_specs.chain(from_patches) {
        let file = insert::open(&reg, table)?;
        let row = file.row(id).ok_or(format!("self-check: {} {} missing", table.file, id))?;
        for set in sets {
            let mut expected = row.to_vec();
            set.apply(&mut expected);
            if expected != row {
                return Err(format!("self-check: {} {} field @{} not written", table.file, id, set.offset()));
            }
        }
    }
    Ok(())
}
