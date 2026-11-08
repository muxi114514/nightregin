//! 背包物品的原生操作：按条目编号重新设置物品 ID（已装备时游戏会强制重新装备）、按条目编号移除物品。
//! 都会改动游戏状态，只能在游戏主线程（每帧任务）调用。

type ChangeItemIdFn = unsafe extern "C" fn(usize, u32, u32) -> usize;
type AddItemCountFn = unsafe extern "C" fn(usize, u32, i32, u8) -> i32;

#[derive(Clone, Copy)]
pub struct EquipOps {
    change_item_id: ChangeItemIdFn,
    add_item_count: AddItemCountFn,
}

impl EquipOps {
    pub fn new(change_item_id: usize, add_item_count: usize) -> Self {
        unsafe {
            Self {
                change_item_id: std::mem::transmute::<usize, ChangeItemIdFn>(change_item_id),
                add_item_count: std::mem::transmute::<usize, AddItemCountFn>(add_item_count),
            }
        }
    }

    /// 原 ID 再设一次：已装备的武器按实例里的新词条重建缓存、换掉被动效果（出征随机词条已实测）
    pub fn refresh(&self, equip_game_data: usize, index: u32, item_id: u32) {
        unsafe { (self.change_item_id)(equip_game_data, index, item_id) };
    }

    /// 数量减 1；武器只有 1 件，游戏会先卸下所在装备槽再删除条目。返回剩余数量
    pub fn remove_one(&self, equip_game_data: usize, index: u32) -> i32 {
        unsafe { (self.add_item_count)(equip_game_data, index, -1, 0) }
    }
}
