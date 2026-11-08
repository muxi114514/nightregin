//! 初始武器：
//! - 角色互换主手 / 副手：方案（角色 + 器皿）里的 weapons，在属性分配面板中栏选择；applier.rs 按当前器皿改写
//!   本机内存里的 CharaInitParam。武器数量不超过原版：原本单武器的角色只能换主手，两把的角色主副手不能都是近战；
//! - 出击时改战技 / 改魔法 / 附加类遗物按角色开关，随所带武器的原主人重算（relic_gates.rs）；
//! - 出征时给初始武器随机 1 条通用词条（passive.rs，配置 [starting_weapons]）。

mod applier;
mod choice;
mod passive;
pub mod pick;
mod relic_gates;

pub use applier::LoadoutApplier;
pub use passive::StartingPassives;
