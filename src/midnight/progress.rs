//! 午夜进度存档 StatAlloc\midnight\progress.toml：积分、胜负场数、最近一局、出击中标记。
//! 只在游戏主线程读写；先写临时文件再改名，写到一半崩溃也不会弄坏原文件。

use std::cell::Cell;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use log::warn;
use nr_stat_alloc_shared::midnight::{Outcome, Scoring, apply};
use serde::{Deserialize, Serialize};

const DIR: &str = "midnight";
const FILE: &str = "progress.toml";

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Progress {
    pub points: u64,
    pub wins: u32,
    pub losses: u32,
    pub voids: u32,
    /// 最近一局：win / everdark / loss / void
    pub last: Option<String>,
    /// 出击后置位、结算后清除；启动时还在 = 上次闪退或掉线
    pub in_progress: bool,
    /// 累计货币（天赋树用）
    pub currency: u64,
}

pub struct ProgressStore {
    path: PathBuf,
    /// 原文件读不出又没能留底时为 false：不能拿默认值覆盖掉它
    writable: Cell<bool>,
}

impl ProgressStore {
    /// dir = DLL 所在目录（StatAlloc）
    pub fn new(dir: &Path) -> Self {
        Self { path: dir.join(DIR).join(FILE), writable: Cell::new(true) }
    }

    #[cfg(test)]
    pub fn can_save(&self) -> bool {
        self.writable.get()
    }

    /// 文件不存在为默认值；内容坏了（非 UTF-8 或解析失败）改名 .bad 留底再从默认值开始；
    /// 其他读取错误（被占用、没权限）也从默认值开始，但本次运行不再写文件，免得覆盖真实进度
    pub fn load(&self) -> Progress {
        let text = match std::fs::read_to_string(&self.path) {
            Ok(text) => text,
            Err(e) if e.kind() == ErrorKind::NotFound => return Progress::default(),
            Err(e) if e.kind() == ErrorKind::InvalidData => return self.reset_corrupt(&e.to_string()),
            Err(e) => {
                self.writable.set(false);
                warn!("midnight: progress file unreadable ({e}); using 0 and not saving this session");
                return Progress::default();
            }
        };
        toml::from_str(&text).unwrap_or_else(|e| self.reset_corrupt(&e.to_string()))
    }

    fn reset_corrupt(&self, why: &str) -> Progress {
        let kept = std::fs::rename(&self.path, self.path.with_extension("toml.bad")).is_ok();
        if !kept {
            self.writable.set(false);
        }
        warn!("midnight: progress file corrupt ({why}), reset to 0; original kept as .bad: {kept}");
        Progress::default()
    }

    pub fn save(&self, p: &Progress) -> std::io::Result<()> {
        if !self.writable.get() {
            return Err(std::io::Error::other("progress file could not be read; not overwriting it"));
        }
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let text = toml::to_string(p).map_err(std::io::Error::other)?;
        let tmp = self.path.with_extension("toml.tmp");
        std::fs::write(&tmp, text)?;
        std::fs::rename(&tmp, &self.path)
    }

