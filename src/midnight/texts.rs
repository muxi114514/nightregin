//! 午夜打开时替换的游戏文本（菜单文本分类 200）：深夜栏名与说明、三条准备文字。
//! 文本查找钩子（game::msg）可能在多个线程调用，这里只读原子量与只初始化一次的表。

use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::lang::{self, Lang};

/// GR_MenuText（取菜单文本 0x7AC800 固定传 200）
pub const MENU_CATEGORY: u32 = 200;

const ZH: [(u32, &str); 5] = [
    (131150, "午夜"),
    (131151, "所谓的“午夜”，是比深夜更深的战场。\n出击会背负诅咒，胜利能提升午夜等级。"),
    (132501, "午夜准备完毕"),
    (336902, "已完成午夜出击的准备"),
    (336903, "正在进行午夜准备……"),
];
const EN: [(u32, &str); 5] = [
    (131150, "Midnight"),
    (131151, "Midnight is a battlefield deeper than the Deep of Night.\nEvery sortie carries a curse; victories raise your Midnight level."),
    (132501, "Midnight ready"),
    (336902, "Ready for the Midnight sortie"),
    (336903, "Preparing for Midnight..."),
];

/// 栏名与说明（开关开着就换）
const ENTRY_IDS: [u32; 2] = [131150, 131151];

static ENTRY: AtomicBool = AtomicBool::new(false);
/// 准备文字（要选着深夜栏、午夜真正生效才换）
static READY: AtomicBool = AtomicBool::new(false);

pub fn set_state(entry: bool, ready: bool) {
    ENTRY.store(entry, Ordering::Release);
    READY.store(ready, Ordering::Release);
}

pub fn table(lang: Lang) -> &'static [(u32, &'static str)] {
    match lang {
        Lang::Zh => &ZH,
        Lang::En => &EN,
    }
}

/// 以 0 结尾的 UTF-16，按语言各生成一次
fn wide(lang: Lang) -> &'static [(u32, Vec<u16>)] {
    static TABLES: OnceLock<[Vec<(u32, Vec<u16>)>; 2]> = OnceLock::new();
    let build = |l| table(l).iter().map(|&(id, s)| (id, s.encode_utf16().chain([0]).collect())).collect();
    let all = TABLES.get_or_init(|| [build(Lang::Zh), build(Lang::En)]);
    &all[(lang == Lang::En) as usize]
}

fn provide_in(lang: Lang, category: u32, id: u32) -> Option<&'static [u16]> {
    let on = if ENTRY_IDS.contains(&id) { &ENTRY } else { &READY };
    if category != MENU_CATEGORY || !on.load(Ordering::Acquire) {
        return None;
    }
    wide(lang).iter().find(|(i, _)| *i == id).map(|(_, s)| s.as_slice())
}

/// 文本查找钩子的提供函数
pub fn provide(category: u32, id: u32) -> Option<&'static [u16]> {
    provide_in(lang::current(), category, id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(p: Option<&'static [u16]>) -> String {
        let s = p.expect("text");
        assert_eq!(s.last(), Some(&0), "not zero terminated");
        String::from_utf16_lossy(&s[..s.len() - 1])
    }

    /// 共用全局开关，放在一个测试里顺序执行
    #[test]
    fn replaces_only_while_active() {
        let _g = crate::midnight::test_lock();
        set_state(false, false);
        assert!(provide_in(Lang::Zh, MENU_CATEGORY, 131150).is_none());
        // 开关开着但没选深夜：栏名换，准备文字不换
        set_state(true, false);
        assert_eq!(text(provide_in(Lang::Zh, MENU_CATEGORY, 131150)), "午夜");
        assert!(provide_in(Lang::Zh, MENU_CATEGORY, 131151).is_some());
        assert!(provide_in(Lang::Zh, MENU_CATEGORY, 132501).is_none());
        set_state(true, true);
        for id in [132501, 336902, 336903] {
            assert!(provide_in(Lang::Zh, MENU_CATEGORY, id).is_some(), "{id}");
        }
        assert!(provide_in(Lang::Zh, MENU_CATEGORY, 131050).is_none());
        assert!(provide_in(Lang::Zh, 10, 131150).is_none());
        set_state(false, false);
        assert!(provide_in(Lang::Zh, MENU_CATEGORY, 132501).is_none());
    }

    #[test]
    fn english_texts_follow_language() {
        let en = table(Lang::En);
        assert_eq!(en.iter().find(|(id, _)| *id == 131150).map(|(_, s)| *s), Some("Midnight"));
        assert_eq!(en.len(), table(Lang::Zh).len());
    }
}
