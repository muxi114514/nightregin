//! 局内武器词条继承的规则：第一把武器的全部词条整把覆盖到第二把上，第一把随后消失。
//! 按当前稀有度分档：白 0 / 蓝 1 / 紫、红、金 2，来源档位不低于目标才允许（同档互换、高档给低档）。
//! 当前稀有度 = 本体稀有度 + 强化等级（物品 ID 末两位），最高传说；红色（深夜 [Unique] 自定义武器行）本体都是稀有，落在第 2 档。
//! 空手与箭矢不带词条，不收进表里：空武器槽装的是空手（110000）条目，误选时在核对阶段就拒绝，不会被写词条或移除。

use std::collections::HashMap;

use crate::param::layout::equip_weapon;
use crate::param::{ParamFile, field};

/// 传说
const LEGENDARY: u8 = 3;
/// 稀有及以上（紫 / 红 / 金）同为最高档
const TOP_TIER: u8 = 2;
/// 不带词条的类别：空手、箭、大箭、弩箭、大弩箭
const NO_PASSIVE_TYPES: [u16; 5] = [33, 81, 83, 85, 86];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InheritError {
    /// 参数表里没有，或是空手 / 箭矢
    NotInheritable(u32),
    /// 来源档位低于目标（如白给蓝、蓝给紫）
    LowerTier { source: u8, target: u8 },
}

#[derive(Debug, Default, Clone)]
pub struct Rarities {
    base: HashMap<u32, u8>,
}

impl Rarities {
    pub fn build(table: &ParamFile) -> Self {
        Self::from_rows(table.rows().filter_map(|r| {
            Some((r.id, field::u16(r.data, equip_weapon::WEP_TYPE)?, field::u8(r.data, equip_weapon::RARITY)?))
        }))
    }

    /// rows：(武器本体 ID, wepType, 本体稀有度)
    pub fn from_rows(rows: impl IntoIterator<Item = (u32, u16, u8)>) -> Self {
        let base = rows.into_iter().filter(|(_, t, _)| !NO_PASSIVE_TYPES.contains(t)).map(|(id, _, rarity)| (id, rarity)).collect();
        Self { base }
    }

    pub fn len(&self) -> usize {
        self.base.len()
    }

    pub fn is_empty(&self) -> bool {
        self.base.is_empty()
    }

    /// 本体稀有度（不含强化等级）
    pub fn base(&self, id: u32) -> Option<u8> {
        self.base.get(&(id - id % 100)).copied()
    }

    /// 带强化等级的物品 ID 的当前稀有度（0~3）
    pub fn current(&self, id: u32) -> Option<u8> {
        let base = *self.base.get(&(id - id % 100))?;
        let level = u8::try_from(id % 100).unwrap_or(LEGENDARY);
        Some(base.saturating_add(level).min(LEGENDARY))
    }

    /// 继承用的档位（0~2）
    pub fn tier(&self, id: u32) -> Option<u8> {
        self.current(id).map(|r| r.min(TOP_TIER))
    }

    /// source / target 为带强化等级的物品 ID
    pub fn check(&self, source: u32, target: u32) -> Result<(), InheritError> {
        let s = self.tier(source).ok_or(InheritError::NotInheritable(source))?;
        let t = self.tier(target).ok_or(InheritError::NotInheritable(target))?;
        if s < t {
            return Err(InheritError::LowerTier { source: s, target: t });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Rarities {
        // 普通长剑、少见、稀有、传说、本体 ID 不以 00 结尾的武器、空手、箭
        Rarities::from_rows([(2000000, 5, 0), (2010000, 5, 1), (2020000, 5, 2), (2030000, 5, 3), (1000500, 1, 0), (110000, 33, 0), (50000000, 81, 0)])
    }

    #[test]
    fn current_rarity_adds_reinforce_level() {
        let r = sample();
        assert_eq!([2000000, 2000001, 2000002, 2000003].map(|id| r.current(id)), [Some(0), Some(1), Some(2), Some(3)]);
        assert_eq!(r.current(2010002), Some(3));
        assert_eq!(r.current(2020001), Some(3));
        assert_eq!(r.current(1000502), Some(2));
        assert_eq!(r.current(9990000), None);
    }

    #[test]
    fn same_or_higher_tier_may_overwrite() {
        let r = sample();
        // 白对白、蓝对蓝、紫 / 红 / 金互通、金对蓝、紫对白
        for (s, t) in [(2000000, 2000000), (2010000, 2000001), (2020000, 2030000), (2030000, 2020000), (2000003, 2010001), (2020000, 2000000)] {
            assert_eq!(r.check(s, t), Ok(()), "{s} -> {t}");
        }
        assert_eq!(r.check(2000000, 2010000), Err(InheritError::LowerTier { source: 0, target: 1 }));
        assert_eq!(r.check(2010000, 2020000), Err(InheritError::LowerTier { source: 1, target: 2 }));
        assert_eq!(r.check(2000001, 2000002), Err(InheritError::LowerTier { source: 1, target: 2 }));
        assert_eq!(r.check(9990000, 2000000), Err(InheritError::NotInheritable(9990000)));
        // 空手、箭矢无论作来源还是目标都拒绝
        assert_eq!(r.check(110000, 2000000), Err(InheritError::NotInheritable(110000)));
        assert_eq!(r.check(2000000, 110000), Err(InheritError::NotInheritable(110000)));
        assert_eq!(r.check(50000000, 2000000), Err(InheritError::NotInheritable(50000000)));
    }
}
