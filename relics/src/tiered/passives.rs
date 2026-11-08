//! 新增武器词条（只有一档，只给 MMV：霸体靠 tools\tiered_buffs.lua 包住 ExecEvent 按动作事件触发）：
//! - 持有时发动战技会进入霸体（只出在近战武器）、持有时发动法术 / 祷告会进入霸体（只出在法杖 / 圣印记）：
//!   词条效果只是标记，脚本在战技 / 施法的动作事件出现时挂 2 秒不硬直（同原版 5300，受击等级全换成 1）
//! - 持有时大幅提升仇恨值（只出在盾牌）：被选为目标的优先度 +600（同复仇者塞巴斯蒂安的嘲讽）
//! 武器词条表（AttachEffectTableParam）按武器分组：501g00000 / +100 / +200 是三个效力档（普通与深夜第 5 栏），505g00100 / +200 是深夜第 6 栏；
//! g = 0~2 近战武器，3 盾牌，4 法杖，5 圣印记，6 弓弩。新词条在分组的五张表里各追加一行，权重同普通词条。

use nr_stat_alloc_patcher::param_write::OwnedRow;
use nr_stat_alloc_shared::param::{ParamFile, field};

use crate::layout::{ATTACH_EFFECT, SP_EFFECT, attach_effect as ae, attach_effect_table as table, sp_effect as sp};
use crate::pool::check_contiguous;
use crate::spec::{NAME_PREFIX, RowSpec, Set};

use super::rows::{MARKER_TEMPLATE, NO_SYNC};

/// 词条与标记效果同号，同号作文本 ID；_ON 是脚本施加的霸体
const SKILL_ARMOR: u32 = 9_972_000;
const SKILL_ARMOR_ON: u32 = 9_972_001;
const CAST_ARMOR: u32 = 9_972_010;
const CAST_ARMOR_ON: u32 = 9_972_011;
const AGGRO: u32 = 9_972_020;

// 模板
/// 武器词条「维持防御时，强化魔法、祷告」：不显示数值的词条行
const PASSIVE_TEMPLATE: u32 = 8_885_200;
/// 「Prevent all Stagger」：类别 1001，受击全部替换为不硬直
const NO_STAGGER_TEMPLATE: u32 = 5_300;
/// 武器词条「敌人比较不会攻击自己」一档（常驻、类别 10，优先度 -300）
const AGGRO_TEMPLATE: u32 = 8_520_000;
/// 武器词条效果的类别（同原版各档）
const PASSIVE_CATEGORY: u16 = 10;

const ARMOR_SECONDS: f32 = 2.0;
const AGGRO_PRIORITY: f32 = 600.0;
/// 状态图标：强韧度（战技「忍耐」）/ 吸引敌人（守护者遗物）
const POISE_ICON: i32 = 20_408;
const AGGRO_ICON: i32 = 20_429;
const WEIGHT: u16 = 100;

const MELEE_GROUPS: &[u32] = &[0, 1, 2];
const SHIELD_GROUPS: &[u32] = &[3];
const CATALYST_GROUPS: &[u32] = &[4, 5];

/// (词条, 出在哪些武器分组)
const PLACEMENTS: [(u32, &[u32]); 3] = [(SKILL_ARMOR, MELEE_GROUPS), (CAST_ARMOR, CATALYST_GROUPS), (AGGRO, SHIELD_GROUPS)];

/// 词条行：指向同号效果与文本，自成一组（不与别的词条互斥、不升档）
macro_rules! passive_row {
    ($id:expr, $icon:expr, $name:literal) => {
        RowSpec { table: &ATTACH_EFFECT, template: PASSIVE_TEMPLATE, id: $id, name: $name,
            sets: &[Set::I32(ae::PASSIVE_SP_EFFECT, $id as i32), Set::I32(ae::TEXT_ID, $id as i32), Set::I32(ae::COMPATIBILITY_ID, $id as i32),
                Set::I32(ae::OVERRIDE_BASE, $id as i32), Set::I32(ae::OVERRIDE_EFFECT, $id as i32), Set::I32(ae::STATUS_ICON, $icon),
                Set::Bit(ae::FLAGS_BYTE, ae::PERSISTENT_BIT, true)] }
    };
}

