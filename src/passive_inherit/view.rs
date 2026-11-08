//! 游戏线程与词条继承面板之间交换的数据：出征中 6 个武器槽里的武器（游戏线程发布）、继承请求（面板发出）与结果。

use crate::game::gaitem::PASSIVE_COUNT;
use crate::game::inventory::WEAPON_SLOT_COUNT;

/// 选中的武器：背包条目编号 + 实例句柄 + 面板看到的词条槽，三者都对上才执行（防止选中后武器被换掉或改过）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WeaponPick {
    pub index: u32,
    pub handle: u32,
    pub passives: [i32; PASSIVE_COUNT],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InheritWeapon {
    pub pick: WeaponPick,
    /// 带强化等级的物品 ID
    pub item_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InheritView {
    /// 装备槽 0..6（0 左手 1、1 右手 1、2 左手 2、3 右手 2、4 左手 3、5 右手 3）；空槽、空手为 None
    pub slots: [Option<InheritWeapon>; WEAPON_SLOT_COUNT],
}

/// 来源的词条整把覆盖到目标上，来源随后消失
#[derive(Debug, Clone, Copy)]
pub struct InheritRequest {
    pub source: WeaponPick,
    pub target: WeaponPick,
}

#[derive(Debug, Clone)]
pub struct InheritOutcome {
    pub ok: bool,
    pub text: String,
}

impl From<Result<String, String>> for InheritOutcome {
    fn from(result: Result<String, String>) -> Self {
        match result {
            Ok(text) => Self { ok: true, text },
            Err(text) => Self { ok: false, text },
        }
    }
}
