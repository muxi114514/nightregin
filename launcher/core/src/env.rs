//! PackEnv：启动流程（launch::run）用到的真实操作——预检、开始前备份、写配置、拼启动命令

use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::NaiveDateTime;

use crate::backup::{BackupError, BackupKind, BackupOutcome, BackupStore};
use crate::config::service::ConfigFlush;
use crate::launch::{FailReason, LaunchEnv, LaunchJob};
use crate::os::{LaunchCommand, Os, me3_command};
use crate::paths::GameDir;
use crate::preflight::{self, Issue, PackState};
use crate::saves::{SaveLocateError, nightreign_save_root, resolve};
use crate::steam::steam_id64;

/// 每次开始游戏建一个；建好后只读，启动线程里直接用
pub struct PackEnv {
    state: PackState,
    os: Arc<dyn Os>,
    flusher: Arc<dyn ConfigFlush>,
    /// 注册表 ActiveUser 原值（0 = Steam 开着但没登录）；读不到为 None
    active_user: Option<u32>,
    /// 备份文件名里的时间
    now: fn() -> NaiveDateTime,
    /// %APPDATA%\Nightreign；APPDATA 没设时为 None
    save_root: Option<PathBuf>,
}

impl PackEnv {
    pub fn new(
        state: PackState,
        os: Arc<dyn Os>,
        flusher: Arc<dyn ConfigFlush>,
        active_user: Option<u32>,
        now: fn() -> NaiveDateTime,
    ) -> PackEnv {
        PackEnv {
            state,
            os,
            flusher,
            active_user,
            now,
            save_root: nightreign_save_root(),
        }
    }

    /// 换掉存档根目录：测试指向临时目录，不去改进程全局的 APPDATA
    pub fn with_save_root(self, root: Option<PathBuf>) -> PackEnv {
        PackEnv {
            save_root: root,
            ..self
        }
    }

    /// 开始前备份用的存档根目录（只读）；测试先用它确认 with_save_root 换成功了
    pub fn save_root(&self) -> Option<&Path> {
        self.save_root.as_deref()
    }
}

impl LaunchEnv for PackEnv {
    fn preflight(&self, job: &LaunchJob) -> Result<(), Issue> {
        preflight::preflight(&self.state, &*self.os, job)
    }

    /// 关闭了开始前备份、找不到存档根目录、没登录又没有账号目录（新玩家）都是 NoSave。
    /// 备份结果由 run 记日志，这里只记前两种 NoSave 的原因
    fn backup(&self) -> Result<BackupOutcome, FailReason> {
        let settings = &self.state.settings;
        if !settings.backup_before_start {
            log::info!("已关闭开始前备份");
            return Ok(BackupOutcome::NoSave);
        }
        let Some(root) = &self.save_root else {
            log::warn!("APPDATA 环境变量缺失或不是绝对路径，找不到存档目录，不备份");
            return Ok(BackupOutcome::NoSave);
        };
        let active = self.active_user.and_then(steam_id64);
        let save = match resolve(root, self.state.savefile(), active, settings.save_account) {
            Ok(save) => save,
            Err(SaveLocateError::NoAccounts) => return Ok(BackupOutcome::NoSave),
            Err(SaveLocateError::Ambiguous(ids)) => {
                // 已是升序；SteamID64 超过 2^53，给前端转成字符串
                let accounts = ids.iter().map(u64::to_string).collect();
                return Err(FailReason::AccountUnknown { accounts });
            }
        };
        let store = BackupStore::new(&save);
        store
            .create(BackupKind::Auto, (self.now)())
            .map_err(|e| match e {
                BackupError::SaveBusy => FailReason::SaveBusy,
                // BackupError 的 Display 带「备份失败：」，界面也会加，只交内层原文
                BackupError::Io(err) => FailReason::Backup {
                    message: err.to_string(),
                },
            })
    }

    /// 界面改过的设置都写回磁盘
    fn flush_config(&self) -> Result<(), FailReason> {
        self.flusher.flush().map_err(|e| FailReason::ConfigWrite {
            message: e.to_string(),
        })
    }

    /// 找不到游戏时预检会先拦下；万一走到这里，沿用首页「找不到游戏」那条
    fn command(&self) -> Result<LaunchCommand, FailReason> {
        match &self.state.game {
            Some(game) => Ok(me3_command(&self.state.pack, game)),
            None => Err(FailReason::Preflight {
                issue: preflight::game_not_found(),
            }),
        }
    }

    fn dumps_dir(&self) -> Option<PathBuf> {
        self.state.game.as_ref().map(GameDir::dump_reports)
    }
}
