//! 局内武器词条的三档数值：最高档按定好的值，低档保持 MMV 原来的档差。各档效果只有词条自己在用，原地改写：
//! 词条行（AttachEffectParam）改界面显示值，效果行（SpEffectParam，与词条同号）改五项攻击力倍率。
//! - 血量全满时提升攻击力 11 / 14.5 / 18（界面同原版 10.5 显示 10 的取整，显示 14）、提升战技攻击力 11 / 14 / 17
//! - 强化魔法 / 强化祷告 5 / 8 / 11，强化魔法、祷告 2 / 5 / 8，强化魔法、祷告的蓄力施展 / 蓄力执行 9 / 12 / 15
//! - 提升双手共持 / 各持时的攻击力 12 / 15 / 18（同原版；该姿势下每一下都吃加成，比只管蓄力攻击的词条强太多，11-01 从 15 / 18 / 21 降回）、
//!   强化蓄力攻击 24 / 27 / 30（原版 16 / 19 / 22，只对攻击分类 100 蓄力攻击生效）；强化冲刺攻击 20 / 24 / 28 不变
//! MMV 给强化祷告加了第二个效果 8330105~107，「强化魔法、祷告」8330110~112 也是 MMV 才有；原版没有就跳过（见 OPTIONAL）。

use crate::layout::{ATTACH_EFFECT, SP_EFFECT, attach_effect as ae, sp_effect as sp};
use crate::spec::{RowSpec, Set};

/// 词条行：改界面显示值
macro_rules! shown_rows {
    ($(($id:expr, $shown:expr, $name:literal)),* $(,)?) => {
        &[$(RowSpec { table: &ATTACH_EFFECT, template: $id, id: $id, name: $name, sets: &[Set::I32(ae::DISPLAYED_VALUE, $shown)] }),*]
    };
}

/// 效果行：物理 / 魔力 / 火 / 雷 / 圣五项攻击力倍率（原版各档五项相同）
macro_rules! rate_rows {
    ($(($id:expr, $rate:expr, $name:literal)),* $(,)?) => {
        &[$(RowSpec { table: &SP_EFFECT, template: $id, id: $id, name: $name, sets: &[
            Set::F32(sp::ATTACK_RATES[0], $rate), Set::F32(sp::ATTACK_RATES[1], $rate), Set::F32(sp::ATTACK_RATES[2], $rate),
            Set::F32(sp::ATTACK_RATES[3], $rate), Set::F32(sp::ATTACK_RATES[4], $rate)] }),*]
    };
}

/// 只有 MMV 才有的行
pub const OPTIONAL: [u32; 6] = [8_330_105, 8_330_106, 8_330_107, 8_330_110, 8_330_111, 8_330_112];

pub const SHOWN: &[RowSpec] = shown_rows![
    (8_670_000, 11, "[Weapon] Improved Attack Power at Full HP - Potency 1"),
    (8_670_001, 14, "[Weapon] Improved Attack Power at Full HP - Potency 2"),
    (8_670_002, 18, "[Weapon] Improved Attack Power at Full HP - Potency 3"),
    (8_350_000, 11, "[Weapon] Improved Skill Attack Power - Potency 1"),
    (8_350_001, 14, "[Weapon] Improved Skill Attack Power - Potency 2"),
    (8_350_002, 17, "[Weapon] Improved Skill Attack Power - Potency 3"),
    (8_330_000, 5, "[Weapon] Improved Sorceries - Potency 1"),
    (8_330_001, 8, "[Weapon] Improved Sorceries - Potency 2"),
    (8_330_002, 11, "[Weapon] Improved Sorceries - Potency 3"),
    (8_330_100, 5, "[Weapon] Improved Incantations - Potency 1"),
    (8_330_101, 8, "[Weapon] Improved Incantations - Potency 2"),
    (8_330_102, 11, "[Weapon] Improved Incantations - Potency 3"),
    (8_330_110, 2, "[Weapon] Improved Spells - Potency 1"),
    (8_330_111, 5, "[Weapon] Improved Spells - Potency 2"),
    (8_330_112, 8, "[Weapon] Improved Spells - Potency 3"),
    (8_330_200, 9, "[Weapon] Improved Charged Sorceries - Potency 1"),
    (8_330_201, 12, "[Weapon] Improved Charged Sorceries - Potency 2"),
    (8_330_202, 15, "[Weapon] Improved Charged Sorceries - Potency 3"),
    (8_330_300, 9, "[Weapon] Improved Charged Incantations - Potency 1"),
    (8_330_301, 12, "[Weapon] Improved Charged Incantations - Potency 2"),
    (8_330_302, 15, "[Weapon] Improved Charged Incantations - Potency 3"),
    (8_300_000, 12, "[Weapon] Improved Attack Power when Two-Handing - Potency 1"),
    (8_300_001, 15, "[Weapon] Improved Attack Power when Two-Handing - Potency 2"),
    (8_300_002, 18, "[Weapon] Improved Attack Power when Two-Handing - Potency 3"),
    (8_310_000, 12, "[Weapon] Attack Up when Wielding Two Armaments - Potency 1"),
    (8_310_001, 15, "[Weapon] Attack Up when Wielding Two Armaments - Potency 2"),
    (8_310_002, 18, "[Weapon] Attack Up when Wielding Two Armaments - Potency 3"),
    (8_320_100, 24, "[Weapon] Improved Charge Attacks - Potency 1"),
    (8_320_101, 27, "[Weapon] Improved Charge Attacks - Potency 2"),
    (8_320_102, 30, "[Weapon] Improved Charge Attacks - Potency 3"),
];

