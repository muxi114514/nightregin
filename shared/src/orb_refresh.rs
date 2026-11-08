//! 三选一选项的刷新规则：按选项原来那行掉落的权重重抽锻体，比例（如宝箱怪第二选项紫 / 专属各半）不变。
//! 只抽锻体：掉落行里的武器表、徘徊赐福等不是锻体的候选不参与，武器选项也不刷新。
//! 不会抽到这个球当前显示的词条，也不会抽到已经持有且不能叠加的词条。

use std::collections::{HashMap, HashSet};

use crate::param::layout::{item_lot, item_table, permanent_buff, sp_effect};
use crate::param::{ParamFile, field};

/// 掉落行与掉落表共用的候选类别
const CATEGORY_GOODS: i32 = 1;
const CATEGORY_TABLE: i32 = 7;
/// 物品编码的高 4 位是种类，道具为 4（同 ER）
const ITEM_KIND_MASK: u32 = 0xF000_0000;
const ITEM_KIND_GOODS: u32 = 0x4000_0000;
/// 效果类别 10：同号效果可以重复持有（叠加），其余类别重复拿到没有意义
const STACKING_CATEGORY: i16 = 10;
/// 原版外层表只嵌套一层，这里留余量并防止表互相引用
const MAX_NESTING: u32 = 4;
/// onlyForClass 为 0：不限角色
const ANY_CLASS: u8 = 0;

/// 物品编码是道具时返回道具编号
pub fn goods_id(item: u32) -> Option<u32> {
    (item & ITEM_KIND_MASK == ITEM_KIND_GOODS).then_some(item & !ITEM_KIND_MASK)
}

pub fn goods_item(id: u32) -> u32 {
    id | ITEM_KIND_GOODS
}

