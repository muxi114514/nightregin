//! 把规则结果写回参数表：降攻效果是新增的常驻 SpEffect（以原版中性档 7700 为模板，只改五种攻击力倍率），
//! 挂到首领 NpcParam 的空 SpEffect 槽上，与档位、深夜修正连乘。ID = 9969000 + 保留百分比。

use std::collections::BTreeSet;

use nr_stat_alloc_patcher::param_write::{self, OwnedRow};
use nr_stat_alloc_shared::param::ParamFile;

use crate::layout::{npc, sp_effect};
use crate::rules::Rows;
use crate::tables::{i32_at, put};

const SP_BASE: u32 = 9_969_000;
const SP_RANGE: std::ops::Range<u32> = SP_BASE..SP_BASE + 100;
/// 本工具写入的行名前缀，用来区分"上次的结果"和"其他模组占用了同一 ID"
const NAME_PREFIX: &str = "[MMVBalance]";

pub fn attack_effect_id(percent: u8) -> u32 {
    SP_BASE + percent as u32
}

/// 挂上降攻效果：已挂本工具的效果则替换，否则用最靠后的空槽（-1 优先，其次 0）
pub fn attach(row: &mut [u8], effect: u32) -> Result<(), String> {
    let slots = npc::SP_EFFECT_SLOTS;
    let value = |row: &[u8], o: usize| i32_at(row, o);
    let slot = slots
        .iter()
        .find(|&&o| SP_RANGE.contains(&(value(row, o) as u32)))
        .or_else(|| slots.iter().rev().find(|&&o| value(row, o) == -1))
        .or_else(|| slots.iter().rev().find(|&&o| value(row, o) == 0))
        .ok_or("no free SpEffect slot")?;
    put(row, *slot, (effect as i32).to_le_bytes());
    Ok(())
}

/// 原地覆盖改动行的数据，其余字节不动（MMV 的 NpcParam 有一处行顺序颠倒，重写整表会改变它）
pub fn npc_param(base: &ParamFile, rows: &Rows) -> Vec<u8> {
    let mut out = base.bytes().to_vec();
    for r in base.rows() {
        if let Some(data) = rows.get(&r.id) {
            out[r.offset..r.offset + data.len()].copy_from_slice(data);
        }
    }
    out
}

/// 去掉上次写入的降攻效果，按需要的百分比重新加入
pub fn sp_effect_param(base: &ParamFile, percents: &BTreeSet<u8>) -> Result<Vec<u8>, String> {
    let template = base.row(sp_effect::NEUTRAL_TEMPLATE).ok_or("template SpEffect 7700 missing")?;
    let mut rows = Vec::with_capacity(base.len() + percents.len());
    for r in param_write::rows_of(base) {
        if !SP_RANGE.contains(&r.id) {
            rows.push(r);
        } else if !r.name.as_deref().is_some_and(|n| n.starts_with(NAME_PREFIX)) {
            return Err(format!("SpEffect {} is used by another mod", r.id));
        }
    }
    let added = percents.iter().map(|&p| {
        let mut data = template.to_vec();
        for o in sp_effect::ATTACK_POWER_RATES {
            put(&mut data, o, (p as f32 / 100.0).to_le_bytes());
        }
        let name = Some(format!("{NAME_PREFIX} Attack x{:.2}", p as f32 / 100.0));
        OwnedRow { id: attack_effect_id(p), data, name }
    });
    param_write::write_ordered(base, &param_write::merge(rows, added.collect()))
}
