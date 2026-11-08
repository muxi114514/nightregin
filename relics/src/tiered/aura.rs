//! 第四批：放技艺 / 绝招时给自己和附近队友挂效果的角色锻体。照原版遗物「圣杯瓶也能治疗队友」：脚本给自己施加「发出」效果
//! （状态 275 + 行为号）→ 行为 → 子弹（只打队友、0.1 秒）把 1~2 个效果挂到范围内队友身上；子弹不打自己，自己那份脚本直接施加。
//! 「绝招结束时再延长」这类要等原版效果消失才能做的，子弹只挂一个标记，由每个人自己的脚本（tools\tiered_buffs.lua）在原版效果消失时补上。
//! 行为与子弹用 900997101 起的编号（同原版 900102221 的写法，两张表同号）。

use crate::layout::{BEHAVIOR_PC, BULLET, SP_EFFECT, behavior, bullet, sp_effect as sp};
use crate::spec::{RowSpec, Set};

use super::pools::RoleBuff;
use super::rows::{
    DUCHESS, GUARDIAN, ICON_SKILL, ICON_ULTIMATE, MARKER_TEMPLATE, NO_SYNC, RAIDER, RARITY_PURPLE, REVENANT, SAVE, attack, buff_rows, cut, icon, text,
};

// 守护者4：放技艺时，自己与附近队友获得振奋香
const GUARDIAN_AROMATIC: u32 = 9_971_450;
const GUARDIAN_AROMATIC_SPAWN: u32 = 9_971_451;
// 女爵5：绝招隐身结束后再隐身 7.5 秒
const DUCHESS_LONG_FINALE: u32 = 9_971_420;
const DUCHESS_LONG_FINALE_SPAWN: u32 = 9_971_421;
const DUCHESS_LONG_FINALE_MARK: u32 = 9_971_422;
const DUCHESS_LONG_FINALE_EXTRA: u32 = 9_971_423;
// 女爵6：放绝招时，隐身的我方魔力攻击力 +50%
const DUCHESS_FINALE_MAGIC: u32 = 9_971_430;
const DUCHESS_FINALE_MAGIC_SPAWN: u32 = 9_971_431;
const DUCHESS_FINALE_MAGIC_BUFF: u32 = 9_971_432;
// 无赖2：放绝招时，附近我方 30 秒减伤 15%、攻击力 +15%
const RAIDER_TOTEM_GUARD: u32 = 9_971_460;
const RAIDER_TOTEM_GUARD_SPAWN: u32 = 9_971_461;
const RAIDER_TOTEM_GUARD_BUFF: u32 = 9_971_462;
// 无赖3：放绝招时，附近我方 30 秒霸体
const RAIDER_TOTEM_POISE: u32 = 9_971_470;
const RAIDER_TOTEM_POISE_SPAWN: u32 = 9_971_471;
const RAIDER_TOTEM_POISE_BUFF: u32 = 9_971_472;
// 复仇者4：绝招的不死结束后再延长 7.5 秒
const REVENANT_LONG_MARCH: u32 = 9_971_480;
const REVENANT_LONG_MARCH_SPAWN: u32 = 9_971_481;
const REVENANT_LONG_MARCH_MARK: u32 = 9_971_482;
const REVENANT_LONG_MARCH_ALLY: u32 = 9_971_483;
const REVENANT_LONG_MARCH_SELF: u32 = 9_971_484;
// 复仇者5：放绝招后，被附加不死的我方获得一次安定者式的复活（1 分钟内没用掉自行消失）
const REVENANT_REVIVE: u32 = 9_971_490;
const REVENANT_REVIVE_SPAWN: u32 = 9_971_491;
const REVENANT_REVIVE_TIMER: u32 = 9_971_492;
const REVENANT_REVIVE_CHARGE: u32 = 9_971_493;

/// 行为 / 子弹编号：900997101 起，按上面的顺序
const AURA_FIRST: u32 = 900_997_101;

// 原版效果
/// 振奋香：一次性大幅减伤（状态 42）+ 攻击力，各 40 秒
const AROMATIC_BUBBLE: u32 = 503_500;
const AROMATIC_ATTACK: u32 = 503_501;

