//! 词条继承用到的数据：武器稀有度（档位规则）、菜单图标编号、词条文本 ID。与 GameData 同一次读取参数表时构建，游戏线程与面板共用（只读）。

use std::collections::HashMap;

use nr_stat_alloc_shared::param::layout::attach_effect;
use nr_stat_alloc_shared::param::{ParamFile, field};
use nr_stat_alloc_shared::passive_inherit::Rarities;
use nr_stat_alloc_shared::weapons::WeaponInfo;

use super::relic_names;

pub struct InheritData {
    pub rarities: Rarities,
    weapons: WeaponInfo,
    /// AttachEffectParam ID → 词条文本 ID（武器词条与遗物词条共用 AttachEffectName 文本）
    passive_texts: HashMap<u32, i32>,
}

impl InheritData {
    pub fn build(weapons: &ParamFile, attach: &ParamFile) -> Option<Self> {
        let rarities = Rarities::build(weapons);
        if rarities.is_empty() {
            return None;
        }
        let passive_texts = attach.rows().filter_map(|r| Some((r.id, field::i32(r.data, attach_effect::ATTACH_TEXT_ID)?))).filter(|&(_, t)| t > 0).collect();
        Some(Self { rarities, weapons: WeaponInfo::build(weapons), passive_texts })
    }

    /// 菜单图标编号（带强化等级的 ID 按本体查）
    pub fn icon(&self, item_id: u32) -> Option<u16> {
        self.weapons.icon(item_id)
    }

    /// 词条名；名称表里没有时显示编号
    pub fn passive_name(&self, id: i32) -> String {
        u32::try_from(id)
            .ok()
            .and_then(|id| self.passive_texts.get(&id))
            .and_then(|&text| relic_names::effect(text))
            .map_or_else(|| format!("#{id}"), |n| n.replace('\n', ""))
    }
}

#[cfg(test)]
impl InheritData {
    pub fn from_parts(rarities: Rarities, weapons: WeaponInfo, passive_texts: HashMap<u32, i32>) -> Self {
        Self { rarities, weapons, passive_texts }
    }
}
