//! 诊断（cargo 特性 tiered-diag）：主角内存差异。每帧给 PlayerIns / PlayerGameData 开头一段拍快照（保留最近 RING 帧），
//! 统计每个字节逐帧变化的次数；标记某个时刻后再等 AFTER 帧，比对「标记前 RING 帧」与现在，只列很少变化的位置（找内部计数 / 开关）。

use std::collections::VecDeque;

use log::info;

use crate::game::memory::read_bytes;

const REGIONS: [(&str, usize); 2] = [("PlayerIns", 0x2000), ("PlayerGameData", 0x1000)];
const RING: usize = 30;
const AFTER: u32 = 30;
/// 变化次数超过它的字节当作每帧都在动的噪声（坐标、计时器等）
const MAX_CHURN: u16 = 6;
const MAX_LINES: usize = 60;

type Snapshot = [Vec<u8>; 2];

struct Pending {
    label: String,
    frames_left: u32,
    base: Snapshot,
}

#[derive(Default)]
pub struct MemDiff {
    bases: (usize, usize),
    ring: VecDeque<Snapshot>,
    churn: [Vec<u16>; 2],
    pending: Vec<Pending>,
}

impl MemDiff {
    /// 每帧调用；主角对象换了就从头统计
    pub fn capture(&mut self, player: usize, game_data: usize) {
        if self.bases != (player, game_data) {
            self.reset();
            self.bases = (player, game_data);
        }
        let (Some(a), Some(b)) = (read_bytes(player, REGIONS[0].1), read_bytes(game_data, REGIONS[1].1)) else { return };
        let snap: Snapshot = [a, b];
        if let Some(last) = self.ring.back() {
            for (r, churn) in self.churn.iter_mut().enumerate() {
                churn.resize(snap[r].len(), 0);
                for (c, (x, y)) in churn.iter_mut().zip(last[r].iter().zip(&snap[r])) {
                    if x != y {
                        *c = c.saturating_add(1);
                    }
                }
            }
        }
        self.ring.push_back(snap);
        if self.ring.len() > RING {
            self.ring.pop_front();
        }
        let current = self.ring.back().expect("just pushed");
        let mut done = Vec::new();
        for (i, p) in self.pending.iter_mut().enumerate() {
            p.frames_left = p.frames_left.saturating_sub(1);
            if p.frames_left == 0 {
                log_diff(&p.label, &p.base, current, &self.churn);
                done.push(i);
            }
        }
        for i in done.into_iter().rev() {
            self.pending.remove(i);
        }
    }

    /// 标记一个时刻：AFTER 帧后比对它之前约 RING 帧与那时的内存
    pub fn mark(&mut self, label: String) {
        if let Some(base) = self.ring.front() {
            self.pending.push(Pending { label, frames_left: AFTER, base: base.clone() });
        }
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// 按 4 字节对齐的字逐个列出：字里有变化、且变化的字节都不常变
fn log_diff(label: &str, base: &Snapshot, now: &Snapshot, churn: &[Vec<u16>; 2]) {
    let mut lines = 0;
    for (r, (name, _)) in REGIONS.iter().enumerate() {
        for w in (0..base[r].len().min(now[r].len())).step_by(4) {
            let end = (w + 4).min(base[r].len());
            let changed: Vec<usize> = (w..end).filter(|&i| base[r][i] != now[r][i]).collect();
            if changed.is_empty() || changed.iter().any(|&i| churn[r].get(i).copied().unwrap_or(0) > MAX_CHURN) {
                continue;
            }
            if lines == MAX_LINES {
                info!("diag: diff {label}: (more omitted)");
                return;
            }
            let word = |s: &Vec<u8>| s[w..end].iter().rev().fold(0u32, |acc, &b| (acc << 8) | b as u32);
            let c = changed.iter().map(|&i| churn[r].get(i).copied().unwrap_or(0)).max().unwrap_or(0);
            info!("diag: diff {label} {name}+0x{w:x}: {:08x} -> {:08x} (churn {c})", word(&base[r]), word(&now[r]));
            lines += 1;
        }
    }
    if lines == 0 {
        info!("diag: diff {label}: no rarely-changing bytes changed");
    }
}
