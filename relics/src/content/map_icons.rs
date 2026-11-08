//! MMV 把 DLC 的湖沼 / 锻造村据点（SmallBaseAndSpotDefine 115 / 116）放进普通宁姆韦德，图标行 77 / 78 在普通地图上不显示。
//! 能显示的图标行（原版 7 / 4、MMV 自加的 122 / 126）的共同点：菜单图标 MENU_MenuIcon_<iconId> 在基础图集 01_common_h 里；
//! 77 / 78 的菜单图标 42047 / 42048 只在 DLC 图集里（换别的行号、只补地图图 MENU_MAP_<行> 都不显示）。
//! 照 MMV 加 122 / 126 的做法：用界面里有帧、参数里没有行的 124 / 125，菜单图标号用新的 42632 / 42633（接在 MMV 的 42630 / 42631 后），
//! 115 / 116 改指向它们；两种图都从 DLC 图集搬进基础图集（tools\dlc_map_icons.ps1）。
//! 两边必须一起改，所以只在 mmv_relics --mmv-map-icons 时加入（MMV 系 pkg）。

use crate::layout::{MAP_POINT_ICON, SMALL_BASE, map_point_icon, small_base};
use crate::spec::{RowSpec, Set};

const MARSH_SPOT: u32 = 115;
const SMITH_SPOT: u32 = 116;
const MARSH_DLC_ICON: u32 = 77;
const SMITH_DLC_ICON: u32 = 78;
const MARSH_ICON: u32 = 124;
const SMITH_ICON: u32 = 125;
const MARSH_MENU_ICON: i32 = 42_632;
const SMITH_MENU_ICON: i32 = 42_633;

pub const ROWS: &[RowSpec] = &[
    RowSpec { table: &MAP_POINT_ICON, template: MARSH_DLC_ICON, id: MARSH_ICON, name: "Marsh (base map)",
        sets: &[Set::I32(map_point_icon::ICON_ID, MARSH_MENU_ICON)] },
    RowSpec { table: &MAP_POINT_ICON, template: SMITH_DLC_ICON, id: SMITH_ICON, name: "Smith (base map)",
        sets: &[Set::I32(map_point_icon::ICON_ID, SMITH_MENU_ICON)] },
    RowSpec { table: &SMALL_BASE, template: MARSH_SPOT, id: MARSH_SPOT, name: "Encounter Setup: Marsh",
        sets: &[Set::U16(small_base::MAP_ICON, MARSH_ICON as u16)] },
    RowSpec { table: &SMALL_BASE, template: SMITH_SPOT, id: SMITH_SPOT, name: "Encounter Setup: Smith",
        sets: &[Set::U16(small_base::MAP_ICON, SMITH_ICON as u16)] },
];

/// 原地改写的据点行
pub const IN_PLACE: [u32; 2] = [MARSH_SPOT, SMITH_SPOT];
