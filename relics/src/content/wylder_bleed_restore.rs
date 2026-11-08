//! 补回 MMV 删掉的原版行：「【追踪者】技艺会附加引发异常状态出血」（词条 6500000 / 7011500，常驻状态值 2035）。
//! 原版：动作 a900 的钩爪命中事件（动画 500000 / 505000，状态条件 2035）调判定 6030 → 行为 600000030 → 攻击 600000030
//! （出血 7500001 = 60），拉近变体（500005 / 505005）调判定 6035 → 行为 / 攻击 600000035（出血 55）。
//! MMV 的 regulation 没有行为与攻击 600000030，钩爪主命中不再积累出血，只剩拉近变体，出血效率大幅下降。
//! - 攻击：新增 600000930（原版 / MMV / 空城包都没有；原先复用的 600000948 在空城 0.46 被子弹 200002453 引用），
//!   以 600000035 为模板改成原版 600000030 的数值（两行的全部差异，测试里逐字节核对）
//! - 行为：插入 600000030，指向上面的攻击行
//!
//! 原版独立版与空城 0.46 里两行本来就有：行为原地改写为指向新攻击行（数值与原版 600000030 相同），效果不变。

use crate::layout::{ATK_PC, BEHAVIOR_PC, atk, behavior};
use crate::spec::{RowSpec, Set};

/// 原版主命中的判定 / 行为 / 攻击
const JUDGE: i32 = 30;
pub const BEHAVIOR: u32 = 600_000_030;
/// 拉近变体（两份 regulation 都有），作模板
const PULL: u32 = 600_000_035;
/// 新增的攻击行
pub const ATK: u32 = 600_000_930;
/// 原版主命中的出血积累（60；拉近变体 7500002 为 55）
const BLEED: i32 = 7_500_001;

pub const ROWS: &[RowSpec] = &[
    RowSpec { table: &ATK_PC, template: PULL, id: ATK, name: "[Relic - Wylder] Character Skill inflicts Blood Loss",
        sets: &[
            Set::F32(atk::HIT0_RADIUS, 0.25),
            Set::I32(atk::SP_EFFECT_1, BLEED),
            Set::U16(atk::HIT1_DMY_POLY1, 0),
            Set::U16(atk::HIT1_DMY_POLY2, 0),
            Set::U8(atk::ATK_ATTRIBUTE, 2),
            Set::U8(atk::ATK_TYPE, 2),
            Set::U8(atk::ATK_DIR_FOR_SFX, 1),
            Set::U8(atk::ATK_BEHAVIOR_ID, 15),
        ] },
    RowSpec { table: &BEHAVIOR_PC, template: PULL, id: BEHAVIOR, name: "[Relic - Wylder] Character Skill inflicts Blood Loss",
        sets: &[Set::I32(behavior::JUDGE_ID, JUDGE), Set::I32(behavior::REF_ID, ATK as i32)] },
];

#[cfg(test)]
mod tests {
    use std::path::Path;

    use nr_stat_alloc_patcher::regulation::Regulation;

    use super::*;
    use crate::insert::open;

    /// 套用行定义后的模板行与原版 600000030 逐字节相同（原版文件不存在时跳过）
    #[test]
    fn restored_rows_match_vanilla() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Game/regulation.bin");
        let Ok(file) = std::fs::read(path) else { return };
        let reg = Regulation::decode(&file).unwrap();
        for spec in ROWS {
            let table = open(&reg, spec.table).unwrap();
            let mut data = table.row(spec.template).unwrap().to_vec();
            for set in spec.sets {
                set.apply(&mut data);
            }
            let mut expected = table.row(BEHAVIOR).unwrap().to_vec();
            if std::ptr::eq(spec.table, &BEHAVIOR_PC) {
                // 行为指向新增的攻击行，其余与原版相同
                Set::I32(behavior::REF_ID, ATK as i32).apply(&mut expected);
            }
            let diff: Vec<String> = data.iter().zip(&expected).enumerate().filter(|(_, (a, b))| a != b).map(|(i, (a, b))| format!("@{i}: {a} -> {b}")).collect();
            assert!(diff.is_empty(), "{}: {diff:?}", spec.table.file);
        }
    }
}
