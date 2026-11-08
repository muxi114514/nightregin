//! 预检与首页状态：读整合包现状（PackState），列首页问题，开始游戏前预检

use std::io;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::config::ConfigFile;
use crate::config::everdark::everdark_ok;
use crate::gamever::{GameVersion, dotted, read_exe_version};
use crate::ini::{IniDoc, read_ini};
use crate::launch::LaunchJob;
use crate::manifest::{ManifestInfo, read_manifest_info};
use crate::nighter::{NighterConfig, read_nighter};
use crate::os::{GAME_EXE, ME3_EXE, ME3_LAUNCHER_EXE, Os, STEAM_EXE};
use crate::paths::{GameDir, PROFILE_FILE, PackLayout, find_game_dir};
use crate::profile::{ConfigError, Me3Profile};
use crate::saves::{resolve, save_busy};
use crate::settings::Settings;
use crate::suffix::{SuffixCheck, chance_setter_ext, me3_ext, nrsc_ext};

/// .me3 没写 savefile 时的存档名
pub const DEFAULT_SAVEFILE: &str = "NR0000.co2";
/// me3 自己的文件，都在 <整合包>\bin 下
const ME3_BIN_FILES: [&str; 3] = [ME3_EXE, ME3_LAUNCHER_EXE, "me3_mod_host.dll"];
/// 缺失的文件最多列几个
const MAX_LISTED: usize = 3;
/// detail 里列表的分隔
const LIST_SEP: &str = "、";
/// 空后缀在横带上的写法
const NO_SUFFIX: &str = "（无后缀）";