/// 第 used 次刷新（从 0 数）的价格；超出价格表就是次数用完
pub fn price(costs: &[u32], used: u32) -> Option<u32> {
    costs.get(used as usize).copied()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Target {
    Buff(u32),
    Table(u32),
}

#[derive(Clone, Copy, Debug)]
struct Choice {
    target: Target,
    weight: u32,
    hero: u8,
}

impl Choice {
    fn allows(&self, hero: u8) -> bool {
        self.weight > 0 && (self.hero == ANY_CLASS || self.hero == hero)
    }
}

#[derive(Clone, Copy, Debug)]
struct Buff {
    effect: i32,
    stacks: bool,
}

/// 要刷新的一个选项
#[derive(Clone, Copy, Debug)]
pub struct OptionSlot {
    /// 这个选项对应的掉落行（首行 + 选项序号）
    pub lot: u32,
    /// 当前的锻体编号
    pub buff: u32,
}

/// 启动时从参数表整理出的抽取数据，只留锻体相关的候选
#[derive(Default)]
pub struct Pools {
    /// enemy / map 两张掉落表，行号可能相同，用当前选项能否抽到来区分
    lots: Vec<HashMap<u32, Vec<Choice>>>,
    tables: HashMap<u32, Vec<Choice>>,
    buffs: HashMap<u32, Buff>,
}

impl Pools {
    /// effects：SpEffect 表，判断锻体效果能否叠加
    pub fn build(lot_files: &[ParamFile], table_file: &ParamFile, buff_file: &ParamFile, effects: &ParamFile) -> Self {
        let buffs: HashMap<u32, Buff> = buff_file
            .rows()
            .filter_map(|r| {
                let effect = field::i32(r.data, permanent_buff::SP_EFFECT)?;
                let category = effects.row(u32::try_from(effect).ok()?).and_then(|e| field::i16(e, sp_effect::SP_CATEGORY));
                Some((r.id, Buff { effect, stacks: category == Some(STACKING_CATEGORY) }))
            })
            .collect();
        let target = |category: i32, id: i32| -> Option<Target> {
            let id = u32::try_from(id).ok().filter(|&i| i > 0)?;
            match category {
                CATEGORY_GOODS if buffs.contains_key(&id) => Some(Target::Buff(id)),
                CATEGORY_TABLE => Some(Target::Table(id)),
                _ => None,
            }
        };
        let mut tables: HashMap<u32, Vec<Choice>> = HashMap::new();
        for r in table_file.rows() {
            let choice = (|| {
                let target = target(field::i32(r.data, item_table::CATEGORY)?, field::i32(r.data, item_table::ITEM_ID)?)?;
                Some(Choice { target, weight: field::u16(r.data, item_table::WEIGHT)? as u32, hero: field::u8(r.data, item_table::ONLY_FOR_CLASS)? })
            })();
            if let Some(c) = choice {
                tables.entry(r.id).or_default().push(c);
            }
        }
        let lots: Vec<HashMap<u32, Vec<Choice>>> = lot_files
            .iter()
            .map(|file| {
                file.rows()
                    .filter_map(|r| {
                        let choices: Vec<Choice> = (0..item_lot::ENTRIES)
                            .filter_map(|i| {
                                let target = target(field::i32(r.data, item_lot::CATEGORY + i * 4)?, field::i32(r.data, item_lot::ITEM_ID + i * 4)?)?;
                                Some(Choice { target, weight: field::u16(r.data, item_lot::WEIGHT + i * 2)? as u32, hero: ANY_CLASS })
                            })
                            .collect();
                        (!choices.is_empty()).then_some((r.id, choices))
                    })
                    .collect()
            })
            .collect();
        Self { lots, tables, buffs }
    }

    pub fn is_buff(&self, id: u32) -> bool {
        self.buffs.contains_key(&id)
    }

    pub fn tables_loaded(&self) -> usize {
        self.tables.len()
    }

    /// 这一行掉落（对这个角色）能抽到该锻体吗：抽不到说明对不上号（掉落行被改过等），这种选项不刷新
    pub fn drops(&self, slot: OptionSlot, hero: u8) -> bool {
        self.row_with(slot, hero).is_some()
    }

    fn row_with(&self, slot: OptionSlot, hero: u8) -> Option<&[Choice]> {
        self.lots.iter().filter_map(|t| t.get(&slot.lot)).find(|row| self.reaches(row, hero, slot.buff, 0)).map(Vec::as_slice)
    }

    fn reaches(&self, choices: &[Choice], hero: u8, buff: u32, depth: u32) -> bool {
        depth <= MAX_NESTING
            && choices.iter().filter(|c| c.allows(hero)).any(|c| match c.target {
                Target::Buff(id) => id == buff,
                Target::Table(t) => self.tables.get(&t).is_some_and(|rows| self.reaches(rows, hero, buff, depth + 1)),
            })
    }

    /// 持有且不能叠加：再抽到也没有用
    fn blocked_by_held(&self, id: u32, held: &[i32]) -> bool {
        self.buffs.get(&id).is_some_and(|b| !b.stacks && held.contains(&b.effect))
    }

    /// 给每个选项抽一个新锻体（与 slots 一一对应，抽不到的为 None）；新结果之间也不重复
    pub fn plan(&self, slots: &[OptionSlot], others: &[u32], hero: u8, held: &[i32], random: &mut impl FnMut() -> u64) -> Vec<Option<u32>> {
        let mut used: HashSet<u32> = slots.iter().map(|s| s.buff).chain(others.iter().copied()).collect();
        slots
            .iter()
            .map(|slot| {
                let row = self.row_with(*slot, hero)?;
                let new = self.pick(row, hero, &|id| used.contains(&id) || self.blocked_by_held(id, held), random, 0)?;
                used.insert(new);
                Some(new)
            })
            .collect()
    }

    fn has_any(&self, choice: &Choice, hero: u8, banned: &impl Fn(u32) -> bool, depth: u32) -> bool {
        match choice.target {
            Target::Buff(id) => !banned(id),
            Target::Table(t) => {
                depth < MAX_NESTING && self.tables.get(&t).is_some_and(|rows| rows.iter().any(|c| c.allows(hero) && self.has_any(c, hero, banned, depth + 1)))
            }
        }
    }

    /// 按权重选一项；选中嵌套表时再往下选。抽不到的项不占权重
    fn pick(&self, choices: &[Choice], hero: u8, banned: &impl Fn(u32) -> bool, random: &mut impl FnMut() -> u64, depth: u32) -> Option<u32> {
        let open: Vec<&Choice> = choices.iter().filter(|c| c.allows(hero) && self.has_any(c, hero, banned, depth)).collect();
        let total: u64 = open.iter().map(|c| c.weight as u64).sum();
        if total == 0 {
            return None;
        }
        let mut roll = random() % total;
        for c in open {
            if roll < c.weight as u64 {
                return match c.target {
                    Target::Buff(id) => Some(id),
                    Target::Table(t) => self.pick(self.tables.get(&t)?, hero, banned, random, depth + 1),
                };
            }
            roll -= c.weight as u64;
        }
        None
    }

    #[cfg(test)]
    fn fixture(lots: Vec<HashMap<u32, Vec<Choice>>>, tables: HashMap<u32, Vec<Choice>>, buffs: HashMap<u32, Buff>) -> Self {
        Self { lots, tables, buffs }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOT: u32 = 6_900_001;
    const PURPLE: u32 = 4_210_100;
    const ROLE: u32 = 4_210_300;
    const EXECUTOR: u8 = 8;
    const HERMIT: u8 = 7;

    fn buff(id: u32, hero: u8) -> Choice {
        Choice { target: Target::Buff(id), weight: 100, hero }
    }

    fn table(id: u32, weight: u32) -> Choice {
        Choice { target: Target::Table(id), weight, hero: ANY_CLASS }
    }

    /// 紫池 3 条（1~3）、角色池：执行者 2 条（11、12）、隐士 1 条（21）；效果号 = 编号，3 可叠加
    fn pools() -> Pools {
        let tables = HashMap::from([
            (PURPLE, vec![buff(1, 0), buff(2, 0), buff(3, 0)]),
            (ROLE, vec![buff(11, EXECUTOR), buff(12, EXECUTOR), buff(21, HERMIT)]),
        ]);
        let lots = vec![HashMap::from([(LOT, vec![table(PURPLE, 500), table(ROLE, 500)]), (LOT + 1, vec![table(ROLE, 1000)])])];
        let buffs = [1, 2, 3, 11, 12, 21].map(|id| (id, Buff { effect: id as i32, stacks: id == 3 })).into();
        Pools::fixture(lots, tables, buffs)
    }

    fn counter() -> impl FnMut() -> u64 {
        let mut n = 0u64;
        move || {
            n = n.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            n >> 16
        }
    }

    #[test]
    fn price_follows_the_table_and_ends_with_it() {
        let costs = [0, 10_000, 50_000];
        assert_eq!([price(&costs, 0), price(&costs, 1), price(&costs, 2), price(&costs, 3)], [Some(0), Some(10_000), Some(50_000), None]);
    }

    #[test]
    fn goods_encoding_round_trips_and_rejects_weapons() {
        assert_eq!(goods_id(0x4098_2830), Some(9_971_760));
        assert_eq!(goods_item(9_971_760), 0x4098_2830);
        assert_eq!(goods_id(0x0089_A260), None);
    }

    #[test]
    fn keeps_the_lot_ratio_and_only_draws_for_this_hero() {
        let p = pools();
        let mut rng = counter();
        let (mut purple, mut role) = (0, 0);
        for _ in 0..2000 {
            match p.plan(&[OptionSlot { lot: LOT, buff: 1 }], &[], EXECUTOR, &[], &mut rng)[0] {
                Some(2 | 3) => purple += 1,
                Some(11 | 12) => role += 1,
                other => panic!("unexpected {other:?}"),
            }
        }
        assert!((800..1200).contains(&purple), "purple {purple}, role {role}");
    }

    #[test]
    fn never_repeats_what_the_orb_shows_or_just_drew() {
        let p = pools();
        let mut rng = counter();
        for _ in 0..200 {
            let slots = [OptionSlot { lot: LOT + 1, buff: 11 }, OptionSlot { lot: LOT, buff: 1 }];
            let plan = p.plan(&slots, &[], EXECUTOR, &[], &mut rng);
            // 第一个选项的专属只剩 12；第二个抽到的不会是 12、11、1
            assert_eq!(plan[0], Some(12));
            assert!(matches!(plan[1], Some(2 | 3)), "{plan:?}");
        }
    }

    #[test]
    fn skips_held_buffs_unless_they_stack() {
        let p = pools();
        let mut rng = counter();
        // 2 持有不能叠加，3 持有但能叠加
        for _ in 0..50 {
            let plan = p.plan(&[OptionSlot { lot: LOT, buff: 1 }], &[11, 12], EXECUTOR, &[2, 3], &mut rng);
            assert_eq!(plan[0], Some(3));
        }
    }

    #[test]
    fn empty_pool_leaves_the_option_alone() {
        let p = pools();
        // 隐士只有一条专属，当前显示的就是它
        let plan = p.plan(&[OptionSlot { lot: LOT + 1, buff: 21 }], &[], HERMIT, &[], &mut counter());
        assert_eq!(plan, [None]);
    }

    #[test]
    fn option_must_belong_to_its_lot() {
        let p = pools();
        assert!(p.drops(OptionSlot { lot: LOT, buff: 2 }, EXECUTOR));
        assert!(!p.drops(OptionSlot { lot: LOT + 1, buff: 2 }, EXECUTOR));
        assert!(!p.drops(OptionSlot { lot: LOT + 1, buff: 21 }, EXECUTOR));
        assert!(!p.drops(OptionSlot { lot: 1, buff: 2 }, EXECUTOR));
        assert_eq!(p.plan(&[OptionSlot { lot: LOT + 1, buff: 2 }], &[], EXECUTOR, &[], &mut counter()), [None]);
    }

    #[test]
    fn follows_nested_tables_and_survives_loops() {
        let mut p = pools();
        const OUTER: u32 = 1_201_600;
        p.tables.insert(OUTER, vec![table(PURPLE, 100), table(OUTER, 100)]);
        p.lots[0].insert(5, vec![table(OUTER, 1000)]);
        let mut rng = counter();
        for _ in 0..50 {
            let new = p.plan(&[OptionSlot { lot: 5, buff: 1 }], &[], EXECUTOR, &[], &mut rng)[0];
            assert!(matches!(new, Some(2 | 3) | None), "{new:?}");
        }
    }
}
