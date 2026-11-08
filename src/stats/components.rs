//! 属性组件 SpEffect：每项属性一个效果，ID 决定数值（±1..±127），所以同一时刻每项属性最多挂一个。
//! ID 规则在 nr_stat_alloc_shared 中定义（补丁工具写进 regulation 的行与这里施加的行必须一致）。
//! 组件为有限时长（租约），由本地 DLL 定期续期；续期会同步给队友，停止续期后各端自然过期。
//! 原因：游戏的施加会联机同步，但移除不会（实测），永久组件在队友端会残留。

pub use nr_stat_alloc_shared::components::{MAX_ABS, RESTORE_FP, RESTORE_HP, component_id, component_value, is_component};

use super::STAT_COUNT;

/// 差值 -> 组件 ID 集合（升序，每项属性至多一个）；超出 ±127 的部分被截断，第二个返回值表示是否发生截断
pub fn decompose(delta: &[i32; STAT_COUNT]) -> (Vec<i32>, bool) {
    let mut clamped = false;
    let mut ids: Vec<i32> = delta
        .iter()
        .enumerate()
        .filter_map(|(stat, &d)| {
            let c = d.clamp(-MAX_ABS, MAX_ABS);
            clamped |= c != d;
            component_id(stat, c)
        })
        .collect();
    ids.sort_unstable();
    (ids, clamped)
}

/// 组件集合合成的差值（用于日志与校验）
pub fn compose(ids: &[i32]) -> [i32; STAT_COUNT] {
    let mut delta = [0; STAT_COUNT];
    for &id in ids {
        if let Some((stat, v)) = component_value(id) {
            delta[stat] += v;
        }
    }
    delta
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_all_values() {
        for v in -MAX_ABS..=MAX_ABS {
            let delta = [v, -v, 0, v, 1, -1, MAX_ABS, -MAX_ABS];
            let (ids, clamped) = decompose(&delta);
            assert!(!clamped);
            assert!(ids.len() <= STAT_COUNT);
            assert_eq!(compose(&ids), delta);
        }
    }

    #[test]
    fn clamps_out_of_range() {
        let (ids, clamped) = decompose(&[200, -200, 0, 0, 0, 0, 0, 0]);
        assert!(clamped);
        assert_eq!(compose(&ids)[..2], [MAX_ABS, -MAX_ABS]);
    }
}