/// 枚举的声明顺序就是首页横带的优先级
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum IssueKind {
    PackNotFound,
    GameNotFound,
    GameVersionMismatch,
    Me3FilesMissing,
    SaveBusy,
    SuffixMismatch,
    ChanceSetterSuffix,
    EverdarkMismatch,
    LastExitAbnormal,
    SteamNotRunning,
    GameAlreadyRunning,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Issue {
    pub kind: IssueKind,
    pub blocking: bool,
    pub detail: String,
}

/// 开始游戏或刷新首页时读到的整合包现状
pub struct PackState {
    pub pack: PackLayout,
    pub game: Option<GameDir>,
    /// 褪色燃烧.me3；读不出来时是原因
    pub profile: Result<Me3Profile, String>,
    pub manifest: Option<ManifestInfo>,
    pub nrsc: Option<IniDoc>,
    pub chance_setter: Option<IniDoc>,
    /// launcher\defaults 里 nighter.json 的永夜名单；没有时为 None
    pub nighter_factory: Option<Vec<u8>>,
    /// 存档正被别的程序占用（只有 probe_save 查过才可能为 true）
    pub save_busy: bool,
    pub nighter: Result<NighterConfig, String>,
    pub settings: Settings,
}

impl PackState {
    /// 查一次存档有没有被占用（首页用；开始游戏时备份那一步自己会查）
    pub fn probe_save(mut self, root: Option<&Path>, active: Option<u64>) -> PackState {
        self.save_busy = root
            .and_then(|root| {
                resolve(root, self.savefile(), active, self.settings.save_account).ok()
            })
            .is_some_and(|save| save_busy(&save));
        self
    }

    /// .me3 的 savefile；没写或 .me3 读不出来时用 NR0000.co2
    pub fn savefile(&self) -> &str {
        let profile = self.profile.as_ref().ok();
        profile
            .and_then(|p| p.savefile.as_deref())
            .unwrap_or(DEFAULT_SAVEFILE)
    }

    /// 游戏目录按 往上找 → settings.game_dir → steam 的顺序找；
    /// 读不出的配置记成 None / Err 交给 health_issues，本身不会失败
    pub fn load(
        pack: PackLayout,
        settings: Settings,
        steam: &dyn Fn() -> Option<PathBuf>,
    ) -> PackState {
        let game = find_game_dir(&pack.root, settings.game_dir.as_deref(), steam);
        let profile = Me3Profile::load(&pack.profile()).map_err(|e| e.to_string());
        let nighter_json = pack.nighter_json();
        let nighter = read_nighter(&nighter_json).map_err(|e| {
            warn_unless_missing(&nighter_json, &e);
            e.to_string()
        });
        PackState {
            game,
            profile,
            manifest: read_manifest_info(&pack.manifest()),
            nrsc: optional_ini(&pack.nrsc_ini()),
            chance_setter: optional_ini(&pack.chance_setter_ini()),
            nighter_factory: read_nighter(&ConfigFile::Nighter.default_path(&pack))
                .ok()
                .map(|c| c.super_lords),
            save_busy: false,
            nighter,
            settings,
            pack,
        }
    }
}

/// 三方存档后缀；.me3 读不出来时 me3 一方按未知处理
pub fn suffix_check(s: &PackState) -> SuffixCheck {
    SuffixCheck {
        me3: s.profile.as_ref().ok().and_then(me3_ext),
        nrsc: nrsc_ext(s.nrsc.as_ref()),
        chance_setter: s.chance_setter.as_ref().and_then(chance_setter_ext),
    }
}

/// 首页问题，已按优先级排序；不含 Steam / 游戏运行两项（要看进程，只在预检时查）
pub fn health_issues(s: &PackState) -> Vec<Issue> {
    let mut issues = Vec::new();
    match &s.game {
        None => issues.push(game_not_found()),
        Some(game) => issues.extend(version_issue(game, s.manifest.as_ref())),
    }
    issues.extend(me3_files_issue(s));
    issues.extend(suffix_issues(&suffix_check(s)));
    issues.extend(everdark_issue(s));
    if s.save_busy {
        issues.push(warning(IssueKind::SaveBusy, String::new()));
    }
    if s.settings.last_exit_abnormal {
        issues.push(warning(IssueKind::LastExitAbnormal, String::new()));
    }
    issues.sort_by_key(|issue| issue.kind);
    issues
}

/// nighter 只管层数，永夜名单要保持出厂值；任一方读不出时不报
fn everdark_issue(s: &PackState) -> Option<Issue> {
    let lords = &s.nighter.as_ref().ok()?.super_lords;
    let factory = s.nighter_factory.as_ref()?;
    (!everdark_ok(lords, factory)).then(|| warning(IssueKind::EverdarkMismatch, String::new()))
}

/// 依次查：Steam 没在运行、游戏已在运行、首页问题里第一个阻断的。
/// 开发模式下这次勾了「仍然启动」（job.ignore_version）时不管游戏版本
pub fn preflight(s: &PackState, os: &dyn Os, job: &LaunchJob) -> Result<(), Issue> {
    if os.find_process(STEAM_EXE).is_none() {
        return Err(blocking(IssueKind::SteamNotRunning, String::new()));
    }
    if let Some(pid) = os.find_process(GAME_EXE) {
        return Err(blocking(
            IssueKind::GameAlreadyRunning,
            format!("pid {pid}"),
        ));
    }
    let ignore_version = s.settings.dev_mode && job.ignore_version;
    let skipped = |issue: &Issue| ignore_version && issue.kind == IssueKind::GameVersionMismatch;
    health_issues(s)
        .into_iter()
        .find(|issue| issue.blocking && !skipped(issue))
        .map_or(Ok(()), Err)
}

/// 强制深夜：nighter.json 里 forceDeepNight 的 enable 与 level
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeepNight {
    pub enabled: bool,
    pub level: u8,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HomeState {
    /// 整合包版本；没有 manifest 或没写时为 None
    pub version: Option<String>,
    /// nighter.json 读不出来时为 None
    pub deep_night: Option<DeepNight>,
    pub issues: Vec<Issue>,
    pub low_effects: bool,
    pub dev_mode: bool,
}

pub fn home_state(s: &PackState) -> HomeState {
    let deep_night = s.nighter.as_ref().ok().map(|nighter| DeepNight {
        enabled: nighter.force_enable,
        level: nighter.level,
    });
    HomeState {
        version: s
            .manifest
            .as_ref()
            .and_then(|manifest| manifest.version.clone()),
        deep_night,
        issues: health_issues(s),
        low_effects: s.settings.low_effects,
        dev_mode: s.settings.dev_mode,
    }
}

/// 首页和 PackEnv::command 共用这一条
pub(crate) fn game_not_found() -> Issue {
    blocking(IssueKind::GameNotFound, String::new())
}

fn blocking(kind: IssueKind, detail: String) -> Issue {
    Issue {
        kind,
        blocking: true,
        detail,
    }
}

fn warning(kind: IssueKind, detail: String) -> Issue {
    Issue {
        kind,
        blocking: false,
        detail,
    }
}

/// manifest 列了游戏版本才查：实际版本（版本号与大小）不在其中，或读不出来，都算不符
fn version_issue(game: &GameDir, manifest: Option<&ManifestInfo>) -> Option<Issue> {
    let wanted = manifest
        .map(|m| m.game.as_slice())
        .filter(|list| !list.is_empty())?;
    let actual = match read_exe_version(&game.exe()) {
        Ok(actual) if wanted.contains(&actual) => return None,
        Ok(actual) => format!("实际 {}", version_text(&actual)),
        Err(e) => format!("读不出实际版本：{e}"),
    };
    let need: Vec<String> = wanted.iter().map(version_text).collect();
    let detail = format!("{actual}，需要 {}", need.join(LIST_SEP));
    Some(blocking(IssueKind::GameVersionMismatch, detail))
}

/// 1.3.3.0 / 118835320
fn version_text(v: &GameVersion) -> String {
    format!("{} / {}", dotted(v.file_version), v.size)
}

/// bin 下 me3 自己的文件、.me3 启用的 natives / packages 有缺失，或 .me3 读不出来；
/// 依次列前 3 项，路径相对整合包根目录；.me3 读不出来时它总占最后一项
fn me3_files_issue(s: &PackState) -> Option<Issue> {
    let root = &s.pack.root;
    let me3_exe = s.pack.me3_exe();
    let mut missing: Vec<String> = ME3_BIN_FILES
        .into_iter()
        .map(|name| me3_exe.with_file_name(name))
        .filter(|path| !path.is_file())
        .map(|path| shown(root, &path))
        .collect();
    match &s.profile {
        Ok(profile) => missing.extend(profile.missing_files().iter().map(|path| shown(root, path))),
        Err(why) => {
            // 解析错误不能被截掉（裁定 2）：bin 列表先让出最后一个名额
            missing.truncate(MAX_LISTED - 1);
            missing.push(format!("{PROFILE_FILE} {why}"));
        }
    }
    if missing.is_empty() {
        return None;
    }
    missing.truncate(MAX_LISTED);
    Some(blocking(IssueKind::Me3FilesMissing, missing.join(LIST_SEP)))
}

/// 整合包里的路径写成相对路径，不在整合包里的写完整路径
fn shown(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

/// me3 与 nrsc 不一致（阻断）；ChanceSetter 与 me3 不一致（只警告，detail 是 ChanceSetter 的后缀）。
/// 后缀都经 suffix_text 显示
fn suffix_issues(check: &SuffixCheck) -> Vec<Issue> {
    let mut issues = Vec::new();
    if check.blocking() {
        let me3 = suffix_text(check.me3.as_deref().unwrap_or_default());
        let nrsc = suffix_text(&check.nrsc);
        let detail = format!("{PROFILE_FILE} 是 {me3}，无缝联机是 {nrsc}");
        issues.push(blocking(IssueKind::SuffixMismatch, detail));
    }
    if check.warning() {
        let chance_setter = suffix_text(check.chance_setter.as_deref().unwrap_or_default());
        issues.push(warning(
            IssueKind::ChanceSetterSuffix,
            chance_setter.to_owned(),
        ));
    }
    issues
}

/// 横带上显示的后缀：照原文，空串写成「（无后缀）」
fn suffix_text(ext: &str) -> &str {
    if ext.is_empty() { NO_SUFFIX } else { ext }
}

/// 文件不存在时为 None；别的读取失败同样为 None，另记一条警告
fn optional_ini(path: &Path) -> Option<IniDoc> {
    match read_ini(path) {
        Ok(doc) => Some(doc),
        Err(e) => {
            warn_unless_missing(path, &e);
            None
        }
    }
}

/// 配置文件不存在算正常，其余读取失败记一条警告
fn warn_unless_missing(path: &Path, e: &ConfigError) {
    if !matches!(e, ConfigError::Io(err) if err.kind() == io::ErrorKind::NotFound) {
        log::warn!("{}：{e}", path.display());
    }
}
