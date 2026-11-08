//! 午夜模式用到的游戏地址：文本查找、出击菜单右侧面板与对话框、准备状态。

use super::{FnSpec, GameModule, resolve_fn, resolve_vtable};

/// MsgRepositoryImp 按 (分类, 编号) 取文本（rcx 仓库、edx 分类、r8d 编号 → 以 0 结尾的 UTF-16）；外层 0xE5B070 入口被改成了跳转，钩里层
const MSG_LOOKUP: FnSpec = FnSpec {
    name: "MsgRepositoryImp::lookup",
    rva: 0xE5BFD0,
    prologue: &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x74, 0x24, 0x10, 0x57, 0x48, 0x83, 0xEC, 0x20, 0x8D, 0x42, 0xFF],
    fallback: None,
};

pub fn resolve_msg_lookup(module: &GameModule) -> Option<usize> {
    resolve_fn(module, &MSG_LOOKUP)
}

/// 出击菜单右侧面板按界面名载入（rcx 面板容器、rdx 指向 {类型, …, +8 宽字符串名}），只有深夜 / 夜王面板两处调用
const PANEL_LOADER: FnSpec = FnSpec {
    name: "MatchingBossSelect::load_side_panel",
    rva: 0x9FFA20,
    prologue: &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x74, 0x24, 0x10, 0x57, 0x48, 0x81, 0xEC, 0xE0, 0x00, 0x00, 0x00],
    fallback: None,
};

/// 出击菜单的对话框（哪一个是当前显示的对话框待实机确认，三个都认）
const MATCHING_DIALOGS: [(&str, usize, &str); 3] = [
    ("MatchingDialog", 0x2C5F018, ".?AVMatchingDialog@CS@@"),
    ("MatchingBossSelectDialog", 0x2C5E408, ".?AVMatchingBossSelectDialog@CS@@"),
    ("MatchingTopDialog", 0x2C60178, ".?AVMatchingTopDialog@CS@@"),
];

/// 出击后的选角界面（训练场是另一个类 TrainingAreaHeroSelectDialog，不用揭晓诅咒）
const HERO_SELECT_DIALOG: (&str, usize, &str) = ("HeroSelectDialog", 0x2C5C8D8, ".?AVHeroSelectDialog@CS@@");

/// 午夜开关识别出击菜单用到的地址；找不到的只停用对应识别
#[derive(Debug, Clone, Default)]
pub struct MidnightMenuAddresses {
    pub panel_loader: Option<usize>,
    pub matching_dialogs: Vec<usize>,
    pub hero_select: Option<usize>,
}

pub fn resolve_midnight_menu(module: &GameModule) -> MidnightMenuAddresses {
    let (name, rva, expected) = HERO_SELECT_DIALOG;
    MidnightMenuAddresses {
        panel_loader: resolve_fn(module, &PANEL_LOADER),
        matching_dialogs: MATCHING_DIALOGS.iter().filter_map(|(name, rva, expected)| resolve_vtable(module, name, *rva, expected)).collect(),
        hero_select: resolve_vtable(module, name, rva, expected),
    }
}

/// 设置本机准备状态（rcx 准备管理对象，dl 1 = 准备 / 0 = 取消）；出击对话框按准备时传 1（0x9EA6F4），游戏自己取消准备都传 0
const SET_READY: FnSpec = FnSpec {
    name: "PartyReady::set_ready",
    rva: 0xDFA0A0,
    prologue: &[0x48, 0x89, 0x74, 0x24, 0x20, 0x57, 0x48, 0x83, 0xEC, 0x20, 0x48, 0x8B, 0xF1, 0x0F, 0xB6, 0xFA],
    fallback: None,
};
/// 出击对话框「1人出击」按钮的回调（函数对象 this 在 rcx，+8 为对话框）：单人不走 set_ready，直接调 0xB67190 开始出击
const SOLO_SORTIE: FnSpec = FnSpec {
    name: "MatchingTopDialog::solo_sortie",
    rva: 0x9EB2B0,
    prologue: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0x48, 0x8B, 0x51, 0x08, 0x48, 0x8B, 0xD9, 0x48, 0x8B, 0x8A],
    fallback: None,
};
/// 准备管理对象的静态指针（set_ready 的调用方都从这里取）
const READY_OWNER_STATIC_RVA: usize = 0x3C21F48;

/// 拦准备 / 取消准备用到的地址
#[derive(Debug, Clone, Default)]
pub struct ReadyAddresses {
    pub set_ready: Option<usize>,
    pub solo_sortie: Option<usize>,
    pub owner_static: usize,
}

pub fn resolve_ready(module: &GameModule) -> ReadyAddresses {
    ReadyAddresses { set_ready: resolve_fn(module, &SET_READY), solo_sortie: resolve_fn(module, &SOLO_SORTIE), owner_static: module.va(READY_OWNER_STATIC_RVA) }
}
