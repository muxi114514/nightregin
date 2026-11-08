//! 7010550「发动绝招时，能燃起灵火、蔓延周围」的附带效果：绝招本身的大爆炸也换成灵火。
//! 照搬原版铁之眼毒箭遗物的替换机制：绝招动画开头无条件施加「火焰爆炸」标记 7010560（状态 2192），
//! 灵火词条的常驻效果每 0.1 秒施加「灵火爆炸」标记 7010561（状态 2193）把它顶掉。两个标记用类别 200 + 独占优先级
//! （顶掉同类别同优先级的旧效果，同原版成对开关；类别必须是引擎认的值，否则两个标记共存、火焰与灵火爆炸一起触发）。
//! 动画里原爆炸事件（判定 7001 蓄力 / 7006 不蓄力 / 7015 空中）改为要求 2192，并加要求 2193 的灵火版
//! （判定 7002 / 7007 / 7017 → 子弹链末段换成死亡勾棒「灵火点燃」爆炸 450823，命中积累冻伤 903）。
//! 伤害：蓄力爆炸原版为物理 150% + 火 150%（绝招基础攻击力只有物理和火），灵火版合并为物理 300%（总量不变、不带火，火伤会解冻），
//! 写在复用的原版空闲攻击行（见 content::IN_PLACE）；不蓄力 / 空中爆炸原版攻击行本就不造成伤害（只削韧），沿用原版行；
//! 爆炸后的余火特效 460002 改为要求 2192 的子弹事件施加的特效效果（判定 7003 / 7004 / 7005 / 7008 对应各动画的余火时长），
//! 写法照搬同动画里原版判定 7490（身体挂点、不占槽位），子弹半径同原版自身效果子弹，不与爆炸争抢命中。
//! 动作文件部分见 tools\relic_assets.ps1。

use crate::layout::{ATK_PC, BEHAVIOR_PC, BULLET, SP_EFFECT, SP_EFFECT_VFX, atk, behavior, bullet, sp_effect as sp, sp_effect_vfx as vfx};
use crate::spec::{RowSpec, Set};

pub const GHOST_BLAST_MARK: i32 = 7_010_561;
/// 灵火蓄力爆炸的攻击行：复用的原版空闲行（见 content::IN_PLACE）
pub const GHOST_BLAST_ATK: u32 = 700_000_806;
const BLAST_CATEGORY: u16 = 200;
/// 类别 200 里原版未用的优先级
const BLAST_PRIORITY: u8 = 70;
const GHOST_BLAST_SFX: i32 = 450_823;
const GHOST_BLAST_FROST: i32 = 903;
/// 原版铁之眼普通 / 毒箭绝招标记（永久）作模板
const REGULAR_TEMPLATE: u32 = 7_010_601;
const RELIC_TEMPLATE: u32 = 7_034_601;
const NEUTRAL_EFFECT: u32 = 7_700;
const FX_BULLET: u32 = 2_900;
/// 原版判定 7490 的行为（绝招后给队友加效果），余火事件照搬它的事件参数
const AFTERFIRE_BEHAVIOR_TEMPLATE: u32 = 700_000_490;
const AFTERFIRE_VFX: i32 = 7_010_562;

const NAME_GHOST: &str = "[Ultimate - Wylder] Onslaught Stake (Relic Ghostflame Blast)";
const NAME_AFTERFIRE: &str = "[Ultimate - Wylder] Onslaught Stake Afterfire";

