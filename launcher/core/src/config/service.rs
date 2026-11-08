//! 配置写入服务：界面的改动进防抖队列，后台线程到期写回；开始游戏和退出前 flush。
//! 暂时性的写失败留在队列里，下一次改动或 flush 再试（文件被占用的短暂失败 atomic_write 已重试过）；
//! 文件缺失、内容不对报一次就丢掉，免得一直卡住开始游戏

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use super::everdark;
use super::game::{GameSettings, SettingChange, apply_changes, overlay, read_game_settings};
use super::queue::{DEBOUNCE, WriteQueue};
use super::store::ConfigStore;
use super::{ApplyReport, ConfigFile, ConfigWriteError};
use crate::paths::PackLayout;
use crate::profile::Me3Profile;
use crate::suffix::{self, me3_ext};

/// PackEnv 用的接缝：开始游戏前确认改动都写回了
pub trait ConfigFlush: Send + Sync {
    fn flush(&self) -> Result<(), ConfigWriteError>;
}

#[derive(Debug)]
pub enum ConfigEvent {
    Applied { external_change: bool },
    WriteFailed(String),
}

type Applier =
    dyn Fn(&ConfigStore, &[SettingChange]) -> Result<ApplyReport, ConfigWriteError> + Send + Sync;

pub struct ConfigService {
    store: ConfigStore,
    queue: Mutex<WriteQueue>,
    wake: Condvar,
    /// 写盘全程持有：flush 要等后台正在写的那一批写完
    io: Mutex<()>,
    shutdown: AtomicBool,
    apply: Box<Applier>,
}

impl ConfigService {
    pub fn new(pack: PackLayout) -> Arc<ConfigService> {
        ConfigService::with_debounce(pack, DEBOUNCE)
    }

    pub fn with_debounce(pack: PackLayout, debounce: Duration) -> Arc<ConfigService> {
        Arc::new(ConfigService::build(
            pack,
            debounce,
            Box::new(apply_changes),
        ))
    }

    fn build(pack: PackLayout, debounce: Duration, apply: Box<Applier>) -> ConfigService {
        ConfigService {
            store: ConfigStore::new(pack),
            queue: Mutex::new(WriteQueue::with_debounce(debounce)),
            wake: Condvar::new(),
            io: Mutex::new(()),
            shutdown: AtomicBool::new(false),
            apply,
        }
    }

    /// 入队并唤醒后台线程，不写盘
    pub fn change(&self, change: SettingChange) {
        lock(&self.queue).push(change, Instant::now());
        self.wake.notify_all();
    }

    /// 读盘，再叠加还没写回的改动；后台正在写时等它写完
    pub fn view(&self) -> GameSettings {
        let _io = lock(&self.io);
        let mut view = read_game_settings(&self.store);
        overlay(&mut view, lock(&self.queue).pending());
        view
    }

    /// 磁盘上的配置和上次读到的不一样了（玩家手改过）
    pub fn changed_on_disk(&self) -> bool {
        let _io = lock(&self.io);
        self.store.changed_on_disk()
    }

    /// 后台线程：循环到 shutdown，到期就写，结果交给 on
    pub fn run_worker(&self, on: &dyn Fn(ConfigEvent)) {
        let mut queue = lock(&self.queue);
        while !self.shutdown.load(Ordering::SeqCst) {
            match queue.next_wait(Instant::now()) {
                None => {
                    queue = self
                        .wake
                        .wait(queue)
                        .unwrap_or_else(PoisonError::into_inner);
                }
                Some(wait) if !wait.is_zero() => {
                    queue = self
                        .wake
                        .wait_timeout(queue, wait)
                        .unwrap_or_else(PoisonError::into_inner)
                        .0;
                }
                Some(_) => {
                    drop(queue);
                    on(match self.write_pending() {
                        Ok(report) => ConfigEvent::Applied {
                            external_change: report.external_change,
                        },
                        Err(e) => {
                            log::error!("配置写回失败：{e}");
                            ConfigEvent::WriteFailed(e.to_string())
                        }
                    });
                    queue = lock(&self.queue);
                }
            }
        }
    }

    pub fn shutdown(&self) {
        // 持队列锁置位：等待中的线程不会错过这次唤醒
        let _queue = lock(&self.queue);
        self.shutdown.store(true, Ordering::SeqCst);
        self.wake.notify_all();
    }

    /// 把 nrsc 和 ChanceSetter 的后缀对齐到 .me3；先写完排队的改动
    pub fn align_suffix(&self) -> Result<ApplyReport, ConfigWriteError> {
        self.flush()?;
        let _io = lock(&self.io);
        let profile = Me3Profile::load(&self.store.pack().profile())
            .map_err(|_| ConfigWriteError::NoMe3Suffix)?;
        let ext = me3_ext(&profile).ok_or(ConfigWriteError::NoMe3Suffix)?;
        suffix::align_suffix(&self.store, &ext)
    }

    pub fn fix_everdark(&self) -> Result<ApplyReport, ConfigWriteError> {
        self.flush()?;
        let _io = lock(&self.io);
        everdark::fix_everdark(&self.store)
    }

    /// 不先 flush：要恢复的多半是读不出的文件，排队的改动写不进去
    pub fn restore_default(&self, file: ConfigFile) -> Result<(), ConfigWriteError> {
        let _io = lock(&self.io);
        self.store.restore_default(file)
    }

    fn write_pending(&self) -> Result<ApplyReport, ConfigWriteError> {
        let _io = lock(&self.io);
        let changes = lock(&self.queue).take();
        if changes.is_empty() {
            return Ok(ApplyReport::default());
        }
        (self.apply)(&self.store, &changes).inspect_err(|e| {
            if e.is_permanent() {
                log::warn!("改动写不进去，已丢弃：{changes:?}");
            } else {
                lock(&self.queue).put_back(changes.clone());
            }
        })
    }
}

impl ConfigFlush for ConfigService {
    fn flush(&self) -> Result<(), ConfigWriteError> {
        self.write_pending().map(drop)
    }
}

/// 持锁时只做赋值和文件操作，中毒时数据仍完整，照常取用
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::thread;
    use std::time::Duration;

    use tempfile::TempDir;

    use super::{ConfigEvent, ConfigFlush, ConfigService};
    use crate::config::ApplyReport;
    use crate::config::game::SettingChange;
    use crate::paths::PackLayout;

    /// 后台写到一半时 flush：要等那一批写完才返回
    #[test]
    fn flush_waits_for_inflight_write() {
        let tmp = TempDir::new().unwrap();
        let done = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&done);
        let service = Arc::new(ConfigService::build(
            PackLayout {
                root: tmp.path().to_path_buf(),
            },
            Duration::ZERO,
            Box::new(move |_, _| {
                thread::sleep(Duration::from_millis(250));
                flag.store(true, Ordering::SeqCst);
                Ok(ApplyReport::default())
            }),
        ));
        let bg = Arc::clone(&service);
        let worker = thread::spawn(move || bg.run_worker(&|_: ConfigEvent| {}));
        service.change(SettingChange::DeepNightLevel(2));
        thread::sleep(Duration::from_millis(80));
        assert!(!done.load(Ordering::SeqCst));
        service.flush().unwrap();
        assert!(done.load(Ordering::SeqCst));
        service.shutdown();
        worker.join().unwrap();
    }
}
