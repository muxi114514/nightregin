//! 界面语言：启动时确定一次（配置指定，或跟随 Steam 里本游戏设置的语言），之后全局只读。
//! 面板文字、规则校验提示、保存结果都按当前语言输出；日志保持英文。

use std::path::Path;
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Zh,
    En,
}

static LANG: OnceLock<Lang> = OnceLock::new();

pub fn set(lang: Lang) {
    let _ = LANG.set(lang);
}

pub fn current() -> Lang {
    LANG.get().copied().unwrap_or(Lang::Zh)
}

/// 按当前语言二选一
pub fn pick<T>(zh: T, en: T) -> T {
    match current() {
        Lang::Zh => zh,
        Lang::En => en,
    }
}

/// 带参数的双语文字：tr!("第 {level} 级", "Level {level}")
#[macro_export]
macro_rules! tr {
    ($zh:tt, $en:tt $(,)?) => {
        match $crate::lang::current() {
            $crate::lang::Lang::Zh => format!($zh),
            $crate::lang::Lang::En => format!($en),
        }
    };
}

/// 配置值 "zh" / "en" / "auto"。auto 读 Steam 的 appmanifest_<appid>.acf 中本游戏的语言，
/// 简繁中文用中文界面，其余用英文；读不到时默认中文。返回语言与说明（写日志用）
pub fn resolve(setting: &str, game_dir: &Path) -> (Lang, String) {
    match setting.trim().to_ascii_lowercase().as_str() {
        "zh" => (Lang::Zh, "language zh (config)".into()),
        "en" => (Lang::En, "language en (config)".into()),
        _ => match steam_language(game_dir) {
            Some(s) => (if s.contains("chinese") { Lang::Zh } else { Lang::En }, format!("language auto: steam \"{s}\"")),
            None => (Lang::Zh, "language auto: steam language not found, using zh".into()),
        },
    }
}

/// game_dir 为 nightreign.exe 所在目录（steamapps/common/<游戏>/Game）
fn steam_language(game_dir: &Path) -> Option<String> {
    let appid = std::fs::read_to_string(game_dir.join("steam_appid.txt")).ok()?;
    let steamapps = game_dir.parent()?.parent()?.parent()?;
    let manifest = std::fs::read_to_string(steamapps.join(format!("appmanifest_{}.acf", appid.trim()))).ok()?;
    manifest_language(&manifest)
}

/// appmanifest 中形如 "language"		"schinese" 的一行
fn manifest_language(manifest: &str) -> Option<String> {
    manifest.lines().find_map(|line| {
        let mut parts = line.split('"').filter(|p| !p.trim().is_empty());
        (parts.next()? == "language").then(|| parts.next().map(str::to_ascii_lowercase)).flatten()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_language_from_manifest() {
        let acf = "\"AppState\"\n{\n\t\"UserConfig\"\n\t{\n\t\t\"language\"\t\t\"schinese\"\n\t}\n}";
        assert_eq!(manifest_language(acf).as_deref(), Some("schinese"));
        assert_eq!(manifest_language("\"name\" \"x\""), None);
    }

    #[test]
    fn explicit_setting_wins() {
        assert_eq!(resolve("EN", Path::new(".")).0, Lang::En);
        assert_eq!(resolve("zh", Path::new(".")).0, Lang::Zh);
    }
}
