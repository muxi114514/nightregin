//! 第六批（事件脚本）：靠改公共事件实现的锻体，标记效果的 ID 写死在 tools\tiered_events.ps1 里，改了要同步。
//! - 紫16：开启监牢的事件 90045000 先查手里有没有石剑钥匙、开门时扣 1 把；带标记时两处都跳过

use crate::layout::{SP_EFFECT, sp_effect as sp};
use crate::spec::{RowSpec, Set};

use super::pools::Candidate;
use super::rows::{ICON_DISCOVERY, MARKER_TEMPLATE, RARITY_PURPLE, SAVE, buff_rows, icon, text};

const GAOL_WITHOUT_KEY: u32 = 9_971_670;

pub const ROWS: &[RowSpec] = &[RowSpec {
    table: &SP_EFFECT,
    template: MARKER_TEMPLATE,
    id: GAOL_WITHOUT_KEY,
    name: "[Tiered Buff] Open gaols without Stonesword Keys",
    sets: &[Set::F32(sp::EFFECT_ENDURANCE, -1.0), SAVE, icon(ICON_DISCOVERY), text(GAOL_WITHOUT_KEY)],
}];

pub const BUFF_ROWS: &[RowSpec] =
    buff_rows![(GAOL_WITHOUT_KEY, GAOL_WITHOUT_KEY, ICON_DISCOVERY, RARITY_PURPLE, "Open gaols without Stonesword Keys")];

pub const PURPLE: &[Candidate] = &[Candidate::Buff(GAOL_WITHOUT_KEY)];