// 模板
/// 遗物「圣杯瓶也能治疗队友」的发出效果、行为与子弹（半径 15、只打队友）
const SPAWN_TEMPLATE: u32 = 7_010_280;
const AURA_TEMPLATE: u32 = 900_102_221;
/// 女爵绝招隐身（15 秒，状态 600）、复仇者绝招不死（队友 705000 / 自己 705010，15 秒，状态 143，条件状态 624）
const FINALE_TEMPLATE: u32 = 703_500;
const MARCH_ALLY_TEMPLATE: u32 = 705_000;
const MARCH_SELF_TEMPLATE: u32 = 705_010;
/// 隐士遗物「发动绝招时提升攻击力」的 30 秒增益（带状态栏图标与文本）
const TIMED_BUFF_TEMPLATE: u32 = 7_032_903;
/// 受击等级全换 1（类别 1001，不硬直）
const NO_STAGGER_TEMPLATE: u32 = 5_300;
/// 安定者复活（状态 2017：倒地动作无敌、放安定特效）。原版类别 1008 与倒地状态 102180 互斥，存在时倒地状态挂不上、队友救不了，只由起身动画消耗
const STEADFAST_REVIVE: u32 = 8_970_061;

const SKILL_RADIUS: f32 = 15.0;
const ULT_RADIUS: f32 = 20.0;
const REVIVE_SECONDS: f32 = 60.0;
/// 同 ID 重复施加只重置时长，不与其他效果互斥
const NON_EXCLUSIVE_CATEGORY: u16 = 20;
/// s8 -1：不写入存档
const NOT_SAVED: u8 = 255;
const DUCHESS_INVIS_STATE: u16 = 600;
const ATTACK_UP_STATUS: i32 = 20_406;
const POISE_STATUS: i32 = 22_208;
/// 徘徊赐福「自动复活」的图标
const REVIVE_STATUS: i32 = 20_467;

const SELF: Set = Set::Bit(sp::TARGET_BYTE, sp::SELF_TARGET_BIT, true);

/// 发出效果 + 行为 + 子弹三行：$n 是第几个（行为 / 子弹编号 AURA_FIRST + $n），子弹命中队友时挂 $a（与 $b）
macro_rules! aura {
    ($spawn:expr, $n:expr, $radius:expr, $a:expr, $b:expr, $name:literal) => {
        [
            RowSpec { table: &SP_EFFECT, template: SPAWN_TEMPLATE, id: $spawn, name: concat!("[Tiered Buff] ", $name, " - Spawn"),
                sets: &[Set::I32(sp::BEHAVIOR_ID, (AURA_FIRST + $n) as i32)] },
            RowSpec { table: &BEHAVIOR_PC, template: AURA_TEMPLATE, id: AURA_FIRST + $n, name: concat!("[Tiered Buff] ", $name),
                sets: &[Set::I32(behavior::REF_ID, (AURA_FIRST + $n) as i32)] },
            RowSpec { table: &BULLET, template: AURA_TEMPLATE, id: AURA_FIRST + $n, name: concat!("[Tiered Buff] ", $name),
                sets: &[Set::F32(bullet::HIT_RADIUS, $radius), Set::I32(bullet::SP_EFFECT_0, $a as i32), Set::I32(bullet::SP_EFFECT_1, $b as i32)] },
        ]
    };
}

const fn marker(id: u32, icon_of: super::rows::Icon) -> [Set; 4] {
    [Set::F32(sp::EFFECT_ENDURANCE, -1.0), SAVE, icon(icon_of), text(id)]
}

const fn shown(icon_id: i32, id: u32) -> [Set; 2] {
    [Set::I32(sp::ICON_ID, icon_id), text(id)]
}

const MAGIC_SHOWN: [Set; 2] = shown(ATTACK_UP_STATUS, DUCHESS_FINALE_MAGIC_BUFF);
const TOTEM_GUARD_SHOWN: [Set; 2] = shown(ATTACK_UP_STATUS, RAIDER_TOTEM_GUARD_BUFF);
const TOTEM_POISE_SHOWN: [Set; 2] = shown(POISE_STATUS, RAIDER_TOTEM_POISE_BUFF);
const REVIVE_SHOWN: [Set; 2] = shown(REVIVE_STATUS, REVENANT_REVIVE_CHARGE);

