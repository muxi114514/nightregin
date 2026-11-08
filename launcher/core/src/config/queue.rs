//! 改动的防抖队列（纯逻辑，时间由调用方传）：同一种改动只留最新一条，最后一次改动停顿够了才到期

use std::time::{Duration, Instant};

use super::game::SettingChange;

pub const DEBOUNCE: Duration = Duration::from_millis(300);

#[derive(Debug)]
pub struct WriteQueue {
    debounce: Duration,
    pending: Vec<SettingChange>,
    /// 最后一次 push 的时刻；写失败放回的改动不带它，等下一次 push 或 flush 再试
    last_change: Option<Instant>,
}

impl Default for WriteQueue {
    fn default() -> WriteQueue {
        WriteQueue::with_debounce(DEBOUNCE)
    }
}

impl WriteQueue {
    pub fn with_debounce(debounce: Duration) -> WriteQueue {
        WriteQueue {
            debounce,
            pending: Vec::new(),
            last_change: None,
        }
    }

    pub fn push(&mut self, change: SettingChange, now: Instant) {
        self.pending.retain(|c| !c.same_key(&change));
        self.pending.push(change);
        self.last_change = Some(now);
    }

    /// None：没有要定时写的；ZERO：已到期
    pub fn next_wait(&self, now: Instant) -> Option<Duration> {
        let last = self.last_change?;
        Some((last + self.debounce).saturating_duration_since(now))
    }

    pub fn take(&mut self) -> Vec<SettingChange> {
        self.last_change = None;
        std::mem::take(&mut self.pending)
    }

    /// 写失败的改动放回；队列里已有更新的同类改动时丢弃旧的
    pub fn put_back(&mut self, failed: Vec<SettingChange>) {
        for change in failed {
            if !self.pending.iter().any(|c| c.same_key(&change)) {
                self.pending.push(change);
            }
        }
    }

    pub fn pending(&self) -> &[SettingChange] {
        &self.pending
    }
}
