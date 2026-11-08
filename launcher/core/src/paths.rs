//! 定位整合包与游戏目录；整合包和游戏里各文件的位置统一从这里取

use std::path::{Path, PathBuf};

/// 整合包根目录的标志文件
pub const PROFILE_FILE: &str = "褪色燃烧.me3";
/// 游戏 exe 的文件名，也是它的进程名
pub const GAME_EXE: &str = "nightreign.exe";
/// me3 的文件名，也是它的进程名
pub const ME3_EXE: &str = "me3.exe";
/// 开发时指定整合包根目录的环境变量
pub const PACK_ROOT_ENV: &str = "NRGA_PACK_ROOT";
/// 从整合包往上找游戏根目录的最大层数
const MAX_WALK_UP: usize = 5;

/// 整合包根目录（含 褪色燃烧.me3）
#[derive(Clone, Debug, PartialEq)]
pub struct PackLayout {
    pub root: PathBuf,
}

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum PathsError {
    /// searched 按查找顺序列出找过的目录
    #[error("找不到整合包")]
    PackNotFound { searched: Vec<PathBuf> },
}

impl PackLayout {
    /// 先看 env_override 再看 exe_dir，取第一个含 褪色燃烧.me3 的目录。
    /// env_override 是相对路径时按当前目录转成绝对路径（转不了就原样用）：
    /// 启动 me3 时工作目录切到整合包根，相对的 -p / -e 会被再解析一遍而指错
    pub fn discover(exe_dir: &Path, env_override: Option<&Path>) -> Result<PackLayout, PathsError> {
        let env_dir =
            env_override.map(|dir| std::path::absolute(dir).unwrap_or_else(|_| dir.to_path_buf()));
        let searched: Vec<PathBuf> = env_dir.into_iter().chain([exe_dir.to_path_buf()]).collect();
        match searched.iter().find(|dir| dir.join(PROFILE_FILE).is_file()) {
            Some(root) => Ok(PackLayout { root: root.clone() }),
            None => Err(PathsError::PackNotFound { searched }),
        }
    }

    pub fn profile(&self) -> PathBuf {
        self.root.join(PROFILE_FILE)
    }

    pub fn me3_exe(&self) -> PathBuf {
        self.root.join("bin").join(ME3_EXE)
    }

    pub fn launcher_dir(&self) -> PathBuf {
        self.root.join("launcher")
    }

    /// 三个配置的出厂副本
    pub fn defaults_dir(&self) -> PathBuf {
        self.launcher_dir().join("defaults")
    }

    pub fn settings_file(&self) -> PathBuf {
        self.launcher_dir().join("settings.json")
    }

    pub fn logs_dir(&self) -> PathBuf {
        self.launcher_dir().join("logs")
    }

    /// 图鉴数据目录（wiki.json 和 img）
    pub fn wiki_dir(&self) -> PathBuf {
        self.launcher_dir().join("data")
    }

    pub fn manifest(&self) -> PathBuf {
        self.root.join("manifest.json")
    }

    pub fn nighter_json(&self) -> PathBuf {
        self.root.join("DepthChange").join("nighter.json")
    }

    pub fn chance_setter_ini(&self) -> PathBuf {
        self.root.join("dll").join("ChanceSetter.ini")
    }

    pub fn nrsc_ini(&self) -> PathBuf {
        self.root.join("SeamlessCoop").join("nrsc_settings.ini")
    }
}

/// 游戏根目录（含 Game\nightreign.exe）
#[derive(Clone, Debug, PartialEq)]
pub struct GameDir {
    pub root: PathBuf,
}

impl GameDir {
    pub fn exe(&self) -> PathBuf {
        game_exe(&self.root)
    }

    pub fn dump_reports(&self) -> PathBuf {
        self.root
            .join("Game")
            .join("SeamlessCoop")
            .join("crashdumps")
            .join("reports")
    }
}

fn game_exe(root: &Path) -> PathBuf {
    root.join("Game").join(GAME_EXE)
}

fn is_game_root(dir: &Path) -> bool {
    game_exe(dir).is_file()
}

/// 依次试：整合包自身及往上最多 5 级 → 玩家手选的目录 → Steam 库。
/// 手选优先于 Steam 是有意的；steam 只在前两步都落空时才调用，结果同样要含 Game\nightreign.exe
pub fn find_game_dir(
    pack_root: &Path,
    configured: Option<&Path>,
    steam: &dyn Fn() -> Option<PathBuf>,
) -> Option<GameDir> {
    pack_root
        .ancestors()
        .take(MAX_WALK_UP + 1)
        .find(|dir| is_game_root(dir))
        .or_else(|| configured.filter(|dir| is_game_root(dir)))
        .map(Path::to_path_buf)
        .or_else(|| steam().filter(|dir| is_game_root(dir)))
        .map(|root| GameDir { root })
}
