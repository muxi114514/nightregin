//! 三档锻体行定义的共用部分：图标、文本、类别、模板，以及一个锻体对应「锻体行 + 道具行」的宏（buffs.rs / scripted.rs 共用）

use crate::layout::sp_effect as sp;
use crate::spec::Set;

/// 图标：道具（选项）/ 锻体 / 状态栏。原版锻体三处同号分三段（20xxx / 21xxx / 22xxx），遗物类的技艺 / 绝招图标三处同号
#[derive(Clone, Copy)]
pub struct Icon {
    pub goods: i32,
    pub buff: i32,
    pub status: i32,
}

pub const fn vanilla(n: i32) -> Icon {
    Icon { goods: 20_000 + n, buff: 21_000 + n, status: 22_000 + n }
}

pub const fn relic(id: i32) -> Icon {
    Icon { goods: id, buff: id, status: id }
}

pub const ICON_SKILL: Icon = relic(20_296);
pub const ICON_ULTIMATE: Icon = relic(20_297);
pub const ICON_SIXTH_SENSE: Icon = relic(20_300);
pub const ICON_ATTACK: Icon = vanilla(206);
pub const ICON_DODGE: Icon = vanilla(222);
pub const ICON_AGGRO: Icon = relic(20_229);
pub const ICON_NEGATION: Icon = vanilla(208);
pub const ICON_DISCOVERY: Icon = vanilla(267);
pub const ICON_HP: Icon = vanilla(291);
pub const ICON_RUNES: Icon = vanilla(267);

/// 道具行稀有度（选项框颜色）：蓝 / 紫（含角色专属）/ 金
pub const RARITY_BLUE: u8 = 1;
pub const RARITY_PURPLE: u8 = 2;
pub const RARITY_GOLD: u8 = 3;

/// 可叠加用类别 10（同原版血量 +10% 等数值锻体），不叠加用 20
pub const STACKING: u16 = 10;
pub const NON_STACKING: u16 = 20;

/// 原版锻体效果的存档类别
pub const SAVE: Set = Set::U8(sp::SAVE_CATEGORY, 10);

pub const fn text(id: u32) -> Set {
    Set::I32(sp::TEXT_ID_1, id as i32)
}

pub const fn icon(icon: Icon) -> Set {
    Set::I32(sp::ICON_ID, icon.status)
}

/// 第 i 项受到的伤害倍率
pub const fn cut(i: usize, rate: f32) -> Set {
    Set::F32(sp::DAMAGE_CUT_RATES[i], rate)
}

/// 第 i 项攻击力倍率（物理 / 魔力 / 火 / 雷 / 圣）
pub const fn attack(i: usize, rate: f32) -> Set {
    Set::F32(sp::ATTACK_RATES[i], rate)
}

/// 周期施加的效果只给自己、不同步给队友
pub const NO_SYNC: Set = Set::Bit(sp::NET_SYNC_BYTE, sp::NET_SYNC_DISABLE_BIT, true);

// 模板
pub const PERMANENT_BUFF_TEMPLATE: u32 = 8_000_000;
/// 遗物「攻击命中时，能恢复精力」：命中触发（状态 2103）→ 回复
pub const ON_HIT_TEMPLATE: u32 = 7_100_100;
pub const ON_HIT_RESTORE_TEMPLATE: u32 = 7_100_101;
/// 守护者遗物「延长技艺」：每 0.1 秒周期施加标记（不同步），拿来作「持有某状态时周期施加」的模板
pub const PERIODIC_TEMPLATE: u32 = 7_011_000;
/// 守护者「强化防御中」：只有类别 20，作纯标记（脚本按 ID 判断的锻体效果都用它）
pub const MARKER_TEMPLATE: u32 = 701_530;

// heroId
pub const WYLDER: u8 = 1;
pub const GUARDIAN: u8 = 2;
pub const IRONEYE: u8 = 3;
pub const DUCHESS: u8 = 4;
pub const RAIDER: u8 = 5;
pub const REVENANT: u8 = 6;
pub const RECLUSE: u8 = 7;
pub const EXECUTOR: u8 = 8;
pub const SCHOLAR: u8 = 9;
pub const UNDERTAKER: u8 = 10;

/// 每个锻体两行：锻体行（指向效果、不显示数值，数值写在名字里）与同号的道具行（类型 11，选项图标、稀有度，
/// 并指回锻体自身，否则游戏按模板的 8000000 当成「血量 +10%」）
macro_rules! buff_rows {
    ($(($id:expr, $sp:expr, $icon:expr, $rarity:expr, $name:literal)),* $(,)?) => {
        &[$(
            $crate::spec::RowSpec { table: &$crate::layout::PERMANENT_BUFF, template: $crate::tiered::rows::PERMANENT_BUFF_TEMPLATE, id: $id,
                name: concat!("[Tiered Buff] ", $name),
                sets: &[$crate::spec::Set::I32($crate::layout::permanent_buff::SP_EFFECT, $sp as i32),
                    $crate::spec::Set::I32($crate::layout::permanent_buff::ICON_ID, $icon.buff),
                    $crate::spec::Set::U16($crate::layout::permanent_buff::DISPLAY_NUMERIC, $crate::layout::permanent_buff::DISPLAY_NONE),
                    $crate::spec::Set::U16($crate::layout::permanent_buff::DISPLAY_PERCENT, 0),
                    $crate::spec::Set::I32($crate::layout::permanent_buff::NUMERIC_VALUE, 0)] },
            $crate::spec::RowSpec { table: &$crate::layout::EQUIP_GOODS, template: $crate::tiered::rows::PERMANENT_BUFF_TEMPLATE, id: $id,
                name: concat!("[Tiered Buff] ", $name),
                sets: &[$crate::spec::Set::U16($crate::layout::equip_goods::ICON_ID, $icon.goods as u16),
                    $crate::spec::Set::U8($crate::layout::equip_goods::RARITY, $rarity),
                    $crate::spec::Set::I32($crate::layout::equip_goods::BUFF_ID, $id as i32)] }
        ),*]
    };
}
pub(crate) use buff_rows;
