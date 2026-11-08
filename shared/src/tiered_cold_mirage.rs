//! 金6：卡莉果入侵奖励「冷冽海市蜃楼」（原版锻体 8970040）的增强版（同 tiered_runes：regulation 里只放标记，DLL 动手）。
//! 原版机制全由引擎按效果判断：身上有「可用」时受到致命伤，这次伤害作废，挂 5 秒「无敌隐身」MIRAGE_ACTIVE（状态 600）与「已用」
//! （同类别同优先级，顶掉「可用」）；赐福休息 / 复活时有「已用」就挂回「可用」。「可用 / 已用」的效果号由引擎每帧从 PlayerCommonParam 现读。
//! 拥有金6 时把这两个字段换成金6 自己的一对 MIRAGE_READY / MIRAGE_USED（shared::tiered_params）：引擎不再用掉原版的 VANILLA_READY，
//! 它在身上就表示另外拿过原版，先拿后拿都认得出（原版没有同号道具，锻体列表也找不到，没有别的办法判断）。
//! 每次触发补满血量 MIRAGE_HEAL 并挂 30 秒增益 MIRAGE_BUFF；拿过原版时多 1 次（「已用」后 DLL 挂回「可用」），赐福休息时补回

pub const GOLD_MIRAGE: i32 = 9_971_260;
pub const MIRAGE_HEAL: i32 = 9_971_261;
pub const MIRAGE_BUFF: i32 = 9_971_262;
pub const MIRAGE_READY: i32 = 9_971_263;
pub const MIRAGE_USED: i32 = 9_971_264;
pub const MIRAGE_ACTIVE: i32 = 8_970_041;
pub const VANILLA_READY: i32 = 8_970_040;
pub const VANILLA_USED: i32 = 8_970_042;
