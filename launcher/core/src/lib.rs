//! 启动器核心逻辑：找整合包与游戏、读写配置、备份存档、唤起游戏。
//! 不依赖 Tauri，全部可以单元测试；app 只做界面外壳。

pub mod backup;
pub mod config;
pub mod dumpinfo;
pub mod env;
pub mod exit_check;
pub mod firstrun;
pub mod fsutil;
pub mod gamever;
pub mod ini;
pub mod launch;
pub mod logging;
pub mod manifest;
pub mod nighter;
pub mod os;
pub mod paths;
pub mod preflight;
pub mod profile;
pub mod report;
mod retry;
pub mod savepage;
pub mod saves;
pub mod settings;
pub mod steam;
pub mod suffix;
pub mod text;
pub mod wiki;