const EFFECTS: &[RowSpec] = &[
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: GUARDIAN_AROMATIC, name: "[Tiered Buff] Guardian: Skill grants Uplifting Aromatic",
        sets: &marker(GUARDIAN_AROMATIC, ICON_SKILL) },
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: DUCHESS_LONG_FINALE, name: "[Tiered Buff] Duchess: Longer Finale concealment",
        sets: &marker(DUCHESS_LONG_FINALE, ICON_ULTIMATE) },
    // 标记要比原版隐身（15 秒）活得久，才能在隐身结束那一刻还在
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: DUCHESS_LONG_FINALE_MARK, name: "[Tiered Buff] Duchess: Longer Finale concealment - Mark",
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, 20.0), SELF] },
    RowSpec { table: &SP_EFFECT, template: FINALE_TEMPLATE, id: DUCHESS_LONG_FINALE_EXTRA, name: "[Tiered Buff] Duchess: Longer Finale concealment - Extra",
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, 7.5)] },
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: DUCHESS_FINALE_MAGIC, name: "[Tiered Buff] Duchess: Concealed allies deal more magic damage",
        sets: &marker(DUCHESS_FINALE_MAGIC, ICON_ULTIMATE) },
    // 只在隐身（状态 600）时生效，涵盖延长的隐身
    RowSpec { table: &SP_EFFECT, template: TIMED_BUFF_TEMPLATE, id: DUCHESS_FINALE_MAGIC_BUFF, name: "[Tiered Buff] Duchess: Concealed allies deal more magic damage - Buff",
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, 25.0), attack(0, 1.0), attack(1, 1.5), attack(2, 1.0), attack(3, 1.0), attack(4, 1.0),
            Set::U16(sp::INVOCATION_STATE_1, DUCHESS_INVIS_STATE), Set::U16(sp::SP_CATEGORY, 20), Set::I32(sp::VFX_ID, -1), SELF,
            MAGIC_SHOWN[0], MAGIC_SHOWN[1]] },
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: RAIDER_TOTEM_GUARD, name: "[Tiered Buff] Raider: Totem Stela protects allies",
        sets: &marker(RAIDER_TOTEM_GUARD, ICON_ULTIMATE) },
    RowSpec { table: &SP_EFFECT, template: TIMED_BUFF_TEMPLATE, id: RAIDER_TOTEM_GUARD_BUFF, name: "[Tiered Buff] Raider: Totem Stela protects allies - Buff",
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, 30.0), attack(0, 1.15), attack(1, 1.15), attack(2, 1.15), attack(3, 1.15), attack(4, 1.15),
            cut(0, 0.85), cut(1, 0.85), cut(2, 0.85), cut(3, 0.85), cut(4, 0.85), cut(5, 0.85), cut(6, 0.85), cut(7, 0.85),
            Set::U16(sp::SP_CATEGORY, 20), Set::I32(sp::VFX_ID, -1), SELF, TOTEM_GUARD_SHOWN[0], TOTEM_GUARD_SHOWN[1]] },
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: RAIDER_TOTEM_POISE, name: "[Tiered Buff] Raider: Totem Stela grants poise",
        sets: &marker(RAIDER_TOTEM_POISE, ICON_ULTIMATE) },
    RowSpec { table: &SP_EFFECT, template: NO_STAGGER_TEMPLATE, id: RAIDER_TOTEM_POISE_BUFF, name: "[Tiered Buff] Raider: Totem Stela grants poise - Buff",
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, 30.0), SELF, TOTEM_POISE_SHOWN[0], TOTEM_POISE_SHOWN[1]] },
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: REVENANT_LONG_MARCH, name: "[Tiered Buff] Revenant: Longer Immortal March",
        sets: &marker(REVENANT_LONG_MARCH, ICON_ULTIMATE) },
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: REVENANT_LONG_MARCH_MARK, name: "[Tiered Buff] Revenant: Longer Immortal March - Mark",
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, 20.0), SELF] },
    // 原版带条件状态 624（绝招期间），延长时已经没有了，清掉；队友版原本不允许施加给自己，打开
    RowSpec { table: &SP_EFFECT, template: MARCH_ALLY_TEMPLATE, id: REVENANT_LONG_MARCH_ALLY, name: "[Tiered Buff] Revenant: Longer Immortal March - Ally",
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, 7.5), Set::U16(sp::INVOCATION_STATE_1, 0), SELF] },
    RowSpec { table: &SP_EFFECT, template: MARCH_SELF_TEMPLATE, id: REVENANT_LONG_MARCH_SELF, name: "[Tiered Buff] Revenant: Longer Immortal March - Self",
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, 7.5), Set::U16(sp::INVOCATION_STATE_1, 0)] },
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: REVENANT_REVIVE, name: "[Tiered Buff] Revenant: Immortal March grants a revive",
        sets: &marker(REVENANT_REVIVE, ICON_ULTIMATE) },
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: REVENANT_REVIVE_TIMER, name: "[Tiered Buff] Revenant: Immortal March grants a revive - Timer",
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, REVIVE_SECONDS), SELF] },
    // 借出的复活次数：不能直接挂原版 8970061（中途移除不会同步给队友，队友那边残留后这个人倒地救不了）。
    // 换普通类别不挡倒地状态、不同步、到时自己消失，状态 2017 保留原版的倒地无敌与特效，由 tiered_buffs.lua 在起身时用掉。
    // 状态栏图标挂在这里：图标在就表示还能复活，用掉或到期就消失
    RowSpec { table: &SP_EFFECT, template: STEADFAST_REVIVE, id: REVENANT_REVIVE_CHARGE, name: "[Tiered Buff] Revenant: Immortal March grants a revive - Charge",
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, REVIVE_SECONDS), Set::U16(sp::SP_CATEGORY, NON_EXCLUSIVE_CATEGORY), Set::U8(sp::SAVE_CATEGORY, NOT_SAVED),
            Set::I32(sp::REPLACE_SP_EFFECT, -1), NO_SYNC, REVIVE_SHOWN[0], REVIVE_SHOWN[1]] },
];

