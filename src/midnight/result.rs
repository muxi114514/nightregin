//! 午夜胜负结算：旗标分发器把夜王击败旗标的写入记下（永夜旗标号更大，取最大即永夜优先），
//! 每帧任务按位置推进结算状态机，出击写「进行中」、回圆桌结算积分；启动时还在「进行中」= 闪退，作废。

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, OnceLock};

use log::{info, warn};
use nr_stat_alloc_shared::midnight::{Outcome, Place, Scoring, SettleEvent, Settler, boss_flag_kind, resume};

use super::progress::{Progress, ProgressStore};
use crate::game::event_flags::FlagVerdict;

/// 尚未处理的击败旗标（0 = 无）；取最大值，永夜旗标号更大所以优先
#[derive(Default)]
pub struct BossFlags(AtomicU32);

impl BossFlags {
    pub fn record(&self, id: u32, value: u8) {
        if value != 0 && boss_flag_kind(id).is_some() {
            self.0.fetch_max(id, Ordering::AcqRel);
        }
    }

    fn take(&self) -> Option<u32> {
        Some(self.0.swap(0, Ordering::AcqRel)).filter(|&id| id != 0)
    }
}

pub fn global_boss_flags() -> Arc<BossFlags> {
    static FLAGS: OnceLock<Arc<BossFlags>> = OnceLock::new();
    Arc::clone(FLAGS.get_or_init(Arc::default))
}

/// 旗标分发器的处理函数：只观察，永远放行
pub fn on_flag(id: u32, value: u8, _sync: u8) -> FlagVerdict {
    global_boss_flags().record(id, value);
    FlagVerdict::Pass
}

pub struct ResultTracker {
    store: ProgressStore,
    scoring: Scoring,
    settler: Settler,
    progress: Progress,
    flags: Arc<BossFlags>,
}

impl ResultTracker {
    pub fn new(store: ProgressStore, scoring: Scoring, flags: Arc<BossFlags>) -> Self {
        let mut progress = store.load();
        if let Some(void) = resume(progress.in_progress) {
            info!("midnight: last sortie ended without a result (crash or disconnect), voided");
            if let Err(e) = store.record(&mut progress, void, &scoring) {
                warn!("midnight: saving progress failed: {e}");
            }
        }
        Self { store, scoring, settler: Settler::new(), progress, flags }
    }

    pub fn progress(&self) -> &Progress {
        &self.progress
    }

    /// 每帧调用；返回本帧结算出的结果
    pub fn tick(&mut self, place: Option<Place>, counted: bool) -> Option<Outcome> {
        if let Some(id) = self.flags.take() {
            self.settler.on_boss_flag(id);
        }
        match self.settler.on_place(place, counted) {
            SettleEvent::None => None,
            SettleEvent::Sortied => {
                self.progress.in_progress = true;
                if let Err(e) = self.store.save(&self.progress) {
                    warn!("midnight: saving progress failed: {e}");
                }
                info!("midnight: sortie counted");
                None
            }
            SettleEvent::Settled(outcome) => {
                let before = self.progress.points;
                if let Err(e) = self.store.record(&mut self.progress, outcome, &self.scoring) {
                    warn!("midnight: saving progress failed: {e}");
                }
                info!("midnight: {outcome:?}, points {before} -> {}", self.progress.points);
                Some(outcome)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use nr_stat_alloc_shared::midnight::{Outcome, Place, Scoring};

    use super::*;
    use crate::midnight::progress::{Progress, ProgressStore};

    fn dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("nr_midnight_result_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    fn tracker(d: &PathBuf) -> (ResultTracker, Arc<BossFlags>) {
        let flags = Arc::new(BossFlags::default());
        (ResultTracker::new(ProgressStore::new(d), Scoring::default(), Arc::clone(&flags)), flags)
    }

    #[test]
    fn sortie_marks_in_progress() {
        let d = dir("sortie");
        let (mut t, _) = tracker(&d);
        t.tick(Some(Place::Hub), true);
        assert_eq!(t.tick(Some(Place::Expedition), true), None);
        assert!(ProgressStore::new(&d).load().in_progress);
    }

    #[test]
    fn win_is_recorded() {
        let d = dir("win");
        let (mut t, flags) = tracker(&d);
        t.tick(Some(Place::Hub), true);
        t.tick(Some(Place::Expedition), true);
        flags.record(151, 1);
        assert_eq!(t.tick(Some(Place::Hub), true), Some(Outcome::Win { everdark: false }));
        assert_eq!(ProgressStore::new(&d).load().points, 100);
        assert_eq!(t.progress().wins, 1);
    }

    #[test]
    fn everdark_flag_wins_over_normal() {
        let (mut t, flags) = tracker(&dir("everdark"));
        t.tick(Some(Place::Hub), true);
        t.tick(Some(Place::Expedition), true);
        flags.record(175, 1);
        flags.record(151, 1);
        assert_eq!(t.tick(Some(Place::Hub), true), Some(Outcome::Win { everdark: true }));
    }

    #[test]
    fn cleared_boss_flag_is_ignored() {
        let (mut t, flags) = tracker(&dir("cleared"));
        t.tick(Some(Place::Hub), true);
        t.tick(Some(Place::Expedition), true);
        flags.record(151, 0);
        assert_eq!(t.tick(Some(Place::Hub), true), Some(Outcome::Loss));
    }

    #[test]
    fn uncounted_sortie_writes_nothing() {
        let d = dir("uncounted");
        let (mut t, flags) = tracker(&d);
        t.tick(Some(Place::Hub), false);
        t.tick(Some(Place::Expedition), false);
        flags.record(151, 1);
        assert_eq!(t.tick(Some(Place::Hub), false), None);
        assert!(!d.join("midnight").join("progress.toml").exists());
    }

    #[test]
    fn restart_after_crash_voids() {
        let d = dir("crash");
        ProgressStore::new(&d).save(&Progress { points: 300, in_progress: true, ..Default::default() }).unwrap();
        let (t, _) = tracker(&d);
        assert_eq!((t.progress().points, t.progress().voids, t.progress().in_progress), (300, 1, false));
        assert!(!ProgressStore::new(&d).load().in_progress);
    }

    #[test]
    fn handler_never_swallows() {
        assert_eq!(on_flag(151, 1, 0), FlagVerdict::Pass);
        assert_eq!(on_flag(1_996_800_200, 1, 0), FlagVerdict::Pass);
    }
}
