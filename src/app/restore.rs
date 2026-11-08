//! 组件变化后保持"满血 / 满专注"：变化前已满、变化后上限变了而当前值没动，就施加一次恢复效果。
//! 原因：属性组件改变上限时游戏不会调整当前值；出征落地、升级、复活时游戏先回满，我们的组件随后才变化，
//! 不处理就会出现"升完级反而不满"。变化前不满（例如战斗中）则不恢复，不会带来额外收益。

use std::time::{Duration, Instant};

/// 等待游戏刷新上限的最长时间
const WINDOW: Duration = Duration::from_secs(3);

#[derive(Debug, Clone, Copy)]
struct Watch {
    current: u32,
    max: u32,
}

#[derive(Debug, Default)]
pub struct RestoreTracker {
    player: usize,
    hp: Option<Watch>,
    fp: Option<Watch>,
    deadline: Option<Instant>,
}

impl RestoreTracker {
    /// 在增删组件之前调用，记录变化前是否满值
    pub fn arm(&mut self, player: usize, hp: (u32, u32), fp: (u32, u32)) {
        let full = |(cur, max): (u32, u32)| (max > 0 && cur >= max).then_some(Watch { current: cur, max });
        self.player = player;
        self.hp = full(hp);
        self.fp = full(fp);
        self.deadline = Some(Instant::now() + WINDOW);
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// 之后每次对账调用：返回 (是否恢复血量, 是否恢复专注)
    pub fn poll(&mut self, player: usize, hp: (u32, u32), fp: (u32, u32)) -> (bool, bool) {
        let Some(deadline) = self.deadline else { return (false, false) };
        if player != self.player || Instant::now() > deadline {
            self.reset();
            return (false, false);
        }
        let restore_hp = check(&mut self.hp, hp);
        let restore_fp = check(&mut self.fp, fp);
        if self.hp.is_none() && self.fp.is_none() {
            self.deadline = None;
        }
        (restore_hp, restore_fp)
    }
}

/// 角色刚被创建时血量字段尚未初始化，此时挂组件会让"变化前是否满值"判断失效（实测圆桌出现 220/340）。
/// 新角色需连续两次读到相同且有效的血量 / 专注后才放行，最多等待 SETTLE_TIMEOUT。
const SETTLE_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Default)]
pub struct SpawnGate {
    player: usize,
    since: Option<Instant>,
    last: Option<(u32, u32, u32, u32)>,
    open: bool,
}

impl SpawnGate {
    pub fn ready(&mut self, player: usize, hp: (u32, u32), fp: (u32, u32)) -> bool {
        if player != self.player {
            *self = Self { player, since: Some(Instant::now()), ..Default::default() };
        }
        if self.open {
            return true;
        }
        let reading = (hp.0, hp.1, fp.0, fp.1);
        let valid = hp.0 > 0 && hp.1 > 0;
        let timed_out = self.since.is_some_and(|t| t.elapsed() > SETTLE_TIMEOUT);
        if (valid && self.last == Some(reading)) || timed_out {
            self.open = true;
        }
        self.last = Some(reading);
        self.open
    }
}

/// 上限已变化时做出判定并结束观察；当前值变过（受伤、回复、上限下降被截断）则放弃
fn check(watch: &mut Option<Watch>, (cur, max): (u32, u32)) -> bool {
    let Some(w) = *watch else { return false };
    if max == w.max {
        return false;
    }
    *watch = None;
    cur == w.current && cur < max
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restores_when_full_and_max_grew() {
        let mut t = RestoreTracker::default();
        t.arm(1, (220, 220), (55, 55));
        assert_eq!(t.poll(1, (220, 220), (55, 55)), (false, false));
        assert_eq!(t.poll(1, (220, 1220), (55, 305)), (true, true));
        assert_eq!(t.poll(1, (1220, 1220), (305, 305)), (false, false));
    }

    #[test]
    fn skips_when_not_full_or_damaged() {
        let mut t = RestoreTracker::default();
        t.arm(1, (150, 220), (55, 55));
        assert_eq!(t.poll(1, (130, 1220), (55, 305)), (false, true));
        let mut t = RestoreTracker::default();
        t.arm(1, (220, 220), (55, 55));
        assert_eq!(t.poll(1, (200, 1220), (55, 55)), (false, false));
    }

    #[test]
    fn spawn_gate_waits_for_stable_vitals() {
        let mut g = SpawnGate::default();
        assert!(!g.ready(1, (0, 0), (0, 0)));
        assert!(!g.ready(1, (0, 0), (0, 0)));
        assert!(!g.ready(1, (220, 220), (55, 55)));
        assert!(g.ready(1, (220, 220), (55, 55)));
        assert!(g.ready(1, (100, 340), (55, 55)));
        // 换了新角色重新等待
        assert!(!g.ready(2, (220, 220), (55, 55)));
        assert!(g.ready(2, (220, 220), (55, 55)));
    }

    #[test]
    fn decrease_does_not_restore() {
        let mut t = RestoreTracker::default();
        t.arm(1, (1220, 1220), (305, 305));
        assert_eq!(t.poll(1, (220, 220), (55, 55)), (false, false));
    }
}
