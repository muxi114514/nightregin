//! 金 / 紫 / 蓝三档锻体与角色专属锻体（--tiered-buffs 时才生效，目前只给不带空城的 MMV）：
//! - 锻体池同原版一样只放锻体（叶子行），不嵌套别的表：原版锻体池的行 unknown_7 都是 1，嵌套表只出现在 unknown_7 = 0 的外层表
//!   （如法师塔奖励 → 一级池）。在锻体池里嵌套表会让选项解析错（显示成池里第一个锻体、颜色沿用嵌套那层）
//! - 各档的比例放在掉落行一层，用多个条目分配（split）：原版一级池 → 蓝池 + 护符；二级、特殊、第一夜、高阶监牢的池 → 紫 75%、金 10%、
//!   本角色专属 15%；必出金 → 金色锻体 + 传说武器；MMV 的角色锻体表 → 角色专属
//! - 个别掉落改为必出金或蓝紫混合（loot.rs），罪人入侵的掉落球补一个锻体选项：紫 / 该罪人角色的专属各半（invader.rs），
//!   宝箱怪第一选项必出本角色专属、第二选项紫 / 本角色专属各半（mimic.rs）
//!   新效果与锻体行见 buffs.rs（纯参数）、scripted.rs / role_scripts.rs（靠 tools\tiered_buffs.lua 触发）、
//!   role_actions.rs（脚本改原版判定的角色锻体）、kept.rs（脚本保持挂着多个子效果）、
//!   aura.rs（给附近队友挂效果）、tae.rs（动作文件里的条件事件）、event.rs（公共事件）、dll.rs / low_hp.rs（DLL 实现的标记），
//!   params.rs（DLL 改内存参数的锻体，含可叠加的蓝色三条）、guardian.rs（守护者圣律共享与冰风暴），
//!   新武器词条（持有时战技 / 施法霸体、盾牌仇恨）见 passives.rs，
//!   SpEffect / 锻体 9971000 段，同号作文本 ID，文本在 tools\tiered_texts.ps1；候选池见 pools.rs。

pub mod aura;
pub mod buffs;
pub mod dll;
pub mod event;
pub mod guardian;
pub mod invader;
pub mod kept;
pub mod loot;
pub mod low_hp;
pub mod mimic;
pub mod params;
pub mod passives;
pub mod pools;
pub mod role_actions;
pub mod role_scripts;
pub mod rows;
pub mod scripted;
pub mod tae;

use crate::spec::RowSpec;

/// 三档锻体新增的全部行（效果、锻体、道具）
pub fn row_groups() -> impl Iterator<Item = &'static [RowSpec]> {
    [
        buffs::ROWS,
        buffs::BUFF_ROWS,
        scripted::ROWS,
        scripted::BUFF_ROWS,
        kept::BUFF_ROWS,
        aura::BUFF_ROWS,
        dll::ROWS,
        dll::BUFF_ROWS,
        tae::ROWS,
        tae::BUFF_ROWS,
        role_scripts::ROWS,
        role_scripts::BUFF_ROWS,
        role_actions::ROWS,
        role_actions::BUFF_ROWS,
        params::ROWS,
        params::BUFF_ROWS,
        guardian::BUFF_ROWS,
        event::ROWS,
        event::BUFF_ROWS,
        low_hp::ROWS,
        low_hp::BUFF_ROWS,
        passives::ROWS,
    ]
    .into_iter()
        .chain(kept::ROW_GROUPS)
        .chain(aura::ROW_GROUPS)
        .chain(guardian::ROW_GROUPS)
}

/// 新锻体池（ItemTableParam）
pub const BLUE: u32 = 4_210_000;
pub const PURPLE: u32 = 4_210_100;
pub const GOLD: u32 = 4_210_200;
/// 角色专属：每行带 onlyForClass；还没有专属锻体的角色放一份只给他抽的紫池，保证每个角色都抽得到
pub const ROLE: u32 = 4_210_300;
pub const TABLES: [u32; 4] = [BLUE, PURPLE, GOLD, ROLE];
/// heroId 1 追踪者 … 10 送葬者
pub const HEROES: std::ops::RangeInclusive<u8> = 1..=10;

/// 指定角色的专属池（罪人入侵用）：内容同该角色在 ROLE 里的行，但不设 onlyForClass，任何角色都抽得到
pub const fn hero_role(hero: u8) -> u32 {
    ROLE + hero as u32
}

pub fn is_hero_role(table: u32) -> bool {
    HEROES.into_iter().any(|h| hero_role(h) == table)
}
/// 旧版本生成过、现已不用的表，重复运行时一并删掉
pub const RETIRED: [u32; 1] = [4_210_400];

/// 物品候选：法师塔奖励用的护符表、原版传说武器表（不是锻体，挂在掉落行一层）
pub const TALISMAN_TABLE: u32 = 1_200_603;
pub const LEGENDARY_WEAPON_TABLE: u32 = 2_000_004;

/// 原版锻体池：一级 → 蓝；其余 → 紫色抽取
pub const BLUE_SOURCES: &[u32] = &[4_200_100];
pub const PURPLE_SOURCES: &[u32] = &[4_200_200, 4_200_300, 4_200_210, 4_200_220];
/// MMV 的角色锻体表，整张不再使用
pub const MMV_ROLE_TABLE: u32 = 2_000;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    Blue,
    Purple,
    Role,
    Gold,
}

/// 旧表属于哪一档
pub fn tier_of(table: u32) -> Option<Tier> {
    if BLUE_SOURCES.contains(&table) {
        Some(Tier::Blue)
    } else if PURPLE_SOURCES.contains(&table) {
        Some(Tier::Purple)
    } else if table == MMV_ROLE_TABLE {
        Some(Tier::Role)
    } else {
        None
    }
}

/// 一档拆成的条目（表, 份额）：蓝池 18 : 护符 1（原蓝池 19 个候选里护符占 1 个）；紫 750 : 金 100 : 角色 150；金色锻体 3 : 传说武器 1
fn shares(tier: Tier) -> &'static [(u32, u32)] {
    match tier {
        Tier::Blue => &[(BLUE, 18), (TALISMAN_TABLE, 1)],
        Tier::Purple => &[(PURPLE, 750), (GOLD, 100), (ROLE, 150)],
        Tier::Role => &[(ROLE, 1)],
        Tier::Gold => &[(GOLD, 3), (LEGENDARY_WEAPON_TABLE, 1)],
    }
}

/// 把一档的权重按份额拆开，余数给份额最大的一项，权重为 0 的项不要
pub fn split(tier: Tier, weight: u32) -> Vec<(u32, u32)> {
    let parts = shares(tier);
    let total: u32 = parts.iter().map(|p| p.1).sum();
    let mut out: Vec<(u32, u32)> = parts.iter().map(|&(t, s)| (t, weight * s / total)).collect();
    let rest = weight - out.iter().map(|p| p.1).sum::<u32>();
    out[0].1 += rest;
    out.retain(|p| p.1 > 0);
    out
}
