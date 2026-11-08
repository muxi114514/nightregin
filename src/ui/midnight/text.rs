//! 午夜叠加层文字（中英两套），字形收集见 ui/mod.rs。

use crate::lang::{self, Lang};

pub struct Texts {
    pub toggle_on: &'static str,
    pub toggle_off: &'static str,
    pub host_decides: &'static str,
    pub open_home: &'static str,
    pub level_prefix: &'static str,
    pub points: &'static str,
    pub relics_warning: &'static str,
    pub relics_blocked: &'static str,
    pub teammate_unmodded: &'static str,
    pub curse_title: &'static str,
    pub curse_prefix: &'static str,
    pub home_title: &'static str,
    pub record: &'static str,
    pub max_level: &'static str,
    pub silent: &'static str,
    pub silent_desc: &'static str,
    pub barren: &'static str,
    pub barren_desc: &'static str,
    pub barren_locked: &'static str,
    pub coming_soon: &'static str,
    pub close: &'static str,
    pub roster_title: &'static str,
    /// 名字显示不了时的称呼（后接序号）
    pub teammate: &'static str,
    /// 与 RosterStatus 顺序一致：Ready / NotReady / Relics / Uncounted / Unmodded
    pub status: [&'static str; 5],
    pub pad_hint: &'static str,
}

const ZH: Texts = Texts {
    toggle_on: "午夜：开",
    toggle_off: "午夜：关",
    host_decides: "午夜：由房主决定",
    open_home: "打开午夜界面",
    level_prefix: "午夜等级",
    points: "积分",
    relics_warning: "有角色带着遗物，不能午夜出击",
    relics_blocked: "午夜要求所有角色都不带遗物，请先在遗物仪式卸下",
    teammate_unmodded: "有队友没装午夜模组或版本不同，不能午夜出击",
    curse_title: "本局诅咒",
    curse_prefix: "诅咒：",
    home_title: "午夜",
    record: "胜 / 负",
    max_level: "已满级",
    silent: "静默之域",
    silent_desc: "出击随机背负 1 个诅咒，永久生效",
    barren: "荒芜之域",
    barren_desc: "按类别自选诅咒，选得越多奖励越多",
    barren_locked: "午夜 6 级解锁",
    coming_soon: "即将开放",
    close: "关闭",
    roster_title: "午夜队伍",
    teammate: "队友",
    status: ["午夜准备完毕", "未准备", "携带遗物", "本局不计午夜", "未装午夜模组"],
    pad_hint: "LB + Y 切换午夜　LB + X 打开午夜界面",
};

const EN: Texts = Texts {
    toggle_on: "Midnight: On",
    toggle_off: "Midnight: Off",
    host_decides: "Midnight: host decides",
    open_home: "Open Midnight",
    level_prefix: "Midnight Lv",
    points: "Points",
    relics_warning: "A hero has relics equipped; Midnight sortie not allowed",
    relics_blocked: "Midnight needs every hero relic-free. Unequip them at the Relic Rites first.",
    teammate_unmodded: "A teammate is missing the Midnight mod or on a different version.",
    curse_title: "This run's curse",
    curse_prefix: "Curse: ",
    home_title: "Midnight",
    record: "Wins / Losses",
    max_level: "Max level",
    silent: "Silent Domain",
    silent_desc: "Sortie with one random permanent curse",
    barren: "Barren Domain",
    barren_desc: "Pick curses by category; more curses, more rewards",
    barren_locked: "Unlocks at Midnight Lv 6",
    coming_soon: "Coming soon",
    close: "Close",
    roster_title: "Midnight party",
    teammate: "Teammate",
    status: ["Midnight ready", "Not ready", "Relics equipped", "Not counted", "No Midnight mod"],
    pad_hint: "LB + Y toggle Midnight　LB + X open Midnight",
};

pub fn tx() -> &'static Texts {
    match lang::current() {
        Lang::Zh => &ZH,
        Lang::En => &EN,
    }
}