pub const RATES: &[RowSpec] = rate_rows![
    (8_670_000, 1.11, "[Weapon] Improved Attack Power at Full HP - Potency 1"),
    (8_670_001, 1.145, "[Weapon] Improved Attack Power at Full HP - Potency 2"),
    (8_670_002, 1.18, "[Weapon] Improved Attack Power at Full HP - Potency 3"),
    (8_350_000, 1.11, "[Weapon] Improved Skill Attack Power - Potency 1"),
    (8_350_001, 1.14, "[Weapon] Improved Skill Attack Power - Potency 2"),
    (8_350_002, 1.17, "[Weapon] Improved Skill Attack Power - Potency 3"),
    (8_330_000, 1.05, "[Weapon] Improved Sorceries - Potency 1"),
    (8_330_001, 1.08, "[Weapon] Improved Sorceries - Potency 2"),
    (8_330_002, 1.11, "[Weapon] Improved Sorceries - Potency 3"),
    (8_330_100, 1.05, "[Weapon] Improved Incantations - Potency 1"),
    (8_330_101, 1.08, "[Weapon] Improved Incantations - Potency 2"),
    (8_330_102, 1.11, "[Weapon] Improved Incantations - Potency 3"),
    (8_330_105, 1.05, "[Weapon] Improved Incantations - Potency 1_1"),
    (8_330_106, 1.08, "[Weapon] Improved Incantations - Potency 2_1"),
    (8_330_107, 1.11, "[Weapon] Improved Incantations - Potency 3_1"),
    (8_330_110, 1.02, "[Weapon] Improved Spells - Potency 1"),
    (8_330_111, 1.05, "[Weapon] Improved Spells - Potency 2"),
    (8_330_112, 1.08, "[Weapon] Improved Spells - Potency 3"),
    (8_330_200, 1.09, "[Weapon] Improved Charged Sorceries - Potency 1"),
    (8_330_201, 1.12, "[Weapon] Improved Charged Sorceries - Potency 2"),
    (8_330_202, 1.15, "[Weapon] Improved Charged Sorceries - Potency 3"),
    (8_330_300, 1.09, "[Weapon] Improved Charged Incantations - Potency 1"),
    (8_330_301, 1.12, "[Weapon] Improved Charged Incantations - Potency 2"),
    (8_330_302, 1.15, "[Weapon] Improved Charged Incantations - Potency 3"),
    (8_300_000, 1.12, "[Weapon] Improved Attack Power when Two-Handing - Potency 1"),
    (8_300_001, 1.15, "[Weapon] Improved Attack Power when Two-Handing - Potency 2"),
    (8_300_002, 1.18, "[Weapon] Improved Attack Power when Two-Handing - Potency 3"),
    (8_310_000, 1.12, "[Weapon] Attack Up when Wielding Two Armaments - Potency 1"),
    (8_310_001, 1.15, "[Weapon] Attack Up when Wielding Two Armaments - Potency 2"),
    (8_310_002, 1.18, "[Weapon] Attack Up when Wielding Two Armaments - Potency 3"),
    (8_320_100, 1.24, "[Weapon] Improved Charge Attacks - Potency 1"),
    (8_320_101, 1.27, "[Weapon] Improved Charge Attacks - Potency 2"),
    (8_320_102, 1.30, "[Weapon] Improved Charge Attacks - Potency 3"),
];

pub fn rows() -> impl Iterator<Item = &'static RowSpec> {
    SHOWN.iter().chain(RATES)
}
