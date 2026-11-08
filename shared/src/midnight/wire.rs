//! 午夜联机状态的文本编码：每个玩家把自己的状态写进 Steam 大厅的成员数据，其他成员直接读。
//! 键带版本号，版本不同的人互相当成没装模组。值 = 「在午夜队伍,携带遗物,已午夜准备,等级,诅咒编号」，如 "1,0,1,42,7"。

use super::MAX_LEVEL;

pub const LOBBY_KEY: &str = "nr_stat_alloc.midnight.v2";
pub const MAX_MEMBERS: u8 = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MemberState {
    pub in_midnight: bool,
    pub has_relics: bool,
    pub ready: bool,
    pub level: u8,
    /// 自己抽好的诅咒（队友只用房主的）；0 = 无
    pub curse: u8,
}

pub fn encode(s: &MemberState) -> String {
    format!("{},{},{},{},{}", s.in_midnight as u8, s.has_relics as u8, s.ready as u8, s.level.min(MAX_LEVEL as u8), s.curse)
}

fn flag(text: &str) -> Option<bool> {
    match text {
        "0" => Some(false),
        "1" => Some(true),
        _ => None,
    }
}

/// 格式不对返回 None（当成没装模组）
pub fn decode(text: &str) -> Option<MemberState> {
    let mut parts = text.split(',');
    let state = MemberState {
        in_midnight: flag(parts.next()?)?,
        has_relics: flag(parts.next()?)?,
        ready: flag(parts.next()?)?,
        level: parts.next()?.parse().ok().filter(|&l: &u8| u32::from(l) <= MAX_LEVEL as u32)?,
        curse: parts.next()?.parse().ok()?,
    };
    parts.next().is_none().then_some(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        for level in [0u8, 1, 64, 100] {
            for (in_midnight, has_relics, ready) in [(false, false, false), (true, false, true), (true, true, false), (false, true, true)] {
                for curse in [0u8, 1, 16] {
                    let s = MemberState { in_midnight, has_relics, ready, level, curse };
                    assert_eq!(decode(&encode(&s)), Some(s));
                }
            }
        }
    }

    #[test]
    fn format_is_stable() {
        assert_eq!(encode(&MemberState { in_midnight: true, has_relics: false, ready: true, level: 42, curse: 7 }), "1,0,1,42,7");
    }

    #[test]
    fn level_is_clamped_when_encoding() {
        assert_eq!(decode(&encode(&MemberState { level: 127, ..Default::default() })).map(|s| s.level), Some(MAX_LEVEL as u8));
    }

    #[test]
    fn malformed_text_is_rejected() {
        for bad in ["", "1", "1,0,1", "1,0,1,42", "1,0,1,42,256", "1,0,1,42,7,1", "2,0,1,42,0", "1,0,x,42,0", "1,0,1,abc,0", "1,0,1,-1,0", "1,0,1,101,0", "1,0,1,42,"] {
            assert_eq!(decode(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn key_fits_steam_limit() {
        assert!(LOBBY_KEY.len() < 255);
    }

    #[test]
    fn key_is_versioned() {
        assert!(LOBBY_KEY.ends_with(".v2"));
    }
}