/// 脚本施加的霸体：只给自己、不同步
macro_rules! armor_row {
    ($id:expr, $name:literal) => {
        RowSpec { table: &SP_EFFECT, template: NO_STAGGER_TEMPLATE, id: $id, name: $name,
            sets: &[Set::F32(sp::EFFECT_ENDURANCE, ARMOR_SECONDS), Set::Bit(sp::TARGET_BYTE, sp::SELF_TARGET_BIT, true), NO_SYNC] }
    };
}

const MARKER: [Set; 2] = [Set::F32(sp::EFFECT_ENDURANCE, -1.0), Set::U16(sp::SP_CATEGORY, PASSIVE_CATEGORY)];

pub const ROWS: &[RowSpec] = &[
    passive_row!(SKILL_ARMOR, POISE_ICON, "[Weapon] Poise when using Skill"),
    passive_row!(CAST_ARMOR, POISE_ICON, "[Weapon] Poise when casting Spells"),
    passive_row!(AGGRO, AGGRO_ICON, "[Weapon] Greatly draws enemy attention"),
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: SKILL_ARMOR, name: "[Weapon] Poise when using Skill", sets: &MARKER },
    armor_row!(SKILL_ARMOR_ON, "[Weapon] Poise when using Skill - Active"),
    RowSpec { table: &SP_EFFECT, template: MARKER_TEMPLATE, id: CAST_ARMOR, name: "[Weapon] Poise when casting Spells", sets: &MARKER },
    armor_row!(CAST_ARMOR_ON, "[Weapon] Poise when casting Spells - Active"),
    RowSpec { table: &SP_EFFECT, template: AGGRO_TEMPLATE, id: AGGRO, name: "[Weapon] Greatly draws enemy attention",
        sets: &[Set::F32(sp::TARGET_PRIORITY, AGGRO_PRIORITY)] },
];

/// 分组 g 的五张词条表
fn group_tables(g: u32) -> [u32; 5] {
    let base = 501_000_000 + g * 100_000;
    let night = 505_000_000 + g * 100_000;
    [base, base + 100, base + 200, night + 100, night + 200]
}

fn placements() -> impl Iterator<Item = (u32, u32)> {
    PLACEMENTS.into_iter().flat_map(|(passive, groups)| groups.iter().flat_map(move |&g| group_tables(g).map(|t| (passive, t))))
}

fn is_mine(row: &[u8]) -> bool {
    field::i32(row, table::ATTACH_EFFECT_ID).is_some_and(|id| PLACEMENTS.iter().any(|p| p.0 as i32 == id))
}

/// 在各表最后一行之后追加新词条（以该行为模板），返回追加了几行；重复运行先删掉上次加的
pub fn add_to_tables(rows: &mut Vec<OwnedRow>) -> Result<usize, String> {
    rows.retain(|r| !is_mine(&r.data));
    let mut added = 0;
    for (passive, id) in placements() {
        let last = rows.iter().rposition(|r| r.id == id).ok_or(format!("attach effect table {id} not found"))?;
        let mut data = rows[last].data.clone();
        data[table::ATTACH_EFFECT_ID..table::ATTACH_EFFECT_ID + 4].copy_from_slice(&(passive as i32).to_le_bytes());
        data[table::WEIGHT..table::WEIGHT + 2].copy_from_slice(&WEIGHT.to_le_bytes());
        data[table::WEIGHT_DLC..table::WEIGHT_DLC + 2].copy_from_slice(&(-1i16).to_le_bytes());
        rows.insert(last + 1, OwnedRow { id, data, name: Some(format!("{NAME_PREFIX}Weapon passive {passive}")) });
        check_contiguous(rows, id)?;
        added += 1;
    }
    Ok(added)
}

/// 输出自检：每张表都有对应的新词条且权重不为 0
pub fn verify(file: &ParamFile) -> Result<(), String> {
    for (passive, id) in placements() {
        let found = file.rows().any(|r| {
            r.id == id
                && field::i32(r.data, table::ATTACH_EFFECT_ID) == Some(passive as i32)
                && field::u16(r.data, table::WEIGHT).is_some_and(|w| w > 0)
        });
        if !found {
            return Err(format!("self-check: weapon passive {passive} missing from table {id}"));
        }
    }
    Ok(())
}
