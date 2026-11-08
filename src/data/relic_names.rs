//! 重铸面板显示的名称：按当前界面语言从生成的名称表（relic_text.rs）查找，查不到时为 None（界面显示编号）。

use super::relic_text::{EFFECT_NAMES, GROUP_NAMES, RELIC_NAMES};
use crate::lang;

type Table = &'static [(i32, &'static str, &'static str)];

fn lookup(table: Table, id: i32) -> Option<&'static str> {
    table.binary_search_by_key(&id, |e| e.0).ok().map(|i| lang::pick(table[i].1, table[i].2))
}

/// 词条名（参数为 AttachEffectParam 的文本 ID）
pub fn effect(text_id: i32) -> Option<&'static str> {
    lookup(EFFECT_NAMES, text_id)
}

/// 遗物名（参数为 EquipParamAntique 行号）
pub fn relic(id: u32) -> Option<&'static str> {
    lookup(RELIC_NAMES, i32::try_from(id).ok()?)
}

/// 筛选分组名（参数为 AttachEffectFilterSubCategoryParam 的文本 ID）
pub fn group(text_id: i32) -> Option<&'static str> {
    lookup(GROUP_NAMES, text_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables_are_sorted_for_binary_search() {
        for table in [EFFECT_NAMES, RELIC_NAMES, GROUP_NAMES] {
            assert!(table.windows(2).all(|w| w[0].0 < w[1].0));
        }
    }

    #[test]
    fn looks_up_known_names() {
        // 语言是进程内全局只设一次的，按当前语言比较
        assert_eq!(effect(7000002), Some(lang::pick("生命力＋３", "Vigor +3")));
        assert_eq!(relic(100), Some(lang::pick("细腻的火燃情景", "Delicate Burning Scene")));
        assert_eq!(group(288059), Some(lang::pick("送葬者", "Undertaker")));
        assert_eq!(effect(1), None);
    }
}
