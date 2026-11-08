//! 空城 0.46 把第二段追加攻击（a900 动画 500021）的火焰附魔改成了血焰刀刃（7020005 / 7020009，空城自己的行）。
//! 动作文件改成按词条标记分支后（tools\relic_assets.ps1），原版火焰词条（状态 2027）在这一段沿用它：
//! 判定 6046 / 6047 → 子弹施加 7020005 / 7020009，写法同 wylder_ghostflame.rs 的火焰分支（6040 / 6041）。
//! 原版与 MMV 的动作文件不引用这两个判定，行在那里不起作用。

use crate::layout::{BEHAVIOR_PC, BULLET, atk, behavior, bullet};
use crate::spec::{RowSpec, Set};

pub const ROWS: &[RowSpec] = &[
    RowSpec { table: &BULLET, template: 2_900, id: 30_900_130, name: "[Skill - Wylder] Relic Followup 2nd Bloodflame Buff (Right, KongCheng)",
        sets: &[Set::I32(bullet::ATK_ID, atk::NO_TARGET), Set::F32(bullet::HIT_RADIUS, bullet::SELF_EFFECT_RADIUS),
            Set::I32(bullet::SP_EFFECT_FOR_SHOOTER, 7_020_005)] },
    RowSpec { table: &BULLET, template: 2_900, id: 30_900_131, name: "[Skill - Wylder] Relic Followup 2nd Bloodflame Buff (Left, KongCheng)",
        sets: &[Set::I32(bullet::ATK_ID, atk::NO_TARGET), Set::F32(bullet::HIT_RADIUS, bullet::SELF_EFFECT_RADIUS),
            Set::I32(bullet::SP_EFFECT_FOR_SHOOTER, 7_020_009)] },
    RowSpec { table: &BEHAVIOR_PC, template: 700_000_030, id: 600_000_046, name: "[Skill - Wylder] Relic Followup 2nd Bloodflame Buff (Right, KongCheng)",
        sets: &[Set::I32(behavior::JUDGE_ID, 46), Set::I32(behavior::REF_ID, 30_900_130)] },
    RowSpec { table: &BEHAVIOR_PC, template: 700_000_030, id: 600_000_047, name: "[Skill - Wylder] Relic Followup 2nd Bloodflame Buff (Left, KongCheng)",
        sets: &[Set::I32(behavior::JUDGE_ID, 47), Set::I32(behavior::REF_ID, 30_900_131)] },
];