pub const ROWS: &[RowSpec] = &[
    // 替换标记
    RowSpec { table: &SP_EFFECT, template: REGULAR_TEMPLATE, id: 7_010_560, name: "[Ultimate - Wylder] Regular Blast",
        sets: &[Set::U16(sp::STATE_INFO, 2192), Set::U16(sp::SP_CATEGORY, BLAST_CATEGORY), Set::U8(sp::CATEGORY_PRIORITY, BLAST_PRIORITY)] },
    RowSpec { table: &SP_EFFECT, template: RELIC_TEMPLATE, id: GHOST_BLAST_MARK as u32, name: "[Relic - Wylder] Ghostflame Blast",
        sets: &[Set::U16(sp::STATE_INFO, 2193), Set::U16(sp::SP_CATEGORY, BLAST_CATEGORY), Set::U8(sp::CATEGORY_PRIORITY, BLAST_PRIORITY)] },
    // 灵火爆炸：行为 → 发射子弹 → 爆炸子弹（特效 / 冻伤 / 攻击）；蓄力 / 不蓄力 / 空中各一套
    RowSpec { table: &BEHAVIOR_PC, template: 700_000_001, id: 700_000_002, name: NAME_GHOST,
        sets: &[Set::I32(behavior::JUDGE_ID, 2), Set::I32(behavior::REF_ID, 30_900_030)] },
    RowSpec { table: &BULLET, template: 30_900_000, id: 30_900_030, name: NAME_GHOST, sets: &[Set::I32(bullet::HIT_BULLET_ID, 30_900_031)] },
    RowSpec { table: &BULLET, template: 30_900_001, id: 30_900_031, name: NAME_GHOST,
        sets: &[Set::I32(bullet::ATK_ID, GHOST_BLAST_ATK as i32), Set::I32(bullet::SFX_ID, GHOST_BLAST_SFX), Set::I32(bullet::SP_EFFECT_0, GHOST_BLAST_FROST)] },
    RowSpec { table: &ATK_PC, template: 700_000_001, id: GHOST_BLAST_ATK, name: NAME_GHOST,
        sets: &[Set::U16(atk::PHYS_CORRECTION, 300), Set::U16(atk::MAG_CORRECTION, 0), Set::U16(atk::FIRE_CORRECTION, 0),
            Set::U16(atk::THUN_CORRECTION, 0), Set::U16(atk::DARK_CORRECTION, 0), Set::U8(atk::SP_ATTRIBUTE, 10)] },
    RowSpec { table: &BEHAVIOR_PC, template: 700_000_006, id: 700_000_007, name: NAME_GHOST,
        sets: &[Set::I32(behavior::JUDGE_ID, 7), Set::I32(behavior::REF_ID, 30_900_035)] },
    RowSpec { table: &BULLET, template: 30_900_005, id: 30_900_035, name: NAME_GHOST, sets: &[Set::I32(bullet::HIT_BULLET_ID, 30_900_036)] },
    RowSpec { table: &BULLET, template: 30_900_006, id: 30_900_036, name: NAME_GHOST,
        sets: &[Set::I32(bullet::SFX_ID, GHOST_BLAST_SFX), Set::I32(bullet::SP_EFFECT_0, GHOST_BLAST_FROST)] },
    RowSpec { table: &BEHAVIOR_PC, template: 700_000_015, id: 700_000_017, name: NAME_GHOST,
        sets: &[Set::I32(behavior::JUDGE_ID, 17), Set::I32(behavior::REF_ID, 30_900_045)] },
    RowSpec { table: &BULLET, template: 30_900_015, id: 30_900_045, name: NAME_GHOST, sets: &[Set::I32(bullet::HIT_BULLET_ID, 30_900_046)] },
    RowSpec { table: &BULLET, template: 30_900_016, id: 30_900_046, name: NAME_GHOST,
        sets: &[Set::I32(bullet::SFX_ID, GHOST_BLAST_SFX), Set::I32(bullet::SP_EFFECT_0, GHOST_BLAST_FROST)] },
    // 爆炸后的余火：特效行 → 各动画时长的特效效果 → 施加用子弹 → 行为
    RowSpec { table: &SP_EFFECT_VFX, template: 4, id: AFTERFIRE_VFX as u32, name: NAME_AFTERFIRE,
        sets: &[Set::I32(vfx::MIDST_SFX, 460_002), Set::U16(vfx::MIDST_DMY, 451)] },
    RowSpec { table: &SP_EFFECT, template: NEUTRAL_EFFECT, id: 7_010_562, name: NAME_AFTERFIRE,
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, 1.2), Set::I32(sp::VFX_ID, AFTERFIRE_VFX)] },
    RowSpec { table: &SP_EFFECT, template: NEUTRAL_EFFECT, id: 7_010_563, name: NAME_AFTERFIRE,
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, 0.8), Set::I32(sp::VFX_ID, AFTERFIRE_VFX)] },
    RowSpec { table: &SP_EFFECT, template: NEUTRAL_EFFECT, id: 7_010_564, name: NAME_AFTERFIRE,
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, 1.1), Set::I32(sp::VFX_ID, AFTERFIRE_VFX)] },
    RowSpec { table: &SP_EFFECT, template: NEUTRAL_EFFECT, id: 7_010_565, name: NAME_AFTERFIRE,
        sets: &[Set::F32(sp::EFFECT_ENDURANCE, 0.833), Set::I32(sp::VFX_ID, AFTERFIRE_VFX)] },
    RowSpec { table: &BULLET, template: FX_BULLET, id: 30_900_116, name: NAME_AFTERFIRE, sets: &[Set::I32(bullet::ATK_ID, atk::NO_TARGET), Set::F32(bullet::HIT_RADIUS, bullet::SELF_EFFECT_RADIUS),
            Set::I32(bullet::SP_EFFECT_FOR_SHOOTER, 7_010_562)] },
    RowSpec { table: &BULLET, template: FX_BULLET, id: 30_900_117, name: NAME_AFTERFIRE, sets: &[Set::I32(bullet::ATK_ID, atk::NO_TARGET), Set::F32(bullet::HIT_RADIUS, bullet::SELF_EFFECT_RADIUS),
            Set::I32(bullet::SP_EFFECT_FOR_SHOOTER, 7_010_563)] },
    RowSpec { table: &BULLET, template: FX_BULLET, id: 30_900_118, name: NAME_AFTERFIRE, sets: &[Set::I32(bullet::ATK_ID, atk::NO_TARGET), Set::F32(bullet::HIT_RADIUS, bullet::SELF_EFFECT_RADIUS),
            Set::I32(bullet::SP_EFFECT_FOR_SHOOTER, 7_010_564)] },
    RowSpec { table: &BULLET, template: FX_BULLET, id: 30_900_119, name: NAME_AFTERFIRE, sets: &[Set::I32(bullet::ATK_ID, atk::NO_TARGET), Set::F32(bullet::HIT_RADIUS, bullet::SELF_EFFECT_RADIUS),
            Set::I32(bullet::SP_EFFECT_FOR_SHOOTER, 7_010_565)] },
    RowSpec { table: &BEHAVIOR_PC, template: AFTERFIRE_BEHAVIOR_TEMPLATE, id: 700_000_003, name: NAME_AFTERFIRE,
        sets: &[Set::I32(behavior::JUDGE_ID, 3), Set::I32(behavior::REF_ID, 30_900_116)] },
    RowSpec { table: &BEHAVIOR_PC, template: AFTERFIRE_BEHAVIOR_TEMPLATE, id: 700_000_004, name: NAME_AFTERFIRE,
        sets: &[Set::I32(behavior::JUDGE_ID, 4), Set::I32(behavior::REF_ID, 30_900_117)] },
    RowSpec { table: &BEHAVIOR_PC, template: AFTERFIRE_BEHAVIOR_TEMPLATE, id: 700_000_005, name: NAME_AFTERFIRE,
        sets: &[Set::I32(behavior::JUDGE_ID, 5), Set::I32(behavior::REF_ID, 30_900_118)] },
    RowSpec { table: &BEHAVIOR_PC, template: AFTERFIRE_BEHAVIOR_TEMPLATE, id: 700_000_008, name: NAME_AFTERFIRE,
        sets: &[Set::I32(behavior::JUDGE_ID, 8), Set::I32(behavior::REF_ID, 30_900_119)] },
];
