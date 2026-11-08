//! 追踪者追加攻击的入口：c0000.hks 有 5 处入口判断只认原版火焰追斩词条的标记 7020000。原版 HKS 是字节码，
//! 不便像 MMV 的源码那样加 or 条件，所以入口判断改为检查入口效果 GATE（tools\relic_assets.ps1 换常量），
//! 原版火焰标记与各元素词条标记都每 0.1 秒给自己施加 0.2 秒的 GATE（同大山羊护符的周期施加）。
//! GATE 不带状态值，不会触发火焰版的条件事件；高频施加，不做网络同步。

use crate::layout::{SP_EFFECT, sp_effect as sp};
use crate::spec::{RowSpec, Set};

/// 入口效果 ID：HKS 里按 float32 存，必须小于 2^24 才能精确表示
pub const GATE: u32 = 7_020_090;
/// 原版火焰追斩标记（见 content::IN_PLACE，原地加周期施加）
pub const FIRE_MARK: u32 = 7_020_000;
const GATE_SECONDS: f32 = 0.2;

/// 各词条标记追加的两个字段：每 0.1 秒施加一次入口效果
pub const MARK_INTERVAL: Set = Set::F32(sp::MOTION_INTERVAL, 0.1);
pub const MARK_CYCLE: Set = Set::I32(sp::CYCLE_OCCURRENCE, GATE as i32);

pub const ROWS: &[RowSpec] = &[
    RowSpec { table: &SP_EFFECT, template: FIRE_MARK, id: GATE, name: "[Skill - Wylder] Relic Followup Gate",
        sets: &[Set::U16(sp::STATE_INFO, 0), Set::F32(sp::EFFECT_ENDURANCE, GATE_SECONDS),
            Set::Bit(sp::NET_SYNC_BYTE, sp::NET_SYNC_DISABLE_BIT, true)] },
    RowSpec { table: &SP_EFFECT, template: FIRE_MARK, id: FIRE_MARK,
        name: "[Relic - Wylder] Follow-ups when using Character Skill (greatsword only)", sets: &[MARK_INTERVAL, MARK_CYCLE] },
];
