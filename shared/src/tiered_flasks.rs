//! 三档锻体里靠 DLL 实现的圣杯瓶类词条（同 tiered_runes：regulation 里只放标记，DLL 动手）。
//! 实机确认（2026-10-31）：当前次数 = 背包里「已装」红露滴圣杯瓶的数量，喝光时条目换成「空」的编号（数量 1）；
//! 上限 = PlayerGameData+0xFF 的一个字节，赐福补充按它补。
//! - 金2 FLASK_REFILL：被救起（102190 出现）时给自己挂原版「赐福 - 补充」GRACE_RELOAD，由游戏自己补满（含把空瓶换回已装）
//! - 紫5 FLASK_FULL_ATTACK：瓶满时 DLL 补挂 0.5 秒攻击力增益 FLASK_FULL_ATTACK_BUFF
//! - 蓝3 FLASK_PLUS：可叠加，每多一份上限 +1（封顶 MAX_FLASKS），已装的瓶子也立刻 +1

pub const FLASK_REFILL: i32 = 9_971_290;
pub const FLASK_FULL_ATTACK: i32 = 9_971_196;
pub const FLASK_FULL_ATTACK_BUFF: i32 = 9_971_197;
pub const FLASK_PLUS: i32 = 9_971_030;

/// 被救起时起身动画施加的效果（a00 动画 17650 / 17651）
pub const REVIVED: i32 = 102_190;
/// 原版「赐福 - 补充」（状态 339）
pub const GRACE_RELOAD: i32 = 106;
/// 圣杯瓶道具的数量上限（EquipParamGoods 1001 的 maxNum）
pub const MAX_FLASKS: u8 = 20;
/// 红露滴圣杯瓶 +0 ~ +7：偶数为空瓶、奇数为已装
pub const RED_FLASKS: std::ops::RangeInclusive<u32> = 1000..=1015;

pub fn is_filled(item_id: u32) -> bool {
    RED_FLASKS.contains(&item_id) && item_id % 2 == 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn odd_red_flask_ids_are_filled() {
        assert!(is_filled(1001) && is_filled(1015));
        assert!(!is_filled(1000) && !is_filled(1014) && !is_filled(1051));
    }
}
