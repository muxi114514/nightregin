//! 追踪者血焰 / 黑焰 / 命定之死版遗物词条：写法同灵火版（wylder_ghostflame.rs），每种元素两条，按 Element 表由宏生成。
//! - 追加攻击（7020060 / 7020070 / 7020080，标记状态 2194~2196）：追加攻击动画里的条件子弹事件施加附魔与刀身特效。
//!   附魔以原版火焰追斩遗物的左右手附魔 7020002 / 7020004 为模板（20 秒），换伤害属性、命中效果与附魔视觉，
//!   状态值改为 152（命中时对敌人施加效果，原版血焰 / 黑焰刀刃同此）：
//!   血焰 = 火 +20、命中「血焰刀刃」的出血 1632001；黑焰 = 火 +20、命中「黑焰刀刃」的黑焰 1626001；
//!   命定之死 = 圣 +20、命中黑刀「死亡之刃」的持续伤害 6845，并每 0.1 秒周期补挂降低血量上限
//!   （6860 的复制行，打开「自身来源可生效」，否则周期补挂会被拒，敌人血条没有灰色的上限削减）。
//!   附魔视觉复制原版黑焰左右手两行（8565 / 18880200）换中间特效：血焰 303171、黑焰 303161、命定之死 450827；
//!   附魔类特效会按刀身分段伸缩，原版在刀身 8 个挂点各生成一份。命定之死没有原版附魔特效，黑刀战技刀身的 450274 是固定长度
//!   的整段火焰刀，挂到大剑上会伸出刀尖，所以改用 tools\relic_sfx.ps1 把灵火附魔 303122 换成命定之死配色的 450827；
//!   命定之死地火同理用灵火地火 450822 换色的 450828。出刀时的刀身特效直接挂 0.7 秒右手附魔视觉。
//! - 绝招地火（7010570 / 7010580 / 7010590，标记状态 2197~2199）：绝招动画复制原版地火事件为条件事件 → 子弹链复制原版
//!   30900100→101→102，末段换地面特效、命中效果与攻击行。攻击行复用原版空闲行（见 content::IN_PLACE），伤害同原版火 26%
//!   （绝招武器只有物理和火的基础攻击力，命定之死也只能出火伤，只换命中特效与附加效果）。地面特效是原版里挑的最接近的，待实测。
//! 标记同灵火版每 0.1 秒施加追加攻击入口效果（follow_up_gate.rs）。动作文件（TAE）、HKS 与文本由 tools\relic_assets.ps1 处理。

use crate::layout::{
    ATK_PC, ATTACH_EFFECT, ATTACH_EFFECT_FILTER, BEHAVIOR_PC, BULLET, SP_EFFECT, SP_EFFECT_VFX, atk, attach_effect as ae,
    attach_effect_filter as filter, behavior, bullet, sp_effect as sp, sp_effect_vfx as vfx,
};
use crate::spec::{RowSpec, Set};

use super::follow_up_gate::{MARK_CYCLE, MARK_INTERVAL};

pub struct Element {
    /// 追加攻击词条（词条 / 筛选行 / 标记 SpEffect / 词条名同号）；其后依次为 +1 右手触发、+2 右手附魔、+3 左手触发、
    /// +4 左手附魔、+5 左手附魔视觉、+6 刀身特效、+7 命中效果（只有命定之死用）、+8 右手附魔视觉
    follow_up: u32,
    follow_up_state: u16,
    follow_up_buff_text: i32,
    /// 绝招词条（同号规则同上）
    ultimate: u32,
    ultimate_state: u16,
    ultimate_buff_text: i32,
    /// 遗物筛选排序值：追加攻击用它，绝招 +20（同原版火焰 10000010 / 10000030）
    filter_value: i32,
    fire_power: i32,
    holy_power: i32,
    on_hit: i32,
    sp_attribute: u8,
    sp_variation: u8,
    /// 附魔的第二条状态栏文本（第一条与右手附魔同号）
    enchant_text_2: i32,
    enchant_sfx: i32,
    enchant_se: i32,
    enchant_soul: u8,
    enchant_trace: u8,
    /// 附魔特效是否在刀身全部挂点生成（固定长度的特效只在刀根生成一份）
    full_blade: bool,
    /// 追加攻击三个判定（右手附魔、左手附魔、刀身特效）的首号（判定 % 1000，行为 600000000 + 号），子弹从 follow_up_bullet 起三个。
    /// 判定号取原版 / MMV / 空城包行为表都空着的段（空城 0.46 占了 60~67、地火 51~53），须与 tools\relic_assets.ps1 一致
    judge: u32,
    follow_up_bullet: u32,
    /// 地火判定号（判定 % 1000，行为 700000000 + 号），子弹从 ground_bullet 起三个（发射 → 散开 → 地火）
    ground_judge: u32,
    ground_bullet: u32,
    ground_atk: u32,
    ground_sfx: i32,
    ground_sp0: i32,
    ground_sp1: i32,
}