    /// 结算一局：改积分与场数、清出击中标记并保存
    pub fn record(&self, p: &mut Progress, outcome: Outcome, scoring: &Scoring) -> std::io::Result<()> {
        p.points = apply(p.points, scoring.delta(outcome));
        p.currency = p.currency.saturating_add(scoring.currency(outcome));
        let last = match outcome {
            Outcome::Win { everdark } => {
                p.wins += 1;
                if everdark { "everdark" } else { "win" }
            }
            Outcome::Loss => {
                p.losses += 1;
                "loss"
            }
            Outcome::Void => {
                p.voids += 1;
                "void"
            }
        };
        p.last = Some(last.into());
        p.in_progress = false;
        self.save(p)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nr_stat_alloc_shared::midnight::{Outcome, Scoring};

    fn dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("nr_midnight_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn missing_file_is_default() {
        let store = ProgressStore::new(&dir("missing"));
        assert_eq!(store.load(), Progress::default());
    }

    #[test]
    fn save_then_load_roundtrip() {
        let store = ProgressStore::new(&dir("roundtrip"));
        let p = Progress { points: 730, wins: 7, losses: 1, voids: 2, last: Some("win".into()), in_progress: true, currency: 25 };
        store.save(&p).unwrap();
        assert_eq!(store.load(), p);
    }

    #[test]
    fn save_leaves_no_tmp() {
        let d = dir("tmp");
        let store = ProgressStore::new(&d);
        store.save(&Progress::default()).unwrap();
        let names: Vec<_> = std::fs::read_dir(d.join("midnight")).unwrap().map(|e| e.unwrap().file_name()).collect();
        assert_eq!(names, vec![std::ffi::OsString::from("progress.toml")]);
    }

    #[test]
    fn corrupt_file_is_kept_and_reset() {
        let d = dir("corrupt");
        std::fs::create_dir_all(d.join("midnight")).unwrap();
        std::fs::write(d.join("midnight").join("progress.toml"), "points = \"abc\"").unwrap();
        let store = ProgressStore::new(&d);
        assert_eq!(store.load(), Progress::default());
        assert!(d.join("midnight").join("progress.toml.bad").exists());
        assert!(!d.join("midnight").join("progress.toml").exists());
    }

    #[test]
    fn non_utf8_file_is_kept_and_reset() {
        let d = dir("gbk");
        std::fs::create_dir_all(d.join("midnight")).unwrap();
        // 记事本另存为 ANSI 后带中文注释：GBK 字节不是合法 UTF-8
        std::fs::write(d.join("midnight").join("progress.toml"), b"# \xd6\xd0\xce\xc4\npoints = 730\n").unwrap();
        let store = ProgressStore::new(&d);
        assert_eq!(store.load(), Progress::default());
        assert!(d.join("midnight").join("progress.toml.bad").exists());
        assert!(store.save(&Progress::default()).is_ok());
    }

    #[test]
    fn unreadable_file_is_never_overwritten() {
        let d = dir("unreadable");
        // 同名目录：读取报的不是「不存在」
        std::fs::create_dir_all(d.join("midnight").join("progress.toml")).unwrap();
        let store = ProgressStore::new(&d);
        assert!(store.can_save());
        assert_eq!(store.load(), Progress::default());
        assert!(!store.can_save());
        assert!(store.save(&Progress { points: 5, ..Default::default() }).is_err());
        assert!(d.join("midnight").join("progress.toml").is_dir());
    }

    #[test]
    fn record_win_and_loss() {
        let store = ProgressStore::new(&dir("record"));
        let mut p = Progress { in_progress: true, ..Default::default() };
        store.record(&mut p, Outcome::Win { everdark: false }, &Scoring::default()).unwrap();
        assert_eq!((p.points, p.wins, p.in_progress), (100, 1, false));
        p.in_progress = true;
        store.record(&mut p, Outcome::Loss, &Scoring::default()).unwrap();
        assert_eq!((p.points, p.losses, p.in_progress), (50, 1, false));
        assert_eq!(store.load(), p);
    }

    #[test]
    fn record_adds_currency() {
        let store = ProgressStore::new(&dir("currency"));
        let mut p = Progress::default();
        store.record(&mut p, Outcome::Win { everdark: false }, &Scoring::default()).unwrap();
        store.record(&mut p, Outcome::Win { everdark: true }, &Scoring::default()).unwrap();
        store.record(&mut p, Outcome::Loss, &Scoring::default()).unwrap();
        assert_eq!(p.currency, 25);
        assert_eq!(store.load().currency, 25);
    }

    /// 旧版本写的进度文件没有货币字段，要能读出来
    #[test]
    fn old_progress_file_has_no_currency() {
        let d = dir("oldfile");
        std::fs::create_dir_all(d.join("midnight")).unwrap();
        std::fs::write(d.join("midnight").join("progress.toml"), "points = 730\nwins = 7\n").unwrap();
        let p = ProgressStore::new(&d).load();
        assert_eq!((p.points, p.wins, p.currency), (730, 7, 0));
    }

    #[test]
    fn record_void_keeps_points() {
        let store = ProgressStore::new(&dir("void"));
        let mut p = Progress { points: 300, in_progress: true, ..Default::default() };
        store.record(&mut p, Outcome::Void, &Scoring::default()).unwrap();
        assert_eq!((p.points, p.voids, p.in_progress), (300, 1, false));
    }
}