const GUARDIAN_AURA: [RowSpec; 3] = aura!(GUARDIAN_AROMATIC_SPAWN, 0, SKILL_RADIUS, AROMATIC_BUBBLE, AROMATIC_ATTACK, "Guardian: Skill grants Uplifting Aromatic");
const DUCHESS_LONG_AURA: [RowSpec; 3] = aura!(DUCHESS_LONG_FINALE_SPAWN, 1, ULT_RADIUS, DUCHESS_LONG_FINALE_MARK, 0, "Duchess: Longer Finale concealment");
const DUCHESS_MAGIC_AURA: [RowSpec; 3] =
    aura!(DUCHESS_FINALE_MAGIC_SPAWN, 2, ULT_RADIUS, DUCHESS_FINALE_MAGIC_BUFF, 0, "Duchess: Concealed allies deal more magic damage");
const RAIDER_GUARD_AURA: [RowSpec; 3] = aura!(RAIDER_TOTEM_GUARD_SPAWN, 3, ULT_RADIUS, RAIDER_TOTEM_GUARD_BUFF, 0, "Raider: Totem Stela protects allies");
const RAIDER_POISE_AURA: [RowSpec; 3] = aura!(RAIDER_TOTEM_POISE_SPAWN, 4, ULT_RADIUS, RAIDER_TOTEM_POISE_BUFF, 0, "Raider: Totem Stela grants poise");
const REVENANT_LONG_AURA: [RowSpec; 3] = aura!(REVENANT_LONG_MARCH_SPAWN, 5, ULT_RADIUS, REVENANT_LONG_MARCH_MARK, 0, "Revenant: Longer Immortal March");
const REVENANT_REVIVE_AURA: [RowSpec; 3] =
    aura!(REVENANT_REVIVE_SPAWN, 6, ULT_RADIUS, REVENANT_REVIVE_TIMER, 0, "Revenant: Immortal March grants a revive");

pub const ROW_GROUPS: [&[RowSpec]; 8] = [
    EFFECTS,
    &GUARDIAN_AURA,
    &DUCHESS_LONG_AURA,
    &DUCHESS_MAGIC_AURA,
    &RAIDER_GUARD_AURA,
    &RAIDER_POISE_AURA,
    &REVENANT_LONG_AURA,
    &REVENANT_REVIVE_AURA,
];

pub const BUFF_ROWS: &[RowSpec] = buff_rows![
    (GUARDIAN_AROMATIC, GUARDIAN_AROMATIC, ICON_SKILL, RARITY_PURPLE, "Guardian: Skill grants Uplifting Aromatic"),
    (DUCHESS_LONG_FINALE, DUCHESS_LONG_FINALE, ICON_ULTIMATE, RARITY_PURPLE, "Duchess: Longer Finale concealment"),
    (DUCHESS_FINALE_MAGIC, DUCHESS_FINALE_MAGIC, ICON_ULTIMATE, RARITY_PURPLE, "Duchess: Concealed allies deal more magic damage"),
    (RAIDER_TOTEM_GUARD, RAIDER_TOTEM_GUARD, ICON_ULTIMATE, RARITY_PURPLE, "Raider: Totem Stela protects allies"),
    (RAIDER_TOTEM_POISE, RAIDER_TOTEM_POISE, ICON_ULTIMATE, RARITY_PURPLE, "Raider: Totem Stela grants poise"),
    (REVENANT_LONG_MARCH, REVENANT_LONG_MARCH, ICON_ULTIMATE, RARITY_PURPLE, "Revenant: Longer Immortal March"),
    (REVENANT_REVIVE, REVENANT_REVIVE, ICON_ULTIMATE, RARITY_PURPLE, "Revenant: Immortal March grants a revive"),
];

pub const ROLE: &[RoleBuff] = &[
    RoleBuff { hero: GUARDIAN, buff: GUARDIAN_AROMATIC },
    RoleBuff { hero: DUCHESS, buff: DUCHESS_LONG_FINALE },
    RoleBuff { hero: DUCHESS, buff: DUCHESS_FINALE_MAGIC },
    RoleBuff { hero: RAIDER, buff: RAIDER_TOTEM_GUARD },
    RoleBuff { hero: RAIDER, buff: RAIDER_TOTEM_POISE },
    RoleBuff { hero: REVENANT, buff: REVENANT_LONG_MARCH },
    RoleBuff { hero: REVENANT, buff: REVENANT_REVIVE },
];
