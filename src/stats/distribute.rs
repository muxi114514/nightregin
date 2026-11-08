//! 按权重把整数点数分到各项：最大余数法 + 跨等级误差累积（carry），
//! 保证多级累计后的比例与权重一致，且不会因为并列总是偏向某一项。每项受 room（距上限的空间）约束。

/// 返回 (各项分到的点数, 因所有可分项都已到上限而没分出去的点数)。
/// carry 由调用方跨等级保存；权重为 0 或已到上限的项不参与分配。
pub fn distribute<const N: usize>(budget: u32, weights: &[u32; N], room: &[u32; N], carry: &mut [f64; N]) -> ([u32; N], u32) {
    let mut given = [0u32; N];
    let mut remaining = budget;
    // 每轮至少有一项被填满或分完，最多 N 轮
    for _ in 0..=N {
        let active: Vec<usize> = (0..N).filter(|&i| weights[i] > 0 && given[i] < room[i]).collect();
        if remaining == 0 || active.is_empty() {
            break;
        }
        let total: f64 = active.iter().map(|&i| weights[i] as f64).sum();
        let mut quota = [0f64; N];
        let mut give = [0u32; N];
        for &i in &active {
            quota[i] = remaining as f64 * weights[i] as f64 / total;
            let target = quota[i] + carry[i];
            give[i] = (target.floor().max(0.0) as u32).min(room[i] - given[i]);
        }
        // carry 为正时下取整之和可能略超预算，从小数部分最小的项回收
        while give.iter().sum::<u32>() > remaining {
            let i = *active.iter().filter(|&&i| give[i] > 0).min_by(|&&a, &&b| frac(quota[a] + carry[a]).total_cmp(&frac(quota[b] + carry[b]))).unwrap();
            give[i] -= 1;
        }
        // 剩余点数按 (配额 + 累积误差 - 已给) 从大到小逐个补
        let mut left = remaining - give.iter().sum::<u32>();
        let mut order = active.clone();
        order.sort_by(|&a, &b| {
            let (da, db) = (quota[a] + carry[a] - give[a] as f64, quota[b] + carry[b] - give[b] as f64);
            db.total_cmp(&da).then(a.cmp(&b))
        });
        for &i in order.iter().cycle().take(order.len() * 4) {
            if left == 0 {
                break;
            }
            if given[i] + give[i] < room[i] {
                give[i] += 1;
                left -= 1;
            }
        }
        for &i in &active {
            given[i] += give[i];
            carry[i] += quota[i] - give[i] as f64;
            if given[i] >= room[i] {
                carry[i] = 0.0;
            }
        }
        remaining -= active.iter().map(|&i| give[i]).sum::<u32>();
    }
    (given, remaining)
}

fn frac(x: f64) -> f64 {
    x - x.floor()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_budget_is_fully_given() {
        let mut carry = [0.0; 3];
        let (g, left) = distribute(13, &[1, 1, 1], &[100; 3], &mut carry);
        assert_eq!(g.iter().sum::<u32>(), 13);
        assert_eq!(left, 0);
    }

    #[test]
    fn equal_weights_stay_balanced_over_many_levels() {
        let mut carry = [0.0; 3];
        let mut total = [0u32; 3];
        for _ in 0..14 {
            let (g, _) = distribute(13, &[1, 1, 1], &[1000; 3], &mut carry);
            for i in 0..3 {
                total[i] += g[i];
            }
        }
        let (min, max) = (*total.iter().min().unwrap(), *total.iter().max().unwrap());
        assert!(max - min <= 1, "{total:?}");
    }

    #[test]
    fn ratio_is_respected_cumulatively() {
        let mut carry = [0.0; 2];
        let mut total = [0u32; 2];
        for budget in [9, 13, 12, 13, 13, 9, 13, 12, 13, 13] {
            let (g, _) = distribute(budget, &[3, 1], &[1000; 2], &mut carry);
            total[0] += g[0];
            total[1] += g[1];
        }
        // 120 点按 3:1 应为 90 : 30
        assert!(total[0].abs_diff(90) <= 1 && total[1].abs_diff(30) <= 1, "{total:?}");
    }

    #[test]
    fn cap_overflow_moves_to_other_stats() {
        let mut carry = [0.0; 2];
        let (g, left) = distribute(10, &[9, 1], &[3, 100], &mut carry);
        assert_eq!(g, [3, 7]);
        assert_eq!(left, 0);
    }

    #[test]
    fn leftover_when_everything_capped_or_zero_weight() {
        let mut carry = [0.0; 3];
        let (g, left) = distribute(10, &[1, 0, 1], &[2, 100, 3], &mut carry);
        assert_eq!(g, [2, 0, 3]);
        assert_eq!(left, 5);
    }
}
