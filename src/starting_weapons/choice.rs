//! 配置里初始武器写法的解析："执行者" = 执行者的初始主手，"守护者副手" / "Guardian off" = 守护者的初始副手，
//! "无" / "none" = 不带，纯数字 = 武器 ID。

use std::collections::BTreeMap;

use crate::stats::heroes::parse_hero;

const EMPTY: [&str; 3] = ["无", "none", "-"];

/// Ok(None) 表示不带（只对副手有意义）；defaults 为 heroId -> (默认主手, 默认副手)
pub fn resolve(token: &str, defaults: &BTreeMap<u32, (i32, i32)>) -> Result<Option<u32>, String> {
    let token = token.trim();
    if token.is_empty() || EMPTY.iter().any(|e| e.eq_ignore_ascii_case(token)) {
        return Ok(None);
    }
    if let Ok(id) = token.parse::<u32>() {
        return Ok(Some(id));
    }
    let (name, off_hand) = match off_hand_owner(token) {
        Some(owner) => (owner, true),
        None => (token, false),
    };
    let hero_id = parse_hero(name).ok_or(format!("unknown hero \"{name}\""))? as u32 + 1;
    let &(main, off) = defaults.get(&hero_id).ok_or(format!("no starting weapons for \"{name}\""))?;
    let id = if off_hand { off } else { main };
    if id < 0 {
        return Err(format!("\"{name}\" has no starting off-hand weapon"));
    }
    Ok(Some(id as u32))
}

/// "守护者副手" / "Guardian off" / "guardian-off" -> 角色名部分
fn off_hand_owner(token: &str) -> Option<&str> {
    if let Some(owner) = token.strip_suffix("副手") {
        return Some(owner.trim());
    }
    let lower = token.to_ascii_lowercase();
    lower.strip_suffix("off").filter(|rest| rest.ends_with([' ', '-', '_'])).map(|rest| token[..rest.len() - 1].trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn defaults() -> BTreeMap<u32, (i32, i32)> {
        BTreeMap::from([(1, (3750000, 30750000)), (2, (18750000, 32750000)), (8, (9750000, -1))])
    }

    #[test]
    fn resolves_hero_weapons() {
        let d = defaults();
        assert_eq!(resolve("执行者", &d), Ok(Some(9750000)));
        assert_eq!(resolve(" 守护者副手 ", &d), Ok(Some(32750000)));
        assert_eq!(resolve("Guardian off", &d), Ok(Some(32750000)));
        assert_eq!(resolve("wylder-off", &d), Ok(Some(30750000)));
        assert_eq!(resolve("无", &d), Ok(None));
        assert_eq!(resolve("None", &d), Ok(None));
        assert_eq!(resolve("41750000", &d), Ok(Some(41750000)));
        assert_eq!(resolve("追踪者", &d), Ok(Some(3750000)));
        assert!(resolve("执行者副手", &d).is_err());
        assert!(resolve("天秤", &d).is_err());
    }
}
