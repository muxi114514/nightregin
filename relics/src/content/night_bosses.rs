//! 停用的夜晚首领（MMV 新增的首领场地，用户反馈有问题）。替换候选取同一夜、在 MMV 模式表里出现次数相同的那组首领，
//! 份额由它们平分（见 play_area.rs）。

use crate::play_area::BossSwap;

pub const SWAPS: &[BossSwap] = &[
    // 罹疾的盗墓人（m49_88，血条名 903050503）：只当第一夜首领
    BossSwap { boss: 4988, replacements: &[4980, 4981, 4982, 4989] },
    // “黑暗怪兽”帕尔（m49_84，血条名 903050502）：只当第二夜首领
    BossSwap { boss: 4984, replacements: &[4986, 4987] },
];
