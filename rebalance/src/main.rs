//! MMV 空城整合包首领强度重平衡（保留深夜层数）：
//!   mmv_rebalance <整合包 regulation> <输出> --vanilla <原版 regulation> --mmv <MMV regulation> [--quantile 0.75]
//! 输入一般是 modKongCheng\regulation.bin，之后接 mmv_relics 与 nr_stat_alloc_patcher（tools\build_mmv_pkg.ps1）。规则见 rules.rs。
//! 对已经处理过的文件重复运行结果不变。

mod apply;
mod layout;
mod rules;
mod strength;
mod tables;

use std::collections::{BTreeSet, HashMap};
use std::process::ExitCode;

use nr_stat_alloc_patcher::regulation::Regulation;

use layout::npc;
use strength::AttackIndex;
use tables::{Tables, open};

struct Args {
    base: String,
    output: String,
    vanilla: String,
    mmv: String,
    quantile: f64,
}

fn main() -> ExitCode {
    match parse_args().and_then(|a| run(&a)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("错误: {e}");
            ExitCode::FAILURE
        }
    }
}

fn parse_args() -> Result<Args, String> {
    let usage = "用法: mmv_rebalance <整合包 regulation> <输出> --vanilla <原版> --mmv <MMV> [--quantile 0.75]";
    let mut positional = Vec::new();
    let (mut vanilla, mut mmv, mut quantile) = (None, None, 0.75);
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--vanilla" => vanilla = it.next(),
            "--mmv" => mmv = it.next(),
            "--quantile" => quantile = it.next().and_then(|q| q.parse().ok()).filter(|q| (0.0..=1.0).contains(q)).ok_or(usage)?,
            _ => positional.push(a),
        }
    }
    let [base, output] = <[String; 2]>::try_from(positional).map_err(|_| usage)?;
    Ok(Args { base, output, vanilla: vanilla.ok_or(usage)?, mmv: mmv.ok_or(usage)?, quantile })
}

fn decode(path: &str) -> Result<Regulation, String> {
    Regulation::decode(&std::fs::read(path).map_err(|e| format!("{path}: {e}"))?).map_err(|e| format!("{path}: {e}"))
}

fn run(a: &Args) -> Result<(), String> {
    let (base_reg, vanilla_reg, mmv_reg) = (decode(&a.base)?, decode(&a.vanilla)?, decode(&a.mmv)?);
    let base = Tables::load(&base_reg)?;
    let vanilla = open(&vanilla_reg, &npc::TABLE)?;
    let mmv = open(&mmv_reg, &npc::TABLE)?;
    let names: HashMap<u32, String> = base.npc.rows().filter_map(|r| base.npc.name(r.index).map(|n| (r.id, n))).collect();
    let attacks = AttackIndex::build(&base);

    let mut rows = rules::rows_of(&base.npc);
    let tiers = rules::revert_tiers(&vanilla, &mut rows);
    println!("规则1 原版敌人档位改回原版: {} 行", tiers.len());
    for id in &tiers {
        println!("    {id} {}", names.get(id).map_or("", String::as_str));
    }
    println!("规则2 空城包血量 x1.3 改回 MMV: {} 行", rules::revert_hp(&mmv, &mut rows));

    let caps = rules::caps(&vanilla, &names, &attacks, a.quantile);
    println!("规则3 原版同档首领上限（{:.0} 分位）:", a.quantile * 100.0);
    for (tier, c) in &caps {
        let src = if c.named { "夜晚首领" } else { "同档全部" };
        println!("    档 {tier}: 参照 {} 行（{src}） 常规重击 {} / 单下最高 {} / 血量 {}", c.refs, c.p90, c.max, c.hp);
    }
    let capped = rules::cap_new_bosses(&vanilla, &caps, &attacks, &mut rows);
    let mut percents = BTreeSet::new();
    for c in &capped {
        let row = rows.get_mut(&c.id).unwrap();
        if let Some(p) = c.attack_percent {
            apply::attach(row, apply::attack_effect_id(p)).map_err(|e| format!("NpcParam {}: {e}", c.id))?;
            percents.insert(p);
        }
        let atk = c.attack_percent.map_or("不变".to_string(), |p| format!("x{:.2}", p as f32 / 100.0));
        let name = names.get(&c.id).map_or("", String::as_str);
        println!("    {:>10} 档{} 攻击 {}/{} {atk:>5}  血量 {} -> {}  {name}", c.id, c.tier, c.strength.p90, c.strength.max, c.hp.0, c.hp.1);
    }
    let lowered = capped.iter().filter(|c| c.attack_percent.is_some()).count();
    println!("新首领调整 {} 行（降攻 {lowered}），新增降攻效果 {} 种", capped.len(), percents.len());

    let npc_bytes = apply::npc_param(&base.npc, &rows);
    let sp_bytes = apply::sp_effect_param(&base.sp_effect, &percents)?;
    let out = base_reg.encode_with(&[(npc::TABLE.file, npc_bytes), (layout::sp_effect::TABLE.file, sp_bytes)])?;
    verify(&out, &base, &percents)?;
    std::fs::write(&a.output, &out).map_err(|e| format!("{}: {e}", a.output))?;
    println!("已写出 {} ({} bytes)", a.output, out.len());
    Ok(())
}

/// 重新解码输出：行数不变、降攻效果都在
fn verify(out: &[u8], base: &Tables, percents: &BTreeSet<u8>) -> Result<(), String> {
    let reg = Regulation::decode(out).map_err(|e| format!("self-check: {e}"))?;
    let t = Tables::load(&reg)?;
    if t.npc.len() != base.npc.len() {
        return Err(format!("self-check: NpcParam rows {} != {}", t.npc.len(), base.npc.len()));
    }
    if let Some(p) = percents.iter().find(|&&p| t.sp_effect.row(apply::attack_effect_id(p)).is_none()) {
        return Err(format!("self-check: attack effect x{p} missing"));
    }
    Ok(())
}
