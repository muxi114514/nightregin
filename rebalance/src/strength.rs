//! 敌人的原始攻击力：NpcParam.behaviorVariationId → BehaviorParam（行内 variationId）→ AtkParam_Npc，
//! 子弹类行为再经 Bullet.atkId_Bullet。原始值为五种属性攻击力之和，未乘档位与深夜修正
//! （同一档位的敌人倍率相同，所以同档比较原始值即可）。

use std::collections::HashMap;

use nr_stat_alloc_shared::param::field;

use crate::layout::{atk, behavior, bullet};
use crate::tables::Tables;

/// 行为变体 → 该变体全部攻击的原始伤害（升序，不含 0）
pub struct AttackIndex(HashMap<i32, Vec<u32>>);

#[derive(Clone, Copy, Debug)]
pub struct Strength {
    /// 90 分位：代表常规重击
    pub p90: u32,
    /// 单下最高
    pub max: u32,
}

impl AttackIndex {
    pub fn build(t: &Tables) -> Self {
        let mut map: HashMap<i32, Vec<u32>> = HashMap::new();
        for row in t.behavior.rows() {
            let Some(var) = field::i32(row.data, behavior::VARIATION) else { continue };
            let ref_id = field::i32(row.data, behavior::REF_ID).unwrap_or(-1);
            let atk_id = match field::u8(row.data, behavior::REF_TYPE) {
                Some(behavior::REF_ATTACK) => ref_id,
                Some(behavior::REF_BULLET) => {
                    t.bullet.row(ref_id as u32).and_then(|b| field::i32(b, bullet::ATK_ID)).unwrap_or(-1)
                }
                _ => continue,
            };
            let damage = damage_of(t, atk_id);
            if damage > 0 {
                map.entry(var).or_default().push(damage);
            }
        }
        map.values_mut().for_each(|v| v.sort_unstable());
        Self(map)
    }

    pub fn strength(&self, variation: i32) -> Option<Strength> {
        let a = self.0.get(&variation).filter(|a| !a.is_empty())?;
        Some(Strength { p90: a[(a.len() * 9 / 10).min(a.len() - 1)], max: a[a.len() - 1] })
    }
}

fn damage_of(t: &Tables, atk_id: i32) -> u32 {
    if atk_id < 0 {
        return 0;
    }
    let Some(row) = t.atk.row(atk_id as u32) else { return 0 };
    atk::DAMAGE.iter().map(|&o| field::u16(row, o).unwrap_or(0) as u32).sum()
}

/// 升序列表的分位数（q = 1 为最大值）
pub fn quantile(sorted: &[u32], q: f64) -> u32 {
    sorted[(((sorted.len() - 1) as f64) * q).round() as usize]
}