pub const BLOODFLAME: Element = Element {
    follow_up: 7_020_060, follow_up_state: 2194, follow_up_buff_text: 9_100_099,
    ultimate: 7_010_570, ultimate_state: 2197, ultimate_buff_text: 9_100_102,
    filter_value: 10_000_012, fire_power: 20, holy_power: 0, on_hit: 1_632_001, sp_attribute: FIRE, sp_variation: 3,
    enchant_text_2: 7_020_064, enchant_sfx: 303_171, enchant_se: 303_171, enchant_soul: 18, enchant_trace: 18, full_blade: true,
    judge: 170, follow_up_bullet: 30_900_120,
    ground_judge: 54, ground_bullet: 30_900_160, ground_atk: 700_000_807, ground_sfx: 524_134, ground_sp0: 1_632_001, ground_sp1: -1,
};
pub const BLACK_FLAME: Element = Element {
    follow_up: 7_020_070, follow_up_state: 2195, follow_up_buff_text: 9_100_100,
    ultimate: 7_010_580, ultimate_state: 2198, ultimate_buff_text: 9_100_103,
    filter_value: 10_000_013, fire_power: 20, holy_power: 0, on_hit: 1_626_001, sp_attribute: FIRE, sp_variation: 1,
    enchant_text_2: -1, enchant_sfx: 303_161, enchant_se: 303_161, enchant_soul: 17, enchant_trace: 17, full_blade: true,
    judge: 173, follow_up_bullet: 30_900_123,
    ground_judge: 55, ground_bullet: 30_900_170, ground_atk: 700_000_034, ground_sfx: 524_072, ground_sp0: 1_626_001, ground_sp1: -1,
};
pub const DESTINED_DEATH: Element = Element {
    follow_up: 7_020_080, follow_up_state: 2196, follow_up_buff_text: 9_100_101,
    ultimate: 7_010_590, ultimate_state: 2199, ultimate_buff_text: 9_100_104,
    filter_value: 10_000_014, fire_power: 0, holy_power: 20, on_hit: DEATH_ON_HIT as i32, sp_attribute: DESTINED_DEATH_ATTRIBUTE,
    sp_variation: 1, enchant_text_2: 7_020_084, enchant_sfx: 450_827, enchant_se: 303_161, enchant_soul: 17, enchant_trace: 17,
    full_blade: true,
    judge: 176, follow_up_bullet: 30_900_126,
    ground_judge: 56, ground_bullet: 30_900_180, ground_atk: 700_000_035, ground_sfx: 450_828, ground_sp0: DEATH_DOT as i32,
    ground_sp1: DEATH_MAX_HP_DOWN as i32,
};

const FIRE: u8 = 11;
const DESTINED_DEATH_ATTRIBUTE: u8 = 13;
/// 「命中时对敌人施加效果」：附魔的命中效果要靠它生效（原版火焰追斩附魔是 62「火焰附魔视觉」，没有命中效果）
const ENCHANT_STATE: u16 = 152;
/// 出刀时刀身特效的时长，同灵火版
const SWIRL_SECONDS: f32 = 0.7;
/// 命定之死命中效果：复制「死亡之刃」持续伤害，每 0.1 秒周期补挂降低血量上限。
/// 原版 6860 只允许由对手施加，周期触发是目标给自己挂，所以补挂的是打开「自身来源可生效」的复制行
const DEATH_ON_HIT: u32 = 7_020_087;
const DEATH_DOT: u32 = 6_845;
const DEATH_MAX_HP_DOWN: u32 = 6_860;
const DEATH_MAX_HP_DOWN_SELF: u32 = 7_020_089;

