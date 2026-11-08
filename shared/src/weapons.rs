//! 武器信息（EquipParamWeapon）：类别（是否近战）与菜单图标编号，供初始装备互换、随机词条与面板图标使用。
//! 表里只有本体行，+1~+3 由 ID 末位表示，查询时带等级的 ID 按本体查。

use std::collections::HashMap;

use crate::param::layout::equip_weapon;
use crate::param::{ParamFile, field};

/// 非近战类别：弓 / 大弓 / 弩 / 弩炮、法杖、圣印记、各类盾、火把
const NON_MELEE: [u16; 10] = [51, 53, 55, 56, 57, 61, 65, 67, 69, 87];
/// 触媒：法杖、圣印记
const CATALYSTS: [u16; 2] = [57, 61];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Entry {
    wep_type: u16,
    icon: u16,
}

#[derive(Debug, Default, Clone)]
pub struct WeaponInfo {
    weapons: HashMap<u32, Entry>,
}

impl WeaponInfo {
    pub fn build(table: &ParamFile) -> Self {
        Self::from_rows(table.rows().filter_map(|r| {
            Some((r.id, field::u16(r.data, equip_weapon::WEP_TYPE)?, field::u16(r.data, equip_weapon::ICON_ID)?))
        }))
    }

    /// rows：(武器 ID, wepType, iconId)
    pub fn from_rows(rows: impl IntoIterator<Item = (u32, u16, u16)>) -> Self {
        Self { weapons: rows.into_iter().map(|(id, wep_type, icon)| (id, Entry { wep_type, icon })).collect() }
    }

    /// 只认本体 ID
    pub fn exists(&self, id: u32) -> bool {
        self.weapons.contains_key(&id)
    }

    pub fn wep_type(&self, id: u32) -> Option<u16> {
        self.entry(id).map(|e| e.wep_type)
    }

    /// 菜单图标编号（MENU_ItemIcon_<5 位编号>）
    pub fn icon(&self, id: u32) -> Option<u16> {
        self.entry(id).map(|e| e.icon)
    }

    pub fn is_melee(&self, id: u32) -> bool {
        self.wep_type(id).is_some_and(|t| !NON_MELEE.contains(&t))
    }

    pub fn is_catalyst(&self, id: u32) -> bool {
        self.wep_type(id).is_some_and(|t| CATALYSTS.contains(&t))
    }

    fn entry(&self, id: u32) -> Option<Entry> {
        self.weapons.get(&id).or_else(|| self.weapons.get(&(id - id % 100))).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_by_type_and_base_id() {
        let w = WeaponInfo::from_rows([(3750000, 5, 10102), (30750000, 65, 12021), (33750000, 57, 11220)]);
        assert!(w.is_melee(3750000) && w.is_melee(3750002));
        assert!(!w.is_melee(30750000) && !w.is_melee(33750000));
        assert!(w.is_catalyst(33750000) && !w.is_catalyst(30750000) && !w.is_catalyst(3750000));
        assert!(w.exists(3750000) && !w.exists(3750002));
        assert_eq!(w.icon(3750002), Some(10102));
        assert_eq!(w.wep_type(12345600), None);
        assert!(!w.is_melee(12345600));
    }
}
