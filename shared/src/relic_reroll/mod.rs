//! 重铸遗物的规则：只在遗物这一栏原版可能出现的词条里抽，按原版遗物筛选的分组勾选抽取范围，花暗痕或为王之证。
//! - catalog：从参数表建立遗物、抽奖表、词条、分组的索引
//! - effects：遗物上的词条与原版排序
//! - pool：候选、同类排除与抽取
//! - price：计价

mod catalog;
mod effects;
#[cfg(test)]
mod fixture;
mod pool;
mod price;

pub use catalog::{CUSTOM_VARIANTS, Effect, Group, GroupKind, OTHER_GROUP, RelicCatalog, RelicRow};
pub use effects::{RelicEffects, SLOTS, Target};
pub use pool::{Candidate, RerollError, RerollResult, roll, total};
pub use price::{Cost, DEFAULT_PRICES, Prices};