/// 原版火焰版模板：追加攻击标记 / 左右手触发与附魔、绝招标记、地火子弹链与行为、攻击行
const FIRE_FOLLOW_UP: u32 = 7_020_000;
const FIRE_TRIGGER_R: u32 = 7_020_001;
const FIRE_ENCHANT_R: u32 = 7_020_002;
const FIRE_TRIGGER_L: u32 = 7_020_003;
const FIRE_ENCHANT_L: u32 = 7_020_004;
const FIRE_ULTIMATE: u32 = 7_010_500;
const FIRE_GROUND: [u32; 3] = [30_900_100, 30_900_101, 30_900_102];
const FIRE_GROUND_BEHAVIOR: u32 = 700_000_030;
const FIRE_GROUND_ATK: u32 = 700_000_030;
/// 原版黑焰附魔视觉（右 / 左手），中性效果、空子弹
const BLACK_FLAME_VFX_R: u32 = 8_565;
const BLACK_FLAME_VFX_L: u32 = 18_880_200;
const NEUTRAL_EFFECT: u32 = 7_700;
const FX_BULLET: u32 = 2_900;
/// 附魔视觉第一个生成挂点（右 / 左手）
const RIGHT_HAND_DMY: i32 = 10_300;
const LEFT_HAND_DMY: i32 = 20_300;

/// 附魔视觉第 k 个生成挂点（k ≥ 2）：铺满刀身时同原版（10300、11300、10310、11310 … 10330、11330），否则不生成
const fn blade_dummy(full_blade: bool, hand: i32, k: i32) -> i32 {
    if full_blade { hand + (k % 2) * 1000 + (k / 2) * 10 } else { -1 }
}
const fn dmy_offset(k: usize) -> usize {
    vfx::ENCHANT_START_DMY + k * vfx::ENCHANT_DMY_STRIDE
}

