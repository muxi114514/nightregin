//! 追踪者灵火版遗物词条（以原版火焰版为模板）：
//! - 7020050「发动技艺时，轻攻击能使出缠绕灵火的追加攻击（仅限大剑）」：原版 7020000 的标记（状态 2027）换成状态 2190；
//!   追加攻击动画 a900 的 500020/500021 里原本无条件的两样东西改由带状态条件的子弹事件触发（子弹 spEffectIDForShooter 施加）：
//!   刀身火焰特效 440673（判定 6044/6045 → 0.7 秒的特效效果 7020055 火焰 / 7020056 灵火）与附魔（判定 6040~6043 →
//!   状态 2027 施加原版火焰 7020001/7020003，状态 2190 施加灵火 7020051/7020053，灵火晚 1 帧，两种词条同时生效时灵火覆盖火焰）。
//!   灵火附魔 = 赫芬尖塔剑「毁灭灵火」1806/1808（魔力 +25、命中积累冻伤、灵火刀身特效），时长按原版遗物 20 秒；
//!   灵火刀身特效用赫芬点燃灵火时的持续段 450291
//! - 7010550「发动绝招时，能燃起灵火、蔓延周围」：原版 7010500 的标记（状态 2016）换成状态 2191（绝招大爆炸的替换见 wylder_ghostflame_blast.rs）；
//!   绝招动画里复制原版地火事件（判定 7030、条件 2016）为判定 7029、条件 2191 → 子弹链 30900150~152，
//!   末段换成死亡勾棒「灵火点燃」的地火特效 450822，伤害同原版 26% 但由火改物理（绝招无魔力攻击力，火伤会解冻），附加冻伤积累 902
//! 标记每 0.1 秒施加追加攻击入口效果（见 follow_up_gate.rs）。动作文件（TAE）、HKS 与文本由 tools\relic_assets.ps1 处理，这里只负责参数表。

use crate::layout::{
    ATK_PC, ATTACH_EFFECT, ATTACH_EFFECT_FILTER, BEHAVIOR_PC, BULLET, SP_EFFECT, SP_EFFECT_VFX, atk, attach_effect as ae,
    attach_effect_filter as filter, behavior, bullet, sp_effect as sp, sp_effect_vfx as vfx,
};
use crate::spec::{RowSpec, Set};

use super::follow_up_gate::{MARK_CYCLE, MARK_INTERVAL};

pub const FOLLOW_UP_STATE: u16 = 2190;
pub const ULTIMATE_STATE: u16 = 2191;
/// 灵火附魔持续时间：同原版火焰遗物
const ENCHANT_SECONDS: f32 = 20.0;
/// 死亡勾棒地火的特效与冻伤积累
const GHOSTFLAME_GROUND_SFX: i32 = 450_822;
const GHOSTFLAME_FROST: i32 = 902;
/// 灵火地火的攻击行：复用的原版空闲行（见 content::IN_PLACE）
pub const GHOST_GROUND_ATK: u32 = 700_000_808;
/// 追加攻击刀身特效：原动画事件在挂点 100 播 440673，持续约 0.7 秒；灵火版用赫芬点燃灵火的持续段
const SWIRL_DUMMY: u16 = 100;
const FIRE_SWIRL_SFX: i32 = 440_673;
const GHOST_SWIRL_SFX: i32 = 450_291;
const SWIRL_SECONDS: f32 = 0.7;
/// 模板：只有挂点特效的特效行、原版中性效果（全部倍率为 1）
const VFX_TEMPLATE: u32 = 4;
const NEUTRAL_EFFECT: u32 = 7_700;
/// 新文本：常驻效果名（PermanentBuffName_dlc01，9100001~096 已占用）与附魔 buff 名（menu SpEffectName，1211xx 段已占满）
const ULTIMATE_BUFF_TEXT: i32 = 9_100_097;
const FOLLOW_UP_BUFF_TEXT: i32 = 9_100_098;
const ENCHANT_TEXT_MAGIC: i32 = 7_020_052;
const ENCHANT_TEXT_FROST: i32 = 7_020_054;

