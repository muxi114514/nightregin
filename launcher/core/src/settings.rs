//! 启动器自己的设置 <整合包>\launcher\settings.json：键用 camelCase，不认识的键原样保留；
//! 不是合法 JSON 对象时原文件改名为 .bad 并改用默认值；单个字段不对只影响这个字段；
//! 写入走「临时文件 + rename」

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::fsutil::{atomic_write, with_suffix};
use crate::retry;
use crate::text::{compact_json_error, strip_utf8_bom};

/// 当前设置格式版本
const SCHEMA_VERSION: u32 = 1;

/// 进程内所有设置文件的读写串行执行：同一路径的临时文件只有一个名字，store 建了几个都不会互相踩。
/// 里面记着「覆盖前要先留一份 .bad」的文件：上次没读到，或损坏了却没能改名
static IO_STATE: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());

/// 游戏启动后启动器怎么办
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AfterLaunch {
    /// 隐藏到托盘
    Tray,
    /// 窗口保持不动
    Keep,
    /// 退出启动器
    Close,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub schema_version: u32,
    /// 玩家手选的游戏根目录
    pub game_dir: Option<PathBuf>,
    /// 玩家选定的存档账号（SteamID64）
    pub save_account: Option<u64>,
    pub backup_before_start: bool,
    pub after_launch: AfterLaunch,
    pub low_effects: bool,
    pub dev_mode: bool,
    /// Unix 秒；判断崩溃转储是否新出现
    pub last_launch_at: Option<i64>,
    /// 游戏启动后启动器关掉了，下次打开时补查
    pub exit_check_pending: bool,
    pub last_exit_abnormal: bool,
    /// 永夜王「已挑选」的组合（1~6 位）；切到全部 / 无再切回来时恢复
    pub everdark_picks: Option<Vec<u8>>,
    pub first_run_done: bool,
    /// 不认识的键，保存时原样写回
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            schema_version: SCHEMA_VERSION,
            game_dir: None,
            save_account: None,
            backup_before_start: true,
            after_launch: AfterLaunch::Tray,
            low_effects: false,
            dev_mode: false,
            last_launch_at: None,
            exit_check_pending: false,
            last_exit_abnormal: false,
            everdark_picks: None,
            first_run_done: false,
            extra: Map::new(),
        }
    }
}

/// 读写一个设置文件；文件被短暂占用时重试几次
pub struct SettingsStore {
    path: PathBuf,
}

impl SettingsStore {
    pub fn new(path: PathBuf) -> SettingsStore {
        SettingsStore { path }
    }

    /// 没有文件时为默认值、无提示，且不建文件。
    /// 第二项是给日志 / 界面的提示：读不了或解析失败时说明改用了默认值
    pub fn load(&self) -> (Settings, Option<String>) {
        let mut needs_backup = io_state();
        needs_backup.retain(|p| *p != self.path);
        let bytes = match retry::transient(|| fs::read(&self.path)) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return (Settings::default(), None),
            Err(e) => {
                // 原文件还在，只是没读到：下次保存覆盖它之前先留一份
                needs_backup.push(self.path.clone());
                let name = file_name(&self.bad_path());
                let note = format!(
                    "读取设置文件失败（{e}），本次使用默认设置；下次保存前会把原文件另存为 {name}"
                );
                return (Settings::default(), Some(note));
            }
        };
        let body = strip_utf8_bom(&bytes);
        match serde_json::from_slice::<Map<String, Value>>(body) {
            Ok(fields) => (lenient(fields), None),
            Err(e) => (
                Settings::default(),
                Some(self.set_aside(&e, &mut needs_backup)),
            ),
        }
    }

    /// 先建父目录，再写 <文件名>.tmp 并落盘，最后 rename 覆盖。
    /// 记着要先留一份的原文件（上次没读到，或损坏了却没能改名）先另存为 .bad；留不下来就不覆盖，返回错误
    pub fn save(&self, s: &Settings) -> io::Result<()> {
        let mut needs_backup = io_state();
        if needs_backup.contains(&self.path) {
            self.keep_original()?;
            needs_backup.retain(|p| *p != self.path);
        }
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir)?;
        }
        let bytes = serde_json::to_vec(s)?;
        atomic_write(&self.path, &bytes)
    }

    /// 把解析失败的文件改名为 <文件名>.bad（覆盖旧的），返回提示；
    /// 改名失败时记下来，下次保存覆盖前先另存
    fn set_aside(&self, e: &serde_json::Error, needs_backup: &mut Vec<PathBuf>) -> String {
        let bad = self.bad_path();
        let name = file_name(&bad);
        let detail = compact_json_error(e);
        match retry::transient(|| fs::rename(&self.path, &bad)) {
            Ok(()) => format!("设置文件损坏（{detail}），已改名为 {name}，改用默认设置"),
            Err(re) => {
                needs_backup.push(self.path.clone());
                format!(
                    "设置文件损坏（{detail}），改名为 {name} 失败（{re}），本次使用默认设置；下次保存前会另存为 {name}"
                )
            }
        }
    }

    /// 原文件另存为 .bad；文件已经不在就没什么可留
    fn keep_original(&self) -> io::Result<()> {
        match retry::transient(|| fs::copy(&self.path, self.bad_path())) {
            Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        }
    }

    fn bad_path(&self) -> PathBuf {
        with_suffix(&self.path, ".bad")
    }
}

/// 单个字段类型或取值不对时，去掉这个字段，它取自己的默认值，其余字段照常
fn lenient(mut fields: Map<String, Value>) -> Settings {
    let parse =
        |fields: Map<String, Value>| serde_json::from_value::<Settings>(Value::Object(fields));
    let bad: Vec<String> = fields
        .iter()
        .filter(|(key, value)| {
            let one = Map::from_iter([((*key).clone(), (*value).clone())]);
            parse(one).is_err()
        })
        .map(|(key, _)| key.clone())
        .collect();
    for key in &bad {
        log::warn!("设置项 {key} 的值不对，改用默认值");
        fields.remove(key);
    }
    parse(fields).unwrap_or_default()
}

/// 里面的列表只做增删，中毒时内容仍完整，照常取用
fn io_state() -> MutexGuard<'static, Vec<PathBuf>> {
    IO_STATE.lock().unwrap_or_else(PoisonError::into_inner)
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned()
}
