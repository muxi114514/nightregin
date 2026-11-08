//! 三档锻体里靠 DLL 判断时机的守护者词条（行在 relics 的 tiered::guardian）。
//! - 守护者6 SACRED：绝招期间（原版「绝招中」ULT_ACTIVE 在身上）每秒给自己挂一次圣律加成 SACRED_BUFF（1.5 秒），
//!   并施加 SACRED_SPAWN 发子弹把同样的加成挂给 15 米内队友，离开范围或绝招结束后很快消失
//! - 守护者7 ICE：技艺旋风叠加冰风暴。冰风暴事件照原版旋风复制，普通版认 ICE_SHORT 的状态、遗物延长版认 ICE_LONG 的状态，
//!   两个状态效果只能有一个：身上有延长技艺的遗物或锻体（SKILL_EXTENSIONS）就挂 ICE_LONG，否则 ICE_SHORT。
//!   原版旋风也是按遗物状态选普通 / 延长两套子弹，所以冰风暴的持续时间总与旋风一致

pub const GUARDIAN_SACRED: i32 = 9_971_835;
pub const SACRED_SPAWN: i32 = 9_971_836;
pub const SACRED_BUFF: i32 = 9_971_837;
pub const GUARDIAN_ICE: i32 = 9_971_840;
pub const ICE_SHORT: i32 = 9_971_841;
pub const ICE_LONG: i32 = 9_971_842;
/// 所有角色绝招动画期间（含守护者持续释放）挂着的「绝招中」
pub const ULT_ACTIVE: i32 = 6_999_615;
/// 守护者1：同原版遗物「延长技艺」周期挂 7011001（状态 2037）换成延长版旋风，另由 DLL 把延长版的寿命翻倍（shared::tiered_params）
pub const GUARDIAN_LONG_SKILL: i32 = 9_971_320;
/// 延长技艺：原版遗物 7011000（周期挂 7011001，状态 2037）与守护者1
pub const SKILL_EXTENSIONS: [i32; 3] = [7_011_000, 7_011_001, GUARDIAN_LONG_SKILL];
/// 延长版冰风暴的本体（蓄力 / 不蓄力），同延长版旋风一起翻倍
pub const ICE_LONG_BODIES: [u32; 2] = [900_997_126, 900_997_127];
