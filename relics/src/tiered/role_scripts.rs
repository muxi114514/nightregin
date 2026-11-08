//! 第五批（脚本）：由 tools\tiered_buffs.lua 判断原版效果 / 状态再施加的角色锻体。改 ID 要同步改脚本。
//! - 女爵1：原版遗物 7500300 在技艺时挂 0.4 秒无敌 7500301；带锻体时脚本再挂一份 0.8 秒的
//! - 送葬者1：共鸣（免费绝招 709300，4 秒）自然结束时再给 3 秒；放绝招用掉的不补
//! - 隐士3：每吸收一次属性痕（706000~706003 出现）属性攻击力 ×1.005，最多 127 次；次数按二进制拆成 7 个效果，死亡不移除
//! （执行者7 妖刀附加全异常原本也在这里用附魔做，妖刀不吃附魔，已改到 tae.rs 用动作事件）

use crate::layout::{SP_EFFECT, sp_effect as sp};
use crate::spec::{RowSpec, Set};

use super::pools::RoleBuff;
use super::rows::{DUCHESS, ICON_ATTACK, ICON_SKILL, ICON_ULTIMATE, MARKER_TEMPLATE, RARITY_PURPLE, RECLUSE, SAVE, UNDERTAKER, buff_rows, icon, text};

// 女爵1：遗物给的技艺无敌时间 0.4 → 0.8 秒
const DUCHESS_LONG_DODGE: u32 = 9_971_625;
const DUCHESS_LONG_DODGE_EXTRA: u32 = 9_971_626;
// 送葬者1：共鸣多 3 秒
const UNDERTAKER_LONG_RESONANCE: u32 = 9_971_630;
const UNDERTAKER_LONG_RESONANCE_TIMER: u32 = 9_971_631;
// 隐士3：吸收属性痕叠属性攻击力
const RECLUSE_RESIDUE_STACK: u32 = 9_971_635;
const RECLUSE_RESIDUE_BITS: u32 = 9_971_640;

// 原版
const DUCHESS_INVULNERABLE: u32 = 7_500_301;

const DUCHESS_INVULNERABLE_SECONDS: f32 = 0.8;
const RESONANCE_EXTRA_SECONDS: f32 = 3.0;
/// 每层属性攻击力倍率
const RESIDUE_RATE: f32 = 1.005;

const SELF: Set = Set::Bit(sp::TARGET_BYTE, sp::SELF_TARGET_BIT, true);
const KEEP_ON_DEATH: Set = Set::Bit(sp::KEEP_ON_DEATH_BYTE, sp::KEEP_ON_DEATH_BIT, true);

const fn marker(id: u32, icon_of: super::rows::Icon) -> [Set; 4] {
    [Set::F32(sp::EFFECT_ENDURANCE, -1.0), SAVE, icon(icon_of), text(id)]
}

/// 第 bit 位的层数：魔力 / 火 / 雷 / 圣攻击力 ×1.005^(2^bit)
const fn residue_rate(bit: u32) -> f32 {
    let mut rate = RESIDUE_RATE;
    let mut i = 0;
    while i < bit {
        rate *= rate;
        i += 1;
    }
    rate
}

macro_rules! residue_bit {
    ($bit:literal) => {
        RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: RECLUSE_RESIDUE_BITS + $bit, name: concat!("[Tiered Buff] Recluse: Residues raise affinity attack - x2^", $bit),
            sets: &[Set::F32(sp::EFFECT_ENDURANCE, -1.0), Set::F32(sp::ATTACK_RATES[1], residue_rate($bit)), Set::F32(sp::ATTACK_RATES[2], residue_rate($bit)),
                Set::F32(sp::ATTACK_RATES[3], residue_rate($bit)), Set::F32(sp::ATTACK_RATES[4], residue_rate($bit)), SELF, KEEP_ON_DEATH] }
    };
}

const DUCHESS_MARKER: [Set; 4] = marker(DUCHESS_LONG_DODGE, ICON_SKILL);
const RESONANCE_MARKER: [Set; 4] = marker(UNDERTAKER_LONG_RESONANCE, ICON_ULTIMATE);
const RESIDUE_MARKER: [Set; 4] = marker(RECLUSE_RESIDUE_STACK, ICON_ATTACK);

pub const ROWS: &[RowSpec] = &[
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: DUCHESS_LONG_DODGE, name: "[Tiered Buff] Duchess: Longer relic skill invulnerability", sets: &DUCHESS_MARKER },
    RowSpec { table: &SP_EFFECT, template: DUCHESS_INVULNERABLE, id: DUCHESS_LONG_DODGE_EXTRA, name: "[Tiered Buff] Duchess: Longer relic skill invulnerability - Invulnerable",
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, DUCHESS_INVULNERABLE_SECONDS)] },
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: UNDERTAKER_LONG_RESONANCE, name: "[Tiered Buff] Undertaker: Longer bonus Ult window", sets: &RESONANCE_MARKER },
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: UNDERTAKER_LONG_RESONANCE_TIMER, name: "[Tiered Buff] Undertaker: Longer bonus Ult window - Timer",
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, RESONANCE_EXTRA_SECONDS), SELF] },
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: RECLUSE_RESIDUE_STACK, name: "[Tiered Buff] Recluse: Residues raise affinity attack", sets: &RESIDUE_MARKER },
    residue_bit!(0),
    residue_bit!(1),
    residue_bit!(2),
    residue_bit!(3),
    residue_bit!(4),
    residue_bit!(5),
    residue_bit!(6),
];

pub const BUFF_ROWS: &[RowSpec] = buff_rows![
    (DUCHESS_LONG_DODGE, DUCHESS_LONG_DODGE, ICON_SKILL, RARITY_PURPLE, "Duchess: Longer relic skill invulnerability"),
    (UNDERTAKER_LONG_RESONANCE, UNDERTAKER_LONG_RESONANCE, ICON_ULTIMATE, RARITY_PURPLE, "Undertaker: Longer bonus Ult window"),
    (RECLUSE_RESIDUE_STACK, RECLUSE_RESIDUE_STACK, ICON_ATTACK, RARITY_PURPLE, "Recluse: Residues raise affinity attack"),
];

pub const ROLE: &[RoleBuff] = &[
    RoleBuff { hero: DUCHESS, buff: DUCHESS_LONG_DODGE },
    RoleBuff { hero: UNDERTAKER, buff: UNDERTAKER_LONG_RESONANCE },
    RoleBuff { hero: RECLUSE, buff: RECLUSE_RESIDUE_STACK },
];
