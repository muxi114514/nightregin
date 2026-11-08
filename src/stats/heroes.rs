//! 角色序号与名字。序号 0..10 对应 HeroParam / CharaInitParam.heroId 的 1..=10。

pub const HERO_COUNT: usize = 10;

pub const HERO_NAMES: [&str; HERO_COUNT] =
    ["Wylder", "Guardian", "Ironeye", "Duchess", "Raider", "Revenant", "Recluse", "Executor", "Scholar", "Undertaker"];

pub const HERO_NAMES_ZH: [&str; HERO_COUNT] =
    ["追踪者", "守护者", "铁之眼", "女爵", "无赖", "复仇者", "隐士", "执行者", "学者", "送葬者"];

/// 当前界面语言下的角色名
pub fn hero_name(i: usize) -> &'static str {
    crate::lang::pick(HERO_NAMES_ZH[i], HERO_NAMES[i])
}

/// 中英文名（英文不区分大小写）-> 角色序号
pub fn parse_hero(name: &str) -> Option<usize> {
    let name = name.trim();
    HERO_NAMES
        .iter()
        .position(|n| n.eq_ignore_ascii_case(name))
        .or_else(|| HERO_NAMES_ZH.iter().position(|n| *n == name))
}

/// heroId（1..=10）-> 角色序号
pub fn from_hero_id(hero_id: u32) -> Option<usize> {
    (1..=HERO_COUNT as u32).contains(&hero_id).then(|| hero_id as usize - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_both_languages() {
        assert_eq!(parse_hero("Executor"), Some(7));
        assert_eq!(parse_hero("executor"), Some(7));
        assert_eq!(parse_hero(" 执行者 "), Some(7));
        assert_eq!(parse_hero("送葬者"), Some(9));
        assert_eq!(parse_hero("Libra"), None);
        assert_eq!(from_hero_id(1), Some(0));
        assert_eq!(from_hero_id(11), None);
    }
}
