//! 武器词条升档表：锻造石强化武器时把词条换成同类的上位档，上位档全部是 AttachEffectParam 里原版已有的行。
//! 同一词条的各档共用文本 ID，ID 相邻（步长 +1 / +10 / +50），显示值的绝对值递增。
//! 不能按 overrideBaseEffectId 分组：深夜专属词条的紫色档（8850000 族）与红色档（8850050 族）只有文本 ID 相同。
//! 负面词条显示值为负（如 -4% → -6%），比较绝对值即可与正面词条统一处理。

use std::collections::{HashMap, HashSet};

use crate::param::layout::attach_effect;
use crate::param::{ParamFile, field};

/// 同一词条的各档 ID 相差不超过此值；更远的同文本行是别的变体（Debug 行、遗物版等）
const MAX_TIER_GAP: u32 = 100;

#[derive(Debug, Default, Clone)]
pub struct PassiveTiers {
    next: HashMap<u32, u32>,
    known: HashSet<u32>,
}

impl PassiveTiers {
    pub fn build(table: &ParamFile) -> Self {
        Self::from_rows(table.rows().filter_map(|r| {
            let text = field::i32(r.data, attach_effect::ATTACH_TEXT_ID)?;
            let value = field::i32(r.data, attach_effect::DISPLAYED_VALUE)?;
            Some((r.id, text, value))
        }))
    }

    /// rows：(词条 ID, 文本 ID, 显示值)
    pub fn from_rows(rows: impl IntoIterator<Item = (u32, i32, i32)>) -> Self {
        let mut known = HashSet::new();
        let mut groups: HashMap<i32, Vec<(u32, u32)>> = HashMap::new();
        for (id, text, value) in rows {
            known.insert(id);
            if text > 0 {
                groups.entry(text).or_default().push((id, value.unsigned_abs()));
            }
        }
        let mut next = HashMap::new();
        for rows in groups.values_mut() {
            rows.sort_unstable();
            for (i, &(id, value)) in rows.iter().enumerate() {
                let limit = id.saturating_add(MAX_TIER_GAP);
                let upper = rows[i + 1..].iter().take_while(|&&(other, _)| other < limit).find(|&&(_, v)| v > value);
                if let Some(&(up, _)) = upper {
                    next.insert(id, up);
                }
            }
        }
        Self { next, known }
    }

    pub fn next(&self, id: u32) -> Option<u32> {
        self.next.get(&id).copied()
    }

    /// 升 steps 档，到顶为止；没有上位档时原样返回
    pub fn upgrade(&self, id: u32, steps: u32) -> u32 {
        (0..steps).try_fold(id, |cur, _| self.next(cur)).unwrap_or_else(|| self.top(id))
    }

    /// 是否为 AttachEffectParam 中存在的行
    pub fn is_known(&self, id: u32) -> bool {
        self.known.contains(&id)
    }

    /// 有上位档的词条数
    pub fn len(&self) -> usize {
        self.next.len()
    }

    pub fn is_empty(&self) -> bool {
        self.next.is_empty()
    }

    fn top(&self, mut id: u32) -> u32 {
        while let Some(up) = self.next(id) {
            id = up;
        }
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> PassiveTiers {
        PassiveTiers::from_rows([
            // 三档，步长 +1；同文本的 Debug 行显示值更低，不算上位
            (8100100, 8100100, 6),
            (8100101, 8100100, 9),
            (8100102, 8100100, 12),
            (8990000, 8100100, 6),
            // 两条子链共用文本：第一条的顶档不能跳到第二条
            (8110000, 8110000, 18),
            (8110001, 8110000, 23),
            (8110002, 8110000, 30),
            (8110010, 8110000, 18),
            (8110011, 8110000, 23),
            (8110012, 8110000, 30),
            // 显示值相同的变体不是上位
            (8110703, 8110700, 26),
            (8110700, 8110700, 25),
            (8110701, 8110700, 30),
            // 负面词条比绝对值
            (8760000, 8760000, -4),
            (8760050, 8760000, -6),
            // 深夜专属：紫色档与红色档只有文本相同
            (8850000, 18000000, 8),
            (8850050, 18000000, 12),
            // 没有文本的行
            (10000, -1, 0),
        ])
    }

    #[test]
    fn links_tiers_by_text_and_magnitude() {
        let t = sample();
        assert_eq!(t.next(8100100), Some(8100101));
        assert_eq!(t.next(8100101), Some(8100102));
        assert_eq!(t.next(8100102), None);
        assert_eq!(t.next(8110002), None);
        assert_eq!(t.next(8110010), Some(8110011));
        assert_eq!(t.next(8110700), Some(8110701));
        assert_eq!(t.next(8110703), None);
        assert_eq!(t.next(8760000), Some(8760050));
        assert_eq!(t.next(8850000), Some(8850050));
        assert_eq!(t.next(10000), None);
        assert!(t.is_known(10000) && !t.is_known(12345));
    }

    #[test]
    fn upgrade_stops_at_top() {
        let t = sample();
        assert_eq!(t.upgrade(8100100, 1), 8100101);
        assert_eq!(t.upgrade(8100100, 2), 8100102);
        assert_eq!(t.upgrade(8100100, 3), 8100102);
        assert_eq!(t.upgrade(8100102, 1), 8100102);
        assert_eq!(t.upgrade(8850000, 3), 8850050);
        assert_eq!(t.upgrade(12345, 1), 12345);
        assert_eq!(t.upgrade(8100100, 0), 8100100);
    }
}
