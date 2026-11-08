//! 菜单状态。实测布局（2026-10-25，界面开 / 关对照 + 指针扫描）：
//! CSMenuManImp+0xA0 -> CSPopupMenu；其 +0x10 为当前弹出的对话框，+0x58 非 0 表示弹窗显示中。
//! 关闭界面后 +0x10 不会清空，仍指向已释放的对象，所以必须同时检查 +0x58 与对象类型。
//! 遗物装配界面内打开"预设"子页时，当前对话框仍是 AntiqueEquipDialog。

use super::addresses::Addresses;
use super::memory::{looks_like_pointer, read, read_ptr};
use super::rtti::has_vtable;

const MENU_MAN_POPUP: usize = 0xA0;
const POPUP_DIALOG: usize = 0x10;
const POPUP_SHOWING: usize = 0x58;
/// 局内主菜单当前分页（实测 2026-11-02：0 地图 / 1 装备 / 2 状态 / 3 系统）
const MAIN_DIALOG_PAGE: usize = 0xED4;
const MAIN_PAGE_EQUIP: u32 = 1;

/// 遗物装配（含预设子页）是否正在显示
pub fn relic_equip_open(addrs: &Addresses) -> bool {
    showing_dialog(addrs, addrs.vt_antique_equip_dialog).is_some()
}

/// 局内主菜单是否停在「装备」分页。各分页是 MainDialog 的子对象，切走后 EquipDialog 也不销毁，只能看分页号
pub fn equip_open(addrs: &Addresses) -> bool {
    showing_dialog(addrs, addrs.vt_main_dialog).is_some_and(|d| read::<u32>(d + MAIN_DIALOG_PAGE) == Some(MAIN_PAGE_EQUIP))
}

/// 打开锻造台的强化界面，返回是否打开了。输出的句柄只是指针与令牌（0x867550 用它判断界面还在不在），不用释放
pub fn open_reinforce(addrs: &Addresses) -> bool {
    let Some(open) = addrs.open_reinforce else { return false };
    let open: extern "C" fn(*mut [usize; 2]) -> *mut [usize; 2] = unsafe { std::mem::transmute(open) };
    let mut handle = [0usize; 2];
    open(&mut handle);
    handle[0] != 0
}

/// 当前正在显示的对话框 (对象, 虚表)，不限类型；弹窗没显示时为 None
pub fn showing_any_dialog(addrs: &Addresses) -> Option<(usize, usize)> {
    let popup = popup_menu(addrs)?;
    if read::<u8>(popup + POPUP_SHOWING).unwrap_or(0) == 0 {
        return None;
    }
    let dialog = read_ptr(popup + POPUP_DIALOG).filter(|&d| looks_like_pointer(d))?;
    Some((dialog, read_ptr(dialog).filter(|&v| looks_like_pointer(v))?))
}

/// 当前显示的对话框是不是其中任一类型
pub fn showing_any(addrs: &Addresses, vtables: &[usize]) -> bool {
    vtables.iter().any(|&vt| showing_dialog(addrs, vt).is_some())
}

/// 正在显示的对话框（类型相符时）
pub(super) fn showing_dialog(addrs: &Addresses, vtable: usize) -> Option<usize> {
    let popup = popup_menu(addrs)?;
    if read::<u8>(popup + POPUP_SHOWING).unwrap_or(0) == 0 {
        return None;
    }
    read_ptr(popup + POPUP_DIALOG).filter(|&d| looks_like_pointer(d) && has_vtable(d, vtable))
}

fn popup_menu(addrs: &Addresses) -> Option<usize> {
    let menu = read_ptr(addrs.menu_man_static).filter(|&m| looks_like_pointer(m))?;
    if !has_vtable(menu, addrs.vt_menu_man) {
        return None;
    }
    let popup = read_ptr(menu + MENU_MAN_POPUP).filter(|&p| looks_like_pointer(p))?;
    has_vtable(popup, addrs.vt_popup_menu).then_some(popup)
}
