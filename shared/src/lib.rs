//! DLL 与 regulation 补丁工具共用的部分：组件 SpEffect 的 ID 规则、PARAM 文件解析、字段偏移与数据提取。
//! 两边必须完全一致（补丁工具写进 regulation 的行，就是 DLL 运行时施加和读取的行），所以只在这里定义一份。
//! 武器词条升档表、随机词条池、初始装备、出击遗物开关、首领掉落改写、遗物重铸与词条继承规则只有 DLL 使用，
//! 放在这里是为了能在补丁工具的测试里用真实 regulation 核对。

pub mod boss_loot;
pub mod components;
pub mod extract;
pub mod loadouts;
pub mod midnight;
pub mod orb_refresh;
pub mod param;
pub mod passive_inherit;
pub mod passive_pool;
pub mod passive_tiers;
pub mod relic_reroll;
pub mod rune_attack;
pub mod start_relics;
pub mod tiered_beast;
pub mod tiered_cold_mirage;
pub mod tiered_downed;
pub mod tiered_flasks;
pub mod tiered_guardian;
pub mod tiered_items;
pub mod tiered_low_hp;
pub mod tiered_mixcraft;
pub mod tiered_params;
pub mod tiered_quick_forge;
pub mod tiered_refund;
pub mod tiered_runes;
pub mod tiered_sixth_sense;
pub mod tiered_weapons;
pub mod weapons;