/// 一种元素的全部行：追加攻击词条与附魔、附魔视觉、施加用子弹与行为；绝招词条与地火子弹链
macro_rules! element_rows {
    ($e:ident, $name:literal) => {
        &[
            RowSpec { table: &ATTACH_EFFECT, template: FIRE_FOLLOW_UP, id: $e.follow_up,
                name: concat!("[Relic - Wylder] ", $name, " follow-ups when using Character Skill (greatsword only)"),
                sets: &[Set::I32(ae::PASSIVE_SP_EFFECT, $e.follow_up as i32), Set::I32(ae::TEXT_ID, $e.follow_up as i32),
                    Set::I32(ae::FILTER_ID, $e.follow_up as i32)] },
            RowSpec { table: &ATTACH_EFFECT_FILTER, template: FIRE_FOLLOW_UP, id: $e.follow_up, name: concat!("[Relic - Wylder] ", $name, " follow-ups"),
                sets: &[Set::I32(filter::TEXT_ID, $e.follow_up as i32), Set::I32(filter::VALUE, $e.filter_value)] },
            RowSpec { table: &SP_EFFECT, template: FIRE_FOLLOW_UP, id: $e.follow_up,
                name: concat!("[Relic - Wylder] ", $name, " follow-ups when using Character Skill (greatsword only)"),
                sets: &[Set::U16(sp::STATE_INFO, $e.follow_up_state), Set::I32(sp::TEXT_ID_1, $e.follow_up_buff_text), MARK_INTERVAL, MARK_CYCLE] },
            // 附魔：触发（左右手条件同原版）→ 附魔
            RowSpec { table: &SP_EFFECT, template: FIRE_TRIGGER_R, id: $e.follow_up + 1, name: concat!("[Skill - Wylder] Relic Followup ", $name, " Buff (Right Trigger)"),
                sets: &[Set::I32(sp::CYCLE_OCCURRENCE, ($e.follow_up + 2) as i32)] },
            RowSpec { table: &SP_EFFECT, template: FIRE_ENCHANT_R, id: $e.follow_up + 2, name: concat!("[Skill - Wylder] Relic Followup ", $name, " Buff (Right Active)"),
                sets: &[Set::I32(sp::FIRE_ATTACK_POWER, $e.fire_power), Set::I32(sp::HOLY_ATTACK_POWER, $e.holy_power),
                    Set::I32(sp::ATK_OCCURRENCE, $e.on_hit), Set::U16(sp::STATE_INFO, ENCHANT_STATE), Set::U8(sp::SP_ATTRIBUTE, $e.sp_attribute),
                    Set::U8(sp::SP_ATTRIBUTE_VARIATION, $e.sp_variation), Set::I32(sp::VFX_ID, ($e.follow_up + 8) as i32),
                    Set::I32(sp::TEXT_ID_1, ($e.follow_up + 2) as i32), Set::I32(sp::TEXT_ID_2, $e.enchant_text_2)] },
            RowSpec { table: &SP_EFFECT, template: FIRE_TRIGGER_L, id: $e.follow_up + 3, name: concat!("[Skill - Wylder] Relic Followup ", $name, " Buff (Left Trigger)"),
                sets: &[Set::I32(sp::CYCLE_OCCURRENCE, ($e.follow_up + 4) as i32)] },
            RowSpec { table: &SP_EFFECT, template: FIRE_ENCHANT_L, id: $e.follow_up + 4, name: concat!("[Skill - Wylder] Relic Followup ", $name, " Buff (Left Active)"),
                sets: &[Set::I32(sp::FIRE_ATTACK_POWER, $e.fire_power), Set::I32(sp::HOLY_ATTACK_POWER, $e.holy_power),
                    Set::I32(sp::ATK_OCCURRENCE, $e.on_hit), Set::U16(sp::STATE_INFO, ENCHANT_STATE), Set::U8(sp::SP_ATTRIBUTE, $e.sp_attribute),
                    Set::U8(sp::SP_ATTRIBUTE_VARIATION, $e.sp_variation), Set::I32(sp::VFX_ID, ($e.follow_up + 5) as i32),
                    Set::I32(sp::TEXT_ID_1, ($e.follow_up + 2) as i32), Set::I32(sp::TEXT_ID_2, $e.enchant_text_2)] },
            // 附魔视觉（右 / 左手）与出刀时的刀身特效
            RowSpec { table: &SP_EFFECT_VFX, template: BLACK_FLAME_VFX_R, id: $e.follow_up + 8, name: concat!("[Skill - Wylder] Relic Followup ", $name, " Weapon VFX (Right)"),
                sets: &[Set::I32(vfx::MIDST_SFX, $e.enchant_sfx), Set::I32(vfx::MIDST_SE, $e.enchant_se),
                    Set::U8(vfx::ENCHANT_SOUL, $e.enchant_soul), Set::U8(vfx::ENCHANT_TRACE, $e.enchant_trace),
                    Set::I32(dmy_offset(2), blade_dummy($e.full_blade, RIGHT_HAND_DMY, 2)), Set::I32(dmy_offset(3), blade_dummy($e.full_blade, RIGHT_HAND_DMY, 3)),
                    Set::I32(dmy_offset(4), blade_dummy($e.full_blade, RIGHT_HAND_DMY, 4)), Set::I32(dmy_offset(5), blade_dummy($e.full_blade, RIGHT_HAND_DMY, 5)),
                    Set::I32(dmy_offset(6), blade_dummy($e.full_blade, RIGHT_HAND_DMY, 6)), Set::I32(dmy_offset(7), blade_dummy($e.full_blade, RIGHT_HAND_DMY, 7))] },
            RowSpec { table: &SP_EFFECT_VFX, template: BLACK_FLAME_VFX_L, id: $e.follow_up + 5, name: concat!("[Skill - Wylder] Relic Followup ", $name, " Weapon VFX (Left)"),
                sets: &[Set::I32(vfx::MIDST_SFX, $e.enchant_sfx), Set::I32(vfx::MIDST_SE, $e.enchant_se),
                    Set::U8(vfx::ENCHANT_SOUL, $e.enchant_soul), Set::U8(vfx::ENCHANT_TRACE, $e.enchant_trace),
                    Set::I32(dmy_offset(2), blade_dummy($e.full_blade, LEFT_HAND_DMY, 2)), Set::I32(dmy_offset(3), blade_dummy($e.full_blade, LEFT_HAND_DMY, 3)),
                    Set::I32(dmy_offset(4), blade_dummy($e.full_blade, LEFT_HAND_DMY, 4)), Set::I32(dmy_offset(5), blade_dummy($e.full_blade, LEFT_HAND_DMY, 5)),
                    Set::I32(dmy_offset(6), blade_dummy($e.full_blade, LEFT_HAND_DMY, 6)), Set::I32(dmy_offset(7), blade_dummy($e.full_blade, LEFT_HAND_DMY, 7))] },
            RowSpec { table: &SP_EFFECT, template: NEUTRAL_EFFECT, id: $e.follow_up + 6, name: concat!("[Skill - Wylder] Relic Followup ", $name, " Swirl"),
                sets: &[Set::F32(sp::EFFECT_ENDURANCE, SWIRL_SECONDS), Set::I32(sp::VFX_ID, ($e.follow_up + 8) as i32)] },
            // 施加用子弹（空攻击「No Target」、自身效果子弹半径，只给发射者加效果）与行为：右手附魔、左手附魔、刀身特效
            RowSpec { table: &BULLET, template: FX_BULLET, id: $e.follow_up_bullet, name: concat!("[Skill - Wylder] Relic Followup ", $name, " Buff (Right)"),
                sets: &[Set::I32(bullet::ATK_ID, atk::NO_TARGET), Set::F32(bullet::HIT_RADIUS, bullet::SELF_EFFECT_RADIUS),
                    Set::I32(bullet::SP_EFFECT_FOR_SHOOTER, ($e.follow_up + 1) as i32)] },
            RowSpec { table: &BULLET, template: FX_BULLET, id: $e.follow_up_bullet + 1, name: concat!("[Skill - Wylder] Relic Followup ", $name, " Buff (Left)"),
                sets: &[Set::I32(bullet::ATK_ID, atk::NO_TARGET), Set::F32(bullet::HIT_RADIUS, bullet::SELF_EFFECT_RADIUS),
                    Set::I32(bullet::SP_EFFECT_FOR_SHOOTER, ($e.follow_up + 3) as i32)] },
            RowSpec { table: &BULLET, template: FX_BULLET, id: $e.follow_up_bullet + 2, name: concat!("[Skill - Wylder] Relic Followup ", $name, " Swirl"),
                sets: &[Set::I32(bullet::ATK_ID, atk::NO_TARGET), Set::F32(bullet::HIT_RADIUS, bullet::SELF_EFFECT_RADIUS),
                    Set::I32(bullet::SP_EFFECT_FOR_SHOOTER, ($e.follow_up + 6) as i32)] },
            RowSpec { table: &BEHAVIOR_PC, template: FIRE_GROUND_BEHAVIOR, id: 600_000_000 + $e.judge, name: concat!("[Skill - Wylder] Relic Followup ", $name, " Buff (Right)"),
                sets: &[Set::I32(behavior::JUDGE_ID, $e.judge as i32), Set::I32(behavior::REF_ID, $e.follow_up_bullet as i32)] },
            RowSpec { table: &BEHAVIOR_PC, template: FIRE_GROUND_BEHAVIOR, id: 600_000_001 + $e.judge, name: concat!("[Skill - Wylder] Relic Followup ", $name, " Buff (Left)"),
                sets: &[Set::I32(behavior::JUDGE_ID, ($e.judge + 1) as i32), Set::I32(behavior::REF_ID, ($e.follow_up_bullet + 1) as i32)] },
            RowSpec { table: &BEHAVIOR_PC, template: FIRE_GROUND_BEHAVIOR, id: 600_000_002 + $e.judge, name: concat!("[Skill - Wylder] Relic Followup ", $name, " Swirl"),
                sets: &[Set::I32(behavior::JUDGE_ID, ($e.judge + 2) as i32), Set::I32(behavior::REF_ID, ($e.follow_up_bullet + 2) as i32)] },
            // 绝招：词条、筛选、标记
            RowSpec { table: &ATTACH_EFFECT, template: FIRE_ULTIMATE, id: $e.ultimate, name: concat!("[Relic - Wylder] Art activation spreads ", $name, " in area"),
                sets: &[Set::I32(ae::PASSIVE_SP_EFFECT, $e.ultimate as i32), Set::I32(ae::TEXT_ID, $e.ultimate as i32),
                    Set::I32(ae::FILTER_ID, $e.ultimate as i32)] },
            RowSpec { table: &ATTACH_EFFECT_FILTER, template: FIRE_ULTIMATE, id: $e.ultimate, name: concat!("[Relic - Wylder] Art spreads ", $name),
                sets: &[Set::I32(filter::TEXT_ID, $e.ultimate as i32), Set::I32(filter::VALUE, $e.filter_value + 20)] },
            RowSpec { table: &SP_EFFECT, template: FIRE_ULTIMATE, id: $e.ultimate, name: concat!("[Relic - Wylder] Art activation spreads ", $name, " in area"),
                sets: &[Set::U16(sp::STATE_INFO, $e.ultimate_state), Set::I32(sp::TEXT_ID_1, $e.ultimate_buff_text)] },
            // 地火：行为 → 发射 → 12 方向散开 → 地火
            RowSpec { table: &BEHAVIOR_PC, template: FIRE_GROUND_BEHAVIOR, id: 700_000_000 + $e.ground_judge, name: concat!("[Ultimate - Wylder] Onslaught Stake (Relic ", $name, ")"),
                sets: &[Set::I32(behavior::JUDGE_ID, $e.ground_judge as i32), Set::I32(behavior::REF_ID, $e.ground_bullet as i32)] },
            RowSpec { table: &BULLET, template: FIRE_GROUND[0], id: $e.ground_bullet, name: concat!("[Ultimate - Wylder] Onslaught Stake (Relic ", $name, ")"),
                sets: &[Set::I32(bullet::HIT_BULLET_ID, ($e.ground_bullet + 1) as i32)] },
            RowSpec { table: &BULLET, template: FIRE_GROUND[1], id: $e.ground_bullet + 1, name: concat!("[Ultimate - Wylder] Onslaught Stake (Relic ", $name, ")"),
                sets: &[Set::I32(bullet::HIT_BULLET_ID, ($e.ground_bullet + 2) as i32)] },
            RowSpec { table: &BULLET, template: FIRE_GROUND[2], id: $e.ground_bullet + 2, name: concat!("[Ultimate - Wylder] Onslaught Stake (Relic ", $name, ") Damage"),
                sets: &[Set::I32(bullet::ATK_ID, $e.ground_atk as i32), Set::I32(bullet::SFX_ID, $e.ground_sfx),
                    Set::I32(bullet::SP_EFFECT_0, $e.ground_sp0), Set::I32(bullet::SP_EFFECT_1, $e.ground_sp1)] },
            RowSpec { table: &ATK_PC, template: FIRE_GROUND_ATK, id: $e.ground_atk, name: concat!("[Ultimate - Wylder] Onslaught Stake (Relic ", $name, ")"),
                sets: &[Set::U8(atk::SP_ATTRIBUTE, $e.sp_attribute), Set::U8(atk::SP_ATTRIBUTE_VARIATION, $e.sp_variation)] },
        ]
    };
}

