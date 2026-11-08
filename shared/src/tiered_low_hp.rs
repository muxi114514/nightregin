//! 三档锻体里「血量偏低」的两条（同 tiered_runes：regulation 里只放标记，DLL 动手）。
//! 原版所有「血量偏低时」效果（护符、遗物、武器词条）都是 conditionHp = 40：血量 40% 以下才生效。
//! - 紫 THRESHOLD_UP（类别 10 可叠加）：每份把这些效果行的判定线 +10%，最高 90%（DLL 改内存里的参数表，只在本机生效）
//! - 紫 HP_CAP：血量高于判定线时，按超出的点数施加 HP_DRAIN 的扣血效果（1、2、4 … 2048 点，按二进制组合）压回判定线之下

pub const THRESHOLD_UP: i32 = 9_971_700;
pub const HP_CAP: i32 = 9_971_710;
/// 第 i 个扣血效果扣 2^i 点
pub const HP_DRAIN_FIRST: i32 = 9_971_711;
pub const HP_DRAIN_COUNT: usize = 12;

pub const BASE_THRESHOLD: f32 = 40.0;
const STEP: f32 = 10.0;
const MAX_THRESHOLD: f32 = 90.0;
/// 敌人那边也有 conditionHp = 40 的效果（4 万段），只改玩家侧的行
const PLAYER_ROWS_FROM: u32 = 300_000;

/// 判定线（%）：每份 THRESHOLD_UP +10
pub fn threshold(ids: &[i32]) -> f32 {
    let stacks = ids.iter().filter(|&&id| id == THRESHOLD_UP).count();
    (BASE_THRESHOLD + STEP * stacks as f32).min(MAX_THRESHOLD)
}

/// 要改判定线的效果行
pub fn is_low_hp_row(id: u32, condition_hp: f32) -> bool {
    id >= PLAYER_ROWS_FROM && condition_hp == BASE_THRESHOLD
}

/// 锁血的上限：严格低于判定线（判定含不含等号没实测，低 1 点两种都满足），至少 1
pub fn cap(max_hp: u32, threshold: f32) -> u32 {
    let line = (max_hp as f64 * threshold as f64 / 100.0).ceil() as u32;
    line.saturating_sub(1).max(1)
}

/// 扣掉 excess 点要施加的扣血效果；一次最多 4095 点，剩下的下次再扣
pub fn drains(excess: u32) -> impl Iterator<Item = i32> {
    let excess = excess.min((1 << HP_DRAIN_COUNT) - 1);
    (0..HP_DRAIN_COUNT).filter(move |i| excess & (1 << i) != 0).map(|i| HP_DRAIN_FIRST + i as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn threshold_rises_per_stack_up_to_cap() {
        assert_eq!(threshold(&[]), 40.0);
        assert_eq!(threshold(&[THRESHOLD_UP, 5, THRESHOLD_UP]), 60.0);
        assert_eq!(threshold(&[THRESHOLD_UP; 9]), 90.0);
    }

    #[test]
    fn cap_stays_below_line() {
        assert_eq!(cap(1000, 40.0), 399);
        assert_eq!(cap(1001, 40.0), 400);
        assert_eq!(cap(0, 40.0), 1);
    }

    #[test]
    fn drains_add_up_to_excess() {
        let ids: Vec<i32> = drains(13).collect();
        assert_eq!(ids, vec![HP_DRAIN_FIRST, HP_DRAIN_FIRST + 2, HP_DRAIN_FIRST + 3]);
        assert_eq!(drains(10_000).count(), HP_DRAIN_COUNT);
        assert!(is_low_hp_row(8_660_000, 40.0) && !is_low_hp_row(49_136, 40.0) && !is_low_hp_row(8_660_000, 30.0));
    }
}