pub const ROWS: &[RowSpec] = &[
    // 词条（筛选行独立，不与原版火焰版共用，排序值紧跟原版）与标记
    RowSpec { table: &ATTACH_EFFECT, template: 7_020_000, id: 7_020_050, name: "[Relic - Wylder] Ghostflame follow-ups when using Character Skill (greatsword only)",
        sets: &[Set::I32(ae::PASSIVE_SP_EFFECT, 7_020_050), Set::I32(ae::TEXT_ID, 7_020_050), Set::I32(ae::FILTER_ID, 7_020_050)] },
    RowSpec { table: &ATTACH_EFFECT, template: 7_010_500, id: 7_010_550, name: "[Relic - Wylder] Art activation spreads ghostflame in area",
        sets: &[Set::I32(ae::PASSIVE_SP_EFFECT, 7_010_550), Set::I32(ae::TEXT_ID, 7_010_550), Set::I32(ae::FILTER_ID, 7_010_550)] },
    RowSpec { table: &ATTACH_EFFECT_FILTER, template: 7_020_000, id: 7_020_050, name: "[Relic - Wylder] Ghostflame follow-ups",
        sets: &[Set::I32(filter::TEXT_ID, 7_020_050), Set::I32(filter::VALUE, 10_000_011)] },
    RowSpec { table: &ATTACH_EFFECT_FILTER, template: 7_010_500, id: 7_010_550, name: "[Relic - Wylder] Art spreads ghostflame",
        sets: &[Set::I32(filter::TEXT_ID, 7_010_550), Set::I32(filter::VALUE, 10_000_031)] },
    RowSpec { table: &SP_EFFECT, template: 7_020_000, id: 7_020_050, name: "[Relic - Wylder] Ghostflame follow-ups when using Character Skill (greatsword only)",
        sets: &[Set::U16(sp::STATE_INFO, FOLLOW_UP_STATE), Set::I32(sp::TEXT_ID_1, FOLLOW_UP_BUFF_TEXT), MARK_INTERVAL, MARK_CYCLE] },
    // 绝招标记还每 0.1 秒施加灵火爆炸标记（见 wylder_ghostflame_blast.rs），周期效果不做网络同步，同原版铁之眼毒箭遗物
    RowSpec { table: &SP_EFFECT, template: 7_010_500, id: 7_010_550, name: "[Relic - Wylder] Art activation spreads ghostflame in area",
        sets: &[Set::U16(sp::STATE_INFO, ULTIMATE_STATE), Set::I32(sp::TEXT_ID_1, ULTIMATE_BUFF_TEXT),
            Set::F32(sp::MOTION_INTERVAL, 0.1), Set::I32(sp::CYCLE_OCCURRENCE, super::wylder_ghostflame_blast::GHOST_BLAST_MARK),
            Set::Bit(sp::NET_SYNC_BYTE, sp::NET_SYNC_DISABLE_BIT, true)] },
    // 灵火附魔：触发（右 / 左手条件同原版）→ 附魔
    RowSpec { table: &SP_EFFECT, template: 7_020_001, id: 7_020_051, name: "[Skill - Wylder] Relic Ghostflame Followup Buff (Right Trigger)",
        sets: &[Set::I32(sp::CYCLE_OCCURRENCE, 7_020_052)] },
    RowSpec { table: &SP_EFFECT, template: 1_806, id: 7_020_052, name: "[Skill - Wylder] Relic Ghostflame Followup Buff (Right Active)",
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, ENCHANT_SECONDS), Set::I32(sp::TEXT_ID_1, ENCHANT_TEXT_MAGIC), Set::I32(sp::TEXT_ID_2, ENCHANT_TEXT_FROST)] },
    RowSpec { table: &SP_EFFECT, template: 7_020_003, id: 7_020_053, name: "[Skill - Wylder] Relic Ghostflame Followup Buff (Left Trigger)",
        sets: &[Set::I32(sp::CYCLE_OCCURRENCE, 7_020_054)] },
    RowSpec { table: &SP_EFFECT, template: 1_808, id: 7_020_054, name: "[Skill - Wylder] Relic Ghostflame Followup Buff (Left Active)",
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, ENCHANT_SECONDS), Set::I32(sp::TEXT_ID_1, ENCHANT_TEXT_MAGIC), Set::I32(sp::TEXT_ID_2, ENCHANT_TEXT_FROST)] },
    // 追加攻击动画里施加附魔用的子弹（空攻击「No Target」、无特效、原版自身效果子弹的半径，只给发射者加效果）与对应行为（判定 6040~6043）
    RowSpec { table: &BULLET, template: 2_900, id: 30_900_110, name: "[Skill - Wylder] Relic Followup Fire Buff (Right)",
        sets: &[Set::I32(bullet::ATK_ID, atk::NO_TARGET), Set::F32(bullet::HIT_RADIUS, bullet::SELF_EFFECT_RADIUS),
            Set::I32(bullet::SP_EFFECT_FOR_SHOOTER, 7_020_001)] },
    RowSpec { table: &BULLET, template: 2_900, id: 30_900_111, name: "[Skill - Wylder] Relic Followup Fire Buff (Left)",
        sets: &[Set::I32(bullet::ATK_ID, atk::NO_TARGET), Set::F32(bullet::HIT_RADIUS, bullet::SELF_EFFECT_RADIUS),
            Set::I32(bullet::SP_EFFECT_FOR_SHOOTER, 7_020_003)] },
    RowSpec { table: &BULLET, template: 2_900, id: 30_900_112, name: "[Skill - Wylder] Relic Followup Ghostflame Buff (Right)",
        sets: &[Set::I32(bullet::ATK_ID, atk::NO_TARGET), Set::F32(bullet::HIT_RADIUS, bullet::SELF_EFFECT_RADIUS),
            Set::I32(bullet::SP_EFFECT_FOR_SHOOTER, 7_020_051)] },
    RowSpec { table: &BULLET, template: 2_900, id: 30_900_113, name: "[Skill - Wylder] Relic Followup Ghostflame Buff (Left)",
        sets: &[Set::I32(bullet::ATK_ID, atk::NO_TARGET), Set::F32(bullet::HIT_RADIUS, bullet::SELF_EFFECT_RADIUS),
            Set::I32(bullet::SP_EFFECT_FOR_SHOOTER, 7_020_053)] },
    RowSpec { table: &BEHAVIOR_PC, template: 700_000_030, id: 600_000_040, name: "[Skill - Wylder] Relic Followup Fire Buff (Right)",
        sets: &[Set::I32(behavior::JUDGE_ID, 40), Set::I32(behavior::REF_ID, 30_900_110)] },
    RowSpec { table: &BEHAVIOR_PC, template: 700_000_030, id: 600_000_041, name: "[Skill - Wylder] Relic Followup Fire Buff (Left)",
        sets: &[Set::I32(behavior::JUDGE_ID, 41), Set::I32(behavior::REF_ID, 30_900_111)] },
    RowSpec { table: &BEHAVIOR_PC, template: 700_000_030, id: 600_000_042, name: "[Skill - Wylder] Relic Followup Ghostflame Buff (Right)",
        sets: &[Set::I32(behavior::JUDGE_ID, 42), Set::I32(behavior::REF_ID, 30_900_112)] },
    RowSpec { table: &BEHAVIOR_PC, template: 700_000_030, id: 600_000_043, name: "[Skill - Wylder] Relic Followup Ghostflame Buff (Left)",
        sets: &[Set::I32(behavior::JUDGE_ID, 43), Set::I32(behavior::REF_ID, 30_900_113)] },
    // 刀身特效：特效行 → 0.7 秒的特效效果 → 施加用子弹 → 行为（判定 6044 火焰 / 6045 灵火）
    RowSpec { table: &SP_EFFECT_VFX, template: VFX_TEMPLATE, id: 7_020_055, name: "[Skill - Wylder] Relic Followup Fire Swirl",
        sets: &[Set::I32(vfx::MIDST_SFX, FIRE_SWIRL_SFX), Set::U16(vfx::MIDST_DMY, SWIRL_DUMMY)] },
    RowSpec { table: &SP_EFFECT_VFX, template: VFX_TEMPLATE, id: 7_020_056, name: "[Skill - Wylder] Relic Followup Ghostflame Swirl",
        sets: &[Set::I32(vfx::MIDST_SFX, GHOST_SWIRL_SFX), Set::U16(vfx::MIDST_DMY, SWIRL_DUMMY)] },
    RowSpec { table: &SP_EFFECT, template: NEUTRAL_EFFECT, id: 7_020_055, name: "[Skill - Wylder] Relic Followup Fire Swirl",
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, SWIRL_SECONDS), Set::I32(sp::VFX_ID, 7_020_055)] },
    RowSpec { table: &SP_EFFECT, template: NEUTRAL_EFFECT, id: 7_020_056, name: "[Skill - Wylder] Relic Followup Ghostflame Swirl",
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, SWIRL_SECONDS), Set::I32(sp::VFX_ID, 7_020_056)] },
    RowSpec { table: &BULLET, template: 2_900, id: 30_900_114, name: "[Skill - Wylder] Relic Followup Fire Swirl",
        sets: &[Set::I32(bullet::ATK_ID, atk::NO_TARGET), Set::F32(bullet::HIT_RADIUS, bullet::SELF_EFFECT_RADIUS),
            Set::I32(bullet::SP_EFFECT_FOR_SHOOTER, 7_020_055)] },
    RowSpec { table: &BULLET, template: 2_900, id: 30_900_115, name: "[Skill - Wylder] Relic Followup Ghostflame Swirl",
        sets: &[Set::I32(bullet::ATK_ID, atk::NO_TARGET), Set::F32(bullet::HIT_RADIUS, bullet::SELF_EFFECT_RADIUS),
            Set::I32(bullet::SP_EFFECT_FOR_SHOOTER, 7_020_056)] },
    RowSpec { table: &BEHAVIOR_PC, template: 700_000_030, id: 600_000_044, name: "[Skill - Wylder] Relic Followup Fire Swirl",
        sets: &[Set::I32(behavior::JUDGE_ID, 44), Set::I32(behavior::REF_ID, 30_900_114)] },
    RowSpec { table: &BEHAVIOR_PC, template: 700_000_030, id: 600_000_045, name: "[Skill - Wylder] Relic Followup Ghostflame Swirl",
        sets: &[Set::I32(behavior::JUDGE_ID, 45), Set::I32(behavior::REF_ID, 30_900_115)] },
    // 绝招灵火地火：行为（判定 7029）→ 发射 → 12 方向散开 → 地火
    RowSpec { table: &BEHAVIOR_PC, template: 700_000_030, id: 700_000_029, name: "[Ultimate - Wylder] Onslaught Stake (Relic Ghostflame)",
        sets: &[Set::I32(behavior::JUDGE_ID, 29), Set::I32(behavior::REF_ID, 30_900_150)] },
    RowSpec { table: &BULLET, template: 30_900_100, id: 30_900_150, name: "[Ultimate - Wylder] Onslaught Stake (Relic Ghostflame)",
        sets: &[Set::I32(bullet::HIT_BULLET_ID, 30_900_151)] },
    RowSpec { table: &BULLET, template: 30_900_101, id: 30_900_151, name: "[Ultimate - Wylder] Onslaught Stake (Relic Ghostflame)",
        sets: &[Set::I32(bullet::HIT_BULLET_ID, 30_900_152)] },
    RowSpec { table: &BULLET, template: 30_900_102, id: 30_900_152, name: "[Ultimate - Wylder] Onslaught Stake (Relic Ghostflame) Damage",
        sets: &[Set::I32(bullet::ATK_ID, GHOST_GROUND_ATK as i32), Set::I32(bullet::SFX_ID, GHOSTFLAME_GROUND_SFX), Set::I32(bullet::SP_EFFECT_0, GHOSTFLAME_FROST)] },
    // 伤害：追踪者绝招基础攻击力只有物理 205 + 火 205（武器 60010000），没有魔力，所以按原版遗物地火的 26% 改为物理修正
    // （火伤会解除冻伤）；复用原版遗留的空闲行（见 content::IN_PLACE）
    RowSpec { table: &ATK_PC, template: 700_000_030, id: GHOST_GROUND_ATK, name: "[Ultimate - Wylder] Onslaught Stake (Relic Ghostflame)",
        sets: &[Set::U16(atk::FIRE_CORRECTION, 0), Set::U16(atk::PHYS_CORRECTION, 26), Set::U8(atk::SP_ATTRIBUTE, 10)] },
];
