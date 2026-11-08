//! 三选一界面与它的选项（2026-11-04 实机核对）：
//! - 界面 CS::PrivateItemChoiceDialog 在 CSPopupMenu 的当前弹窗槽位，内含列表控件 GridControl 与 3 个 ChoiceViewItem（打开时复制的选项）。
//! - 每个选项是 MapItemManImpl 里的一个掉落物节点，选中时按编号查节点发放，所以节点的物品决定实际到手。
//! - 列表文字只在重绘时读条目：改完条目要把 GridControl 的重绘标记置 1，游戏下一帧重绘并清零。

use super::addresses::{Addresses, PrivateItemAddresses};
use super::memory::{looks_like_pointer, read, read_ptr, write};
use super::menu;
use super::rtti::has_vtable;

const DIALOG_ORB: usize = 0xDE8;
const DIALOG_GRID: usize = 0xE20;
const GRID_REDRAW: usize = 0x94E;
const DIALOG_LIST: usize = 0x1770;
const LIST_ITEMS: usize = 0x8;
const LIST_COUNT: usize = 0x358;
const LIST_CAPACITY: u32 = 3;
const ITEM_STRIDE: usize = 0x118;
const ITEM_NODE_ID: usize = 0x8;
/// 条目里的物品编码有两份，重绘读的是这两处
const ITEM_CODES: [usize; 2] = [0x64, 0xC0];
const NODE_ITEM: usize = 0x28;
const NODE_ID: usize = 0x78;
/// u8 选项序号；u32 这一组掉落的首行
const NODE_OPTION: usize = 0xC9;
const NODE_LOT: usize = 0xD8;

/// 界面里的一个选项
#[derive(Clone, Copy, Debug)]
pub struct ChoiceOption {
    view: usize,
    node: usize,
    /// 物品编码（道具带种类位）
    pub item: u32,
    /// 这个选项对应的掉落行
    pub lot: u32,
}

/// 正在显示的三选一界面
pub struct ChoiceDialog {
    addr: usize,
    grid: usize,
}

impl ChoiceDialog {
    pub fn showing(addrs: &Addresses, private: &PrivateItemAddresses) -> Option<Self> {
        let addr = menu::showing_dialog(addrs, private.vt_choice_dialog)?;
        let grid = addr + DIALOG_GRID;
        has_vtable(grid, private.vt_grid_control).then_some(Self { addr, grid })
    }

    /// 这个球的编号（第一个选项的节点编号），关掉重开不变
    pub fn orb(&self) -> Option<u32> {
        read::<u32>(self.addr + DIALOG_ORB)
    }

    /// 全部选项；任一条目或节点对不上就不返回（宁可不显示按钮也不写错内存）
    pub fn options(&self, private: &PrivateItemAddresses) -> Option<Vec<ChoiceOption>> {
        let list = self.addr + DIALOG_LIST;
        let count = read::<u32>(list + LIST_COUNT).filter(|c| (1..=LIST_CAPACITY).contains(c))?;
        (0..count as usize)
            .map(|i| {
                let view = list + LIST_ITEMS + i * ITEM_STRIDE;
                if !has_vtable(view, private.vt_choice_item) {
                    return None;
                }
                let node = find_node(private, read::<u32>(view + ITEM_NODE_ID)?)?;
                let item = read::<u32>(node + NODE_ITEM)?;
                let matches = ITEM_CODES.iter().all(|&at| read::<u32>(view + at) == Some(item));
                let lot = read::<u32>(node + NODE_LOT)?.checked_add(read::<u8>(node + NODE_OPTION)? as u32)?;
                matches.then_some(ChoiceOption { view, node, item, lot })
            })
            .collect()
    }

    /// 换掉一个选项的物品：节点决定实际到手，条目决定重绘内容
    pub fn replace(&self, option: &ChoiceOption, item: u32) -> bool {
        let node = write(option.node + NODE_ITEM, item);
        ITEM_CODES.iter().fold(node, |ok, &at| write(option.view + at, item) && ok)
    }

    /// 请求重绘列表（虚函数 4 做的就是这一步）
    pub fn redraw(&self) -> bool {
        write(self.grid + GRID_REDRAW, 1u8)
    }
}

/// 球还在不在（被选走后节点就查不到了）
pub fn orb_exists(private: &PrivateItemAddresses, orb: u32) -> bool {
    find_node(private, orb).is_some()
}

fn find_node(private: &PrivateItemAddresses, id: u32) -> Option<usize> {
    let man = read_ptr(private.map_item_man_static).filter(|&m| looks_like_pointer(m) && has_vtable(m, private.vt_map_item_man))?;
    let find: extern "C" fn(usize, u32) -> usize = unsafe { std::mem::transmute(private.find_map_item) };
    Some(find(man, id)).filter(|&n| looks_like_pointer(n) && read::<u32>(n + NODE_ID) == Some(id))
}
