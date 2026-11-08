//! 属性规则层：与游戏内存无关的纯计算，可在宿主机上单元测试。

pub mod components;
pub mod curve;
pub mod distribute;
pub mod growth;
pub mod heroes;
pub mod reconcile;
pub mod rules;
pub mod vanilla;

pub const STAT_COUNT: usize = nr_stat_alloc_shared::components::STAT_COUNT;
pub const STAT_NAMES: [&str; STAT_COUNT] = ["Vig", "Mnd", "End", "Str", "Dex", "Int", "Fth", "Arc"];
/// 游戏中文版的属性名（与法环一致）
pub const STAT_NAMES_ZH: [&str; STAT_COUNT] = ["生命力", "集中力", "耐力", "力气", "灵巧", "智力", "信仰", "感应"];
pub const STAT_NAMES_EN: [&str; STAT_COUNT] = ["Vigor", "Mind", "Endurance", "Strength", "Dexterity", "Intelligence", "Faith", "Arcane"];

/// 当前界面语言下的属性名
pub fn stat_name(i: usize) -> &'static str {
    crate::lang::pick(STAT_NAMES_ZH[i], STAT_NAMES_EN[i])
}