pub const BLOODFLAME_ROWS: &[RowSpec] = element_rows!(BLOODFLAME, "Bloodflame");
pub const BLACK_FLAME_ROWS: &[RowSpec] = element_rows!(BLACK_FLAME, "Black Flame");
pub const DESTINED_DEATH_ROWS: &[RowSpec] = element_rows!(DESTINED_DEATH, "Destined Death");
/// 命定之死额外的命中效果行（地火子弹由我方直接施加，仍用原版 6845 / 6860）
pub const EXTRA_ROWS: &[RowSpec] = &[
    RowSpec { table: &SP_EFFECT, template: DEATH_DOT, id: DEATH_ON_HIT, name: "[Skill - Wylder] Relic Followup Destined Death Hit",
        sets: &[Set::I32(sp::CYCLE_OCCURRENCE, DEATH_MAX_HP_DOWN_SELF as i32)] },
    RowSpec { table: &SP_EFFECT, template: DEATH_MAX_HP_DOWN, id: DEATH_MAX_HP_DOWN_SELF, name: "[Skill - Wylder] Relic Followup Destined Death Max HP Debuff",
        sets: &[Set::Bit(sp::TARGET_BYTE, sp::SELF_TARGET_BIT, true)] },
];

/// 地火攻击行：复用的原版空闲行（见 content::IN_PLACE）
pub const GROUND_ATKS: [u32; 3] = [BLOODFLAME.ground_atk, BLACK_FLAME.ground_atk, DESTINED_DEATH.ground_atk];
