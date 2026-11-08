//! 三条规则，依次作用在整合包 NpcParam 的工作副本上：
//! 1. 原版敌人的强度档位字段（档位倍率、多人修正、第二天、深夜修正）被整合包改过的，改回原版
//! 2. 空城包把血量统一乘 1.3 的行（与 MMV 比对得出），血量改回 MMV 的值
//! 3. 整合包新增、处在夜晚首领档（7750~7758）的首领：血量与攻击不超过原版同档首领的分位数，
//!    参照优先取行名带 "Night Boss" 的原版夜晚首领（不足 3 个时取同档全部）。攻击由 apply 挂降攻效果实现

use std::collections::{BTreeMap, HashMap};

use nr_stat_alloc_shared::param::ParamFile;

use crate::layout::npc;
use crate::strength::{AttackIndex, Strength, quantile};
use crate::tables::{i32_at, put, u32_at};

pub type Rows = BTreeMap<u32, Vec<u8>>;

const KONG_CHENG_HP_RATE: f64 = 1.3;
const BOSS_TIERS: std::ops::RangeInclusive<i32> = 7750..=7758;
/// MMV 新加的 7781（夜晚首领档，不加血量）伤害同 7756
const TIER_ALIAS: [(i32, i32); 1] = [(7781, 7756)];
/// 原版参照里排除的行名片段：首领的骨灰/随从、前奏战、群怪、多人副本、隐藏实体等（"Ancestor Spirit" 这类首领名不受影响）
const REF_SKIP: [&str; 8] = ["Boss Spirit", "Boss Add", "Prelude", "Horde", "Raid", "Clone", "Invis", "Easter"];
/// 血量超过上限这么多倍的是无敌实体或彩蛋，不处理
const SENTINEL_HP_TIMES: u32 = 20;

pub struct Cap {
    pub p90: u32,
    pub max: u32,
    pub hp: u32,
    pub refs: usize,
    pub named: bool,
}

pub struct Capped {
    pub id: u32,
    pub tier: i32,
    pub strength: Strength,
    /// 攻击保留的百分比（5 的倍数），None 表示不降攻
    pub attack_percent: Option<u8>,
    pub hp: (u32, u32),
}

pub fn rows_of(file: &ParamFile) -> Rows {
    file.rows().map(|r| (r.id, r.data.to_vec())).collect()
}

/// 规则 1：返回被改回的行
pub fn revert_tiers(vanilla: &ParamFile, rows: &mut Rows) -> Vec<u32> {
    let mut reverted = Vec::new();
    for v in vanilla.rows() {
        let Some(row) = rows.get_mut(&v.id) else { continue };
        let mut changed = false;
        for o in npc::TIER_FIELDS {
            if row[o..o + 4] != v.data[o..o + 4] {
                row[o..o + 4].copy_from_slice(&v.data[o..o + 4]);
                changed = true;
            }
        }
        if changed {
            reverted.push(v.id);
        }
    }
    reverted
}

/// 规则 2：返回改回的行数（不是 1.3 倍关系的血量改动是空城包有意为之，保留）
pub fn revert_hp(mmv: &ParamFile, rows: &mut Rows) -> usize {
    let mut count = 0;
    for m in mmv.rows() {
        let Some(row) = rows.get_mut(&m.id) else { continue };
        let (before, now) = (u32_at(m.data, npc::HP), u32_at(row, npc::HP));
        if now != before && (now as f64 - (before as f64 * KONG_CHENG_HP_RATE).round()).abs() <= 1.0 {
            put(row, npc::HP, before.to_le_bytes());
            count += 1;
        }
    }
    count
}

/// 原版各夜晚首领档的上限（q 分位）
pub fn caps(vanilla: &ParamFile, names: &HashMap<u32, String>, attacks: &AttackIndex, q: f64) -> BTreeMap<i32, Cap> {
    let mut groups: BTreeMap<i32, Vec<(bool, Strength, u32)>> = BTreeMap::new();
    for v in vanilla.rows() {
        let tier = i32_at(v.data, npc::CHAOS_CORRECT);
        let hp = u32_at(v.data, npc::HP);
        let name = names.get(&v.id).map_or("", String::as_str);
        if !BOSS_TIERS.contains(&tier) || hp <= 1 || REF_SKIP.iter().any(|w| name.contains(w)) {
            continue;
        }
        if let Some(s) = attacks.strength(i32_at(v.data, npc::BEHAVIOR_VARIATION)) {
            groups.entry(tier).or_default().push((name.contains("Night Boss"), s, hp));
        }
    }
    groups
        .into_iter()
        .map(|(tier, all)| {
            let named: Vec<_> = all.iter().filter(|r| r.0).copied().collect();
            let use_named = named.len() >= 3;
            let set = if use_named { named } else { all };
            let pick = |f: fn(&(bool, Strength, u32)) -> u32| {
                let mut v: Vec<u32> = set.iter().map(f).collect();
                v.sort_unstable();
                quantile(&v, q)
            };
            let cap = Cap { p90: pick(|r| r.1.p90), max: pick(|r| r.1.max), hp: pick(|r| r.2), refs: set.len(), named: use_named };
            (tier, cap)
        })
        .collect()
}

/// 规则 3：在 rows 上压血量，返回需要降攻的行与百分比
pub fn cap_new_bosses(vanilla: &ParamFile, caps: &BTreeMap<i32, Cap>, attacks: &AttackIndex, rows: &mut Rows) -> Vec<Capped> {
    let mut out = Vec::new();
    for (&id, row) in rows.iter_mut() {
        if vanilla.row(id).is_some() {
            continue;
        }
        let raw_tier = i32_at(row, npc::CHAOS_CORRECT);
        let tier = TIER_ALIAS.iter().find(|(from, _)| *from == raw_tier).map_or(raw_tier, |(_, to)| *to);
        let (Some(cap), Some(s)) = (caps.get(&tier), attacks.strength(i32_at(row, npc::BEHAVIOR_VARIATION))) else { continue };
        let hp = u32_at(row, npc::HP);
        if hp <= 1 || hp > cap.hp * SENTINEL_HP_TIMES {
            continue;
        }
        let ratio = (cap.p90 as f64 / s.p90 as f64).min(cap.max as f64 / s.max as f64);
        let attack_percent = (ratio < 0.975).then(|| ((ratio * 20.0).round() * 5.0).clamp(30.0, 95.0) as u8);
        let new_hp = hp.min(cap.hp);
        if attack_percent.is_none() && new_hp == hp {
            continue;
        }
        put(row, npc::HP, new_hp.to_le_bytes());
        out.push(Capped { id, tier, strength: s, attack_percent, hp: (hp, new_hp) });
    }
    out
}
