//! 武器名：按当前界面语言从生成的名称表（weapon_text.rs，含 MMV 新增武器）查找。
//! 表里只有本体 ID，带强化等级的 ID 按本体查；表外武器显示为"武器 <ID>"。

use super::weapon_text::WEAPON_NAMES;

pub fn weapon_name(id: u32) -> String {
    let base = id - id % 100;
    i32::try_from(base)
        .ok()
        .and_then(|key| WEAPON_NAMES.binary_search_by_key(&key, |e| e.0).ok())
        .map_or_else(|| crate::tr!("武器 {base}", "Weapon {base}"), |i| crate::lang::pick(WEAPON_NAMES[i].1, WEAPON_NAMES[i].2).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lang;

    #[test]
    fn table_is_sorted_for_binary_search() {
        assert!(WEAPON_NAMES.windows(2).all(|w| w[0].0 < w[1].0));
    }

    #[test]
    fn looks_up_base_id_of_reinforced_weapon() {
        assert_eq!(weapon_name(3750000), lang::pick("追踪者的大剑", "Wylder's Greatsword"));
        assert_eq!(weapon_name(9040002), lang::pick("尸山血海", "Rivers of Blood"));
        assert_eq!(weapon_name(123456700), crate::tr!("武器 123456700", "Weapon 123456700"));
    }
}
