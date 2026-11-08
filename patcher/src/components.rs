//! 生成本模组的 SpEffect 行：属性组件以原版 7000000（"生命+1"遗物效果）为模板，回满效果以原版 110（满血满蓝满精力）为模板。
//! 组件为 6 秒的租约，由 DLL 每 2 秒续期；类别 20 让同 ID 重复施加只重置时长，存档类别 -1 不写入存档。

use nr_stat_alloc_shared::components::{self, RESTORE_FP, RESTORE_HP};
use nr_stat_alloc_shared::param::ParamFile;
use nr_stat_alloc_shared::param::layout::sp_effect as sp;

use crate::param_write::OwnedRow;

const COMPONENT_TEMPLATE: u32 = 7_000_000;
const RESTORE_TEMPLATE: u32 = 110;
const LEASE_SECONDS: f32 = 6.0;
const CATEGORY_REFRESH_ONLY: i16 = 20;
const SAVE_NEVER: i8 = -1;
const STAT_NAMES: [&str; components::STAT_COUNT] = ["Vig", "Mnd", "End", "Str", "Dex", "Int", "Fth", "Arc"];

/// 全部 2034 行（按 ID 升序）
pub fn rows(sp_effect: &ParamFile) -> Result<Vec<OwnedRow>, String> {
    let template = |id: u32| sp_effect.row(id).map(<[u8]>::to_vec).ok_or(format!("template SpEffect {id} missing"));
    let component = template(COMPONENT_TEMPLATE)?;
    let restore = template(RESTORE_TEMPLATE)?;

    let mut out = Vec::new();
    for (id, stat, value) in components::all() {
        let mut data = component.clone();
        for offset in sp::ADD_STATUS {
            data[offset] = 0;
        }
        data[sp::ADD_STATUS[stat]] = value as i8 as u8;
        set(&mut data, sp::SP_CATEGORY, &CATEGORY_REFRESH_ONLY.to_le_bytes());
        set(&mut data, sp::SAVE_CATEGORY, &SAVE_NEVER.to_le_bytes());
        set(&mut data, sp::EFFECT_ENDURANCE, &LEASE_SECONDS.to_le_bytes());
        out.push(OwnedRow { id: id as u32, data, name: Some(format!("[NRStatAlloc] {} {value:+}", STAT_NAMES[stat])) });
    }
    for (id, keep, name) in [(RESTORE_HP, sp::CHANGE_HP_RATE, "Restore HP"), (RESTORE_FP, sp::CHANGE_MP_RATE, "Restore FP")] {
        let mut data = restore.clone();
        for offset in [sp::CHANGE_HP_RATE, sp::CHANGE_MP_RATE, sp::CHANGE_STAMINA_RATE] {
            if offset != keep {
                set(&mut data, offset, &0f32.to_le_bytes());
            }
        }
        out.push(OwnedRow { id: id as u32, data, name: Some(format!("[NRStatAlloc] {name}")) });
    }
    Ok(out)
}

fn set(row: &mut [u8], offset: usize, value: &[u8]) {
    row[offset..offset + value.len()].copy_from_slice(value);
}
