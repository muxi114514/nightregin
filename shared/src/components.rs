//! 属性组件 SpEffect 的 ID 规则：每项属性一个效果，ID 决定数值（±1..±127），同一时刻每项属性最多挂一个。
//! ID = BASE + 属性序号*1000 + (负数 ? 500 : 0) + |数值|；另有两条瞬时回满效果。

/// 属性顺序：生命 集中 耐力 力气 灵巧 智力 信仰 感应
pub const STAT_COUNT: usize = 8;
pub const BASE: i32 = 9_960_000;
pub const MAX_ABS: i32 = 127;
const STAT_STRIDE: i32 = 1000;
const NEGATIVE_OFFSET: i32 = 500;

/// 瞬时回满血量 / 专注值（只保留原版 SpEffect 110 中对应的一项）
pub const RESTORE_HP: i32 = 9_968_001;
pub const RESTORE_FP: i32 = 9_968_002;

/// 数值为 0 时没有对应组件
pub fn component_id(stat: usize, value: i32) -> Option<i32> {
    if stat >= STAT_COUNT || value == 0 || value.abs() > MAX_ABS {
        return None;
    }
    let sign = if value < 0 { NEGATIVE_OFFSET } else { 0 };
    Some(BASE + stat as i32 * STAT_STRIDE + sign + value.abs())
}

/// 组件对应的 (属性序号, 带符号数值)
pub fn component_value(id: i32) -> Option<(usize, i32)> {
    let rel = id - BASE;
    if !(0..STAT_COUNT as i32 * STAT_STRIDE).contains(&rel) {
        return None;
    }
    let (stat, within) = ((rel / STAT_STRIDE) as usize, rel % STAT_STRIDE);
    match within {
        1..=MAX_ABS => Some((stat, within)),
        w if (NEGATIVE_OFFSET + 1..=NEGATIVE_OFFSET + MAX_ABS).contains(&w) => Some((stat, -(w - NEGATIVE_OFFSET))),
        _ => None,
    }
}

pub fn is_component(id: i32) -> bool {
    component_value(id).is_some()
}

/// 全部组件 (ID, 属性序号, 带符号数值)，按 ID 升序
pub fn all() -> impl Iterator<Item = (i32, usize, i32)> {
    (0..STAT_COUNT).flat_map(|stat| {
        (1..=MAX_ABS).chain((1..=MAX_ABS).map(|v| -v)).map(move |v| (component_id(stat, v).unwrap(), stat, v))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_ranges_match_regulation() {
        assert_eq!(component_id(0, 1), Some(9_960_001));
        assert_eq!(component_id(0, 127), Some(9_960_127));
        assert_eq!(component_id(0, -1), Some(9_960_501));
        assert_eq!(component_id(7, -127), Some(9_967_627));
        assert_eq!(component_id(3, 0), None);
        assert!(!is_component(9_960_000));
        assert!(!is_component(9_960_128));
        assert!(!is_component(9_960_500));
        assert!(!is_component(RESTORE_HP));
        assert!(!is_component(7_000_000));
    }

    #[test]
    fn all_components_are_sorted_and_unique() {
        let ids: Vec<i32> = all().map(|c| c.0).collect();
        assert_eq!(ids.len(), STAT_COUNT * 2 * MAX_ABS as usize);
        assert!(ids.windows(2).all(|w| w[0] < w[1]));
        assert!(ids.iter().all(|&id| id > BASE && id < RESTORE_HP));
        assert!(all().all(|(id, stat, v)| component_value(id) == Some((stat, v))));
    }
}
