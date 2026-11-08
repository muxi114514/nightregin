//! 重铸用到的参数表数据：遗物各栏的抽奖表、词条所属筛选子类与排序键、筛选子类（右侧分组）。
//! 有效权重：DLC 权重 >= 0 时用它，否则用基础权重；权重为 0 的条目不会出现。

use std::collections::{BTreeMap, HashMap, HashSet};

use crate::param::layout::{attach_effect, attach_effect_filter, attach_effect_filter_sub, attach_effect_table, equip_antique};
use crate::param::{ParamFile, field};

/// 没有筛选行的词条归入的分组
pub const OTHER_GROUP: u32 = 0;

/// 本模组新增的追踪者词条（relics crate 的 content）：不在任何抽奖表里，重铸时随模板词条出现、与模板平分权重
pub const CUSTOM_VARIANTS: [(u32, [u32; 4]); 2] =
    [(7020000, [7020050, 7020060, 7020070, 7020080]), (7010500, [7010550, 7010570, 7010580, 7010590])];

/// 筛选大类，排列顺序即界面顺序
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GroupKind {
    General,
    Hero(u8),
    Weapon,
    Debuff,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Group {
    pub id: u32,
    pub kind: GroupKind,
    /// 子类名（menu CL_MenuText），OTHER_GROUP 为 -1
    pub text_id: i32,
    pub order: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Effect {
    pub group: u32,
    pub compat: i32,
    pub sort_key: i32,
    pub text_id: i32,
    /// 常驻 SpEffect（-1 为空）
    pub passive: [i32; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RelicRow {
    pub deep: bool,
    pub color: u8,
    pub icon_id: i32,
    pub tables: [i32; 3],
    pub curse_tables: [i32; 3],
}

#[derive(Debug, Default, Clone)]
pub struct RelicCatalog {
    pub(super) relics: HashMap<u32, RelicRow>,
    /// 遗物用到的抽奖表 -> [(词条, 有效权重)]，只含权重 > 0 且词条存在的条目
    pub(super) tables: HashMap<u32, Vec<(u32, u32)>>,
    pub(super) effects: HashMap<u32, Effect>,
    pub(super) groups: BTreeMap<u32, Group>,
}

impl RelicCatalog {
    pub fn build(antique: &ParamFile, attach: &ParamFile, tables: &ParamFile, filter: &ParamFile, sub: &ParamFile, custom: bool) -> Self {
        let relics: HashMap<u32, RelicRow> = antique.rows().filter_map(|r| Some((r.id, relic_row(r.data)?))).collect();
        let groups = sub.rows().filter_map(|r| Some((r.id, group_row(r.id, r.data)?))).collect();
        let effects = attach
            .rows()
            .filter_map(|r| {
                let filter_id = field::i32(r.data, attach_effect::FILTER_PARAM)?;
                let group = u32::try_from(filter_id)
                    .ok()
                    .and_then(|id| filter.row(id))
                    .and_then(|f| field::i32(f, attach_effect_filter::SUB_CATEGORY))
                    .and_then(|s| u32::try_from(s).ok())
                    .filter(|&s| sub.row(s).is_some())
                    .unwrap_or(OTHER_GROUP);
                let passive = |k: usize| field::i32(r.data, attach_effect::PASSIVE_SP_EFFECT + k * 4);
                let effect = Effect {
                    group,
                    compat: field::i32(r.data, attach_effect::COMPATIBILITY)?,
                    sort_key: field::i32(r.data, attach_effect::SORT_KEY)?,
                    text_id: field::i32(r.data, attach_effect::ATTACH_TEXT_ID)?,
                    passive: [passive(0)?, passive(1)?, passive(2)?],
                };
                Some((r.id, effect))
            })
            .collect();
        let used: HashSet<u32> = relics.values().flat_map(|r| r.tables.into_iter().chain(r.curse_tables)).filter_map(|t| u32::try_from(t).ok()).collect();
        let entries = tables.rows().filter(|r| used.contains(&r.id)).filter_map(|r| {
            let id = u32::try_from(field::i32(r.data, attach_effect_table::ATTACH_EFFECT_ID)?).ok()?;
            let base = field::u16(r.data, attach_effect_table::CHANCE_WEIGHT)? as u32;
            let dlc = field::u16(r.data, attach_effect_table::CHANCE_WEIGHT_DLC)? as i16;
            Some((r.id, id, u32::try_from(dlc).unwrap_or(base)))
        });
        Self::from_parts(relics, entries, effects, groups, custom)
    }

    /// entries：(表 ID, 词条, 有效权重)；custom 为 true 时把 CUSTOM_VARIANTS 中存在的词条加进模板所在的表
    pub fn from_parts(
        relics: HashMap<u32, RelicRow>,
        entries: impl IntoIterator<Item = (u32, u32, u32)>,
        effects: HashMap<u32, Effect>,
        groups: BTreeMap<u32, Group>,
        custom: bool,
    ) -> Self {
        let mut tables: HashMap<u32, Vec<(u32, u32)>> = HashMap::new();
        for (table, id, weight) in entries {
            if weight > 0 && effects.contains_key(&id) {
                tables.entry(table).or_default().push((id, weight));
            }
        }
        if custom {
            for list in tables.values_mut() {
                split_custom(list, &effects);
            }
        }
        Self { relics, tables, effects, groups }
    }

    pub fn relic(&self, id: u32) -> Option<&RelicRow> {
        self.relics.get(&id)
    }

    pub fn effect(&self, id: u32) -> Option<&Effect> {
        self.effects.get(&id)
    }

    pub fn group(&self, id: u32) -> Option<&Group> {
        self.groups.get(&id)
    }

    pub fn groups(&self) -> impl Iterator<Item = &Group> {
        self.groups.values()
    }

    /// 所有遗物的图标编号（可能重复）
    pub fn icon_ids(&self) -> impl Iterator<Item = i32> + '_ {
        self.relics.values().map(|r| r.icon_id)
    }

    /// 界面排序键：大类、子类排序值、ID；OTHER_GROUP 排最后
    pub fn group_rank(&self, id: u32) -> (GroupKind, i32, u32) {
        self.groups.get(&id).map_or((GroupKind::Other, 0, id), |g| (g.kind, g.order, id))
    }

    /// 至少有一栏能抽出两种以上词条的遗物才能重铸；首领、纪念等固定词条遗物每栏只有一个候选
    pub fn rerollable(&self, relic: u32) -> bool {
        self.relics.get(&relic).is_some_and(|r| r.tables.iter().any(|&t| self.table(t).is_some_and(|e| e.len() >= 2)))
    }

    pub(super) fn table(&self, id: i32) -> Option<&[(u32, u32)]> {
        u32::try_from(id).ok().and_then(|id| self.tables.get(&id)).map(Vec::as_slice)
    }
}

fn relic_row(data: &[u8]) -> Option<RelicRow> {
    let three = |offset: usize| -> Option<[i32; 3]> {
        Some([field::i32(data, offset)?, field::i32(data, offset + 4)?, field::i32(data, offset + 8)?])
    };
    Some(RelicRow {
        deep: field::u8(data, equip_antique::DEEP_BYTE)? & 1 != 0,
        color: field::u8(data, equip_antique::COLOR)?,
        icon_id: field::i32(data, equip_antique::ICON_ID)?,
        tables: three(equip_antique::TABLES)?,
        curse_tables: three(equip_antique::CURSE_TABLES)?,
    })
}

fn group_row(id: u32, data: &[u8]) -> Option<Group> {
    let order = field::i32(data, attach_effect_filter_sub::ORDER)?;
    let kind = match field::i32(data, attach_effect_filter_sub::CATEGORY)? {
        10 => GroupKind::Hero(u8::try_from(order).ok().filter(|h| (1..=10).contains(h))?),
        20 => GroupKind::Weapon,
        30 => GroupKind::General,
        40 => GroupKind::Debuff,
        _ => return None,
    };
    Some(Group { id, kind, text_id: field::i32(data, attach_effect_filter_sub::TEXT_ID)?, order })
}

/// 模板词条的权重由模板与存在的自定义变体平分，余数留给模板
fn split_custom(list: &mut Vec<(u32, u32)>, effects: &HashMap<u32, Effect>) {
    for (template, variants) in CUSTOM_VARIANTS {
        let Some(pos) = list.iter().position(|&(id, _)| id == template) else { continue };
        let present: Vec<u32> = variants.into_iter().filter(|v| effects.contains_key(v) && list.iter().all(|&(id, _)| id != *v)).collect();
        if present.is_empty() {
            continue;
        }
        let total = list[pos].1;
        let share = total / (present.len() as u32 + 1);
        if share == 0 {
            continue;
        }
        list[pos].1 = total - share * present.len() as u32;
        list.extend(present.into_iter().map(|v| (v, share)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn effect(group: u32, compat: i32, sort_key: i32) -> Effect {
        Effect { group, compat, sort_key, text_id: 0, passive: [-1; 3] }
    }

    #[test]
    fn custom_variants_share_template_weight() {
        let effects: HashMap<u32, Effect> = [7020000, 7020050, 7020060, 7000000].into_iter().map(|id| (id, effect(10000, -1, 0))).collect();
        let entries = [(100, 7020000, 100), (100, 7000000, 50), (100, 9999999, 50), (100, 7000001, 0)];
        let c = RelicCatalog::from_parts(HashMap::new(), entries, effects.clone(), BTreeMap::new(), true);
        let mut t = c.tables[&100].clone();
        t.sort();
        // 不存在的词条与权重 0 的条目被丢掉；只有存在的两个变体参与平分，余数给模板
        assert_eq!(t, [(7000000, 50), (7020000, 34), (7020050, 33), (7020060, 33)]);
        let c = RelicCatalog::from_parts(HashMap::new(), entries, effects, BTreeMap::new(), false);
        assert_eq!(c.tables[&100].len(), 2);
    }
}
