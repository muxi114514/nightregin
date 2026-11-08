//! 对账：比较"期望挂在本地角色上的组件"与"实际存在的组件"，得出要增删的列表。
//! 只做纯计算；真正的增删由调用方在游戏主线程执行。

use super::components::is_component;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Plan {
    pub add: Vec<i32>,
    pub remove: Vec<i32>,
}

impl Plan {
    pub fn is_empty(&self) -> bool {
        self.add.is_empty() && self.remove.is_empty()
    }
}

#[derive(Debug, Default)]
pub struct Reconciler {
    desired: Vec<i32>,
}

impl Reconciler {
    pub fn set_desired(&mut self, mut ids: Vec<i32>) {
        ids.sort_unstable();
        ids.dedup();
        self.desired = ids;
    }

    pub fn desired(&self) -> &[i32] {
        &self.desired
    }

    /// current 为角色身上的全部 SpEffect；非组件 ID 一律忽略，绝不触碰游戏或其它模组的效果。
    /// 重复出现的组件（异常叠加）会被全部移除后在下一轮重新补上。
    pub fn plan(&self, current: &[i32], max_ops: usize) -> Plan {
        let mut present: Vec<i32> = current.iter().copied().filter(|&id| is_component(id)).collect();
        present.sort_unstable();

        let mut plan = Plan::default();
        let mut i = 0;
        while i < present.len() {
            let id = present[i];
            let dup = i + 1 < present.len() && present[i + 1] == id;
            if dup || self.desired.binary_search(&id).is_err() {
                plan.remove.push(id);
            }
            i += 1;
            while i < present.len() && present[i] == id {
                i += 1;
            }
        }
        for &id in &self.desired {
            if present.binary_search(&id).is_err() {
                plan.add.push(id);
            }
        }
        plan.remove.truncate(max_ops);
        plan.add.truncate(max_ops.saturating_sub(plan.remove.len()));
        plan
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::components::component_id;

    fn id(stat: usize, value: i32) -> i32 {
        component_id(stat, value).unwrap()
    }

    #[test]
    fn adds_missing_and_removes_stale() {
        let mut r = Reconciler::default();
        let (a, b, stale) = (id(3, 10), id(0, -4), id(3, 9));
        r.set_desired(vec![a, b]);
        let plan = r.plan(&[7_000_000, a, stale], 64);
        assert_eq!(plan.add, vec![b]);
        assert_eq!(plan.remove, vec![stale]);
    }

    #[test]
    fn ignores_foreign_effects_and_removes_duplicates() {
        let mut r = Reconciler::default();
        let a = id(1, 20);
        r.set_desired(vec![a]);
        let plan = r.plan(&[46260, a, a, 9_100_000], 64);
        assert_eq!(plan.remove, vec![a]);
        assert!(plan.add.is_empty());
    }

    #[test]
    fn empty_desired_clears_everything() {
        let r = Reconciler::default();
        let a = id(2, -30);
        assert_eq!(r.plan(&[a], 64), Plan { add: vec![], remove: vec![a] });
    }
}
