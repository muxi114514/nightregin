//! 游戏函数 / 静态指针 / vtable 地址。
//! RVA 来自对当前 nightreign.exe 的静态比对（以 eldenring.exe 与 fromsoftware-rs 的特征码为参照），
//! 启动时逐项校验函数头字节与 RTTI 类型名；函数校验失败时尝试特征码兜底，仍失败则拒绝启用。

use log::{info, warn};

use super::memory::{read, read_bytes};
use super::module::GameModule;
use super::rtti::vtable_type_name;
use super::scan::{Pattern, find_all, rel32_target};

/// 结构体字段偏移（黑夜君临版本，均已在静态分析中交叉验证）
pub mod offsets {
    /// WorldChrManImp -> main_player（ER 为 0x1E508）
    pub const WCM_MAIN_PLAYER: usize = 0x174E8;
    /// ChrIns -> SpecialEffect*（ER 为 0x178，来自删除函数 thunk）
    pub const CHR_SPECIAL_EFFECT: usize = 0x190;
    /// SpecialEffect -> 链表头
    pub const SE_HEAD: usize = 0x8;
    /// 链表节点 -> SpEffect ID / 下一节点（ER 下一节点为 0x30）
    pub const SE_ENTRY_ID: usize = 0x8;
    pub const SE_ENTRY_NEXT: usize = 0x38;
}

mod midnight;
pub use midnight::{MidnightMenuAddresses, ReadyAddresses, resolve_midnight_menu, resolve_msg_lookup, resolve_ready};

#[derive(Debug, Clone)]
pub struct Addresses {
    pub apply_speffect: usize,
    pub remove_speffect: usize,
    pub register_task: usize,
    pub world_chr_man_static: usize,
    /// register_task 内部使用的 CSTaskGroupImp 静态指针（实测不是 CSTaskImp）
    pub task_group_static: usize,
    pub vt_world_chr_man: usize,
    pub vt_player_ins: usize,
    pub vt_player_game_data: usize,
    pub vt_task_group_imp: usize,
    pub menu_man_static: usize,
    pub vt_menu_man: usize,
    pub vt_popup_menu: usize,
    pub vt_antique_equip_dialog: usize,
    pub vt_main_dialog: usize,
    /// 打开锻造台强化界面（对话指令 c1_24(0)）；找不到时蓝10 不可用
    pub open_reinforce: Option<usize>,
    /// 混合魔法模块；校验不过时隐士1 不可用
    pub vt_mixcraft_module: Option<usize>,
    /// 参数仓库（实例由 .data 扫描得到）与其中的参数资源
    pub vt_param_repository: usize,
    pub vt_param_res_cap: usize,
}

/// 锻造石强化武器词条用到的地址；与属性分配分开校验，失败只停用该功能
#[derive(Debug, Clone)]
pub struct GaitemAddresses {
    pub change_item_id: usize,
    /// 强化提交调用 change_item_id 后的返回地址：只处理来自这里的调用
    pub reinforce_return: usize,
    pub gaitem_imp_static: usize,
    pub vt_gaitem_imp: usize,
    pub vt_wep_gaitem_ins: usize,
    /// 遗物实例；校验失败只停用遗物重铸
    pub vt_atq_gaitem_ins: Option<usize>,
}

/// 词条继承用到的地址；移除函数失败停用该功能
#[derive(Debug, Clone)]
pub struct InheritAddresses {
    pub remove_item: usize,
}

/// 写事件旗标的钩子（game::event_flags）用到的地址；函数找不到只停用对应钩子
#[derive(Debug, Clone)]
pub struct EventFlagAddresses {
    pub set_event_flag: Option<usize>,
    /// 找不到时继承请求仍会发给队友（接收端按"不请求同步"忽略）
    pub queue_net_sync: Option<usize>,
}

/// 三选一选项刷新用到的地址；任一校验失败只停用刷新
#[derive(Debug, Clone)]
pub struct PrivateItemAddresses {
    pub find_map_item: usize,
    pub map_item_man_static: usize,
    pub vt_map_item_man: usize,
    pub vt_choice_dialog: usize,
    pub vt_choice_item: usize,
    pub vt_grid_control: usize,
}

struct FnSpec {
    name: &'static str,
    rva: usize,
    prologue: &'static [u8],
    /// (调用点特征码, rel32 在特征码中的偏移)
    fallback: Option<(&'static str, usize)>,
}

const APPLY_SPEFFECT: FnSpec = FnSpec {
    name: "ChrIns::apply_speffect",
    rva: 0x3C7FD0,
    prologue: &[0x48, 0x8B, 0xC4, 0x48, 0x89, 0x58, 0x08, 0x48, 0x89, 0x70, 0x10, 0x57, 0x48, 0x81, 0xEC, 0xE0, 0x00, 0x00, 0x00],
    fallback: Some(("48 8B C8 45 33 C0 BA 7F 14 00 00 E8 ?? ?? ?? ??", 12)),
};

const REMOVE_SPEFFECT: FnSpec = FnSpec {
    name: "ChrIns::remove_speffect",
    rva: 0x3CDD40,
    prologue: &[0x48, 0x8B, 0x89, 0x90, 0x01, 0x00, 0x00, 0xE9],
    fallback: None,
};

/// 打开强化界面（rcx = 输出句柄 [指针, 令牌]）：取 CSPopupMenu，创建场景「02_013_Equip」压进弹窗；对话指令 c1_24(0) 的处理 0xFD3C40 调它
const OPEN_REINFORCE: FnSpec = FnSpec {
    name: "CSPopupMenu::open_reinforce",
    rva: 0x86B1B0,
    prologue: &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x57, 0x48, 0x83, 0xEC, 0x30, 0x48, 0x8B, 0xD9, 0x48, 0x8B, 0x05],
    fallback: None,
};

const REGISTER_TASK: FnSpec = FnSpec {
    name: "CSTaskImp::register_task",
    rva: 0xFEC580,
    prologue: &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x57, 0x48, 0x83, 0xEC, 0x40, 0x48, 0x8D, 0x4C, 0x24, 0x20, 0x49, 0x8B, 0xD8, 0x8B, 0xFA],
    fallback: Some(("E8 ?? ?? ?? ?? 48 8B 0D ?? ?? ?? ?? 4C 8B C7 8B D3 E8 ?? ?? ?? ??", 18)),
};

/// 按背包条目编号改物品 ID（rcx = PlayerGameData + 0x1D8，edx 条目编号，r8d 新 ID），武器已装备时随后重新装备
const CHANGE_ITEM_ID: FnSpec = FnSpec {
    name: "EquipGameData::change_item_id",
    rva: 0x1F3500,
    prologue: &[0x40, 0x53, 0x55, 0x56, 0x57, 0x41, 0x56, 0x48, 0x83, 0xEC, 0x60, 0x45, 0x8B, 0xF0, 0x8B, 0xFA],
    fallback: None,
};
/// 按背包条目编号增减数量（rcx = EquipGameData，edx 条目编号，r8d 增量，r9b 0）；减到 0 时先卸下装备槽再删条目。
/// 菜单丢弃 / 消耗都传负增量调用它（0x7EAB30 等 15 处）
const REMOVE_ITEM: FnSpec = FnSpec {
    name: "EquipGameData::add_item_count",
    rva: 0x1F06B0,
    prologue: &[0x48, 0x89, 0x5C, 0x24, 0x10, 0x44, 0x89, 0x44, 0x24, 0x18, 0x55, 0x56, 0x57, 0x41, 0x54, 0x41, 0x55, 0x41, 0x56],
    fallback: None,
};
/// CSEventFlagMan::set_event_flag（rcx = 管理器，rdx = &旗标，r8b 值，r9 调试上下文，栈上两个 bool：请求同步、日志）；
/// 对话脚本 SetEventFlag 命令（CSEzStateTalkEvent 0xFDF4E4）传请求同步 1，SetNetSyncEventFlag（0x5E7B70）写队友发来的旗标传 0
const SET_EVENT_FLAG: FnSpec = FnSpec {
    name: "CSEventFlagMan::set_event_flag",
    rva: 0x5E79F0,
    prologue: &[0x48, 0x89, 0x5C, 0x24, 0x20, 0x55, 0x56, 0x57, 0x48, 0x83, 0xEC, 0x30, 0x83, 0x3A, 0x00, 0x49, 0x8B, 0xE9],
    fallback: None,
};
/// 把旗标放进发给队友的同步队列（rcx = 管理器，rdx = &旗标，r8b 值）；对话脚本写完本地旗标后紧接着调用（0xFDF52B）
const QUEUE_NET_SYNC: FnSpec = FnSpec {
    name: "CSEventFlagMan::queue_net_sync",
    rva: 0x5E7470,
    prologue: &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x74, 0x24, 0x10, 0x57, 0x48, 0x83, 0xEC, 0x30, 0x41, 0x0F, 0xB6, 0xF0, 0x48, 0x8B, 0xDA],
    fallback: None,
};
/// 按编号查掉落物节点（rcx = MapItemManImpl，edx 编号），查不到返回 0；三选一界面取选项（0x572610）与选中发放都走它
const FIND_MAP_ITEM: FnSpec = FnSpec {
    name: "MapItemManImpl::find",
    rva: 0x582680,
    prologue: &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x89, 0x54, 0x24, 0x10, 0x55, 0x56, 0x57, 0x48, 0x8B, 0xEC, 0x48, 0x83, 0xEC, 0x20],
    fallback: None,
};
/// MapItemManImpl 静态指针（三选一工厂函数 ~0x9C6F90 读它）
const MAP_ITEM_MAN_STATIC_RVA: usize = 0x3C10B68;
const PRIVATE_ITEM_VTABLES: [(&str, usize, &str); 4] = [
    ("MapItemManImpl", 0x2C234C0, ".?AVMapItemManImpl@CS@@"),
    ("PrivateItemChoiceDialog", 0x2C5B020, ".?AVPrivateItemChoiceDialog@CS@@"),
    ("ChoiceViewItem", 0x2C5AF90, ".?AVChoiceViewItem@PrivateItemChoiceDialog@CS@@"),
    ("GridControl", 0x2C38200, ".?AVGridControl@CS@@"),
];
/// 强化提交（rva 0x8093B0）里调用 change_item_id 的位置：`add rcx, 0x1D8; call rel32`
const REINFORCE_CALL_RVA: usize = 0x809424;
const REINFORCE_CALL: &str = "48 81 C1 D8 01 00 00 E8 ?? ?? ?? ??";
const REINFORCE_CALL_LEN: usize = 12;
/// CSGaitemImp 静态指针（构造调用点 0xE22881 之后写入）
const GAITEM_IMP_STATIC_RVA: usize = 0x3C131C8;
const GAITEM_VTABLES: [(&str, usize, &str); 2] =
    [("CSGaitemImp", 0x2C32260, ".?AVCSGaitemImp@CS@@"), ("CSWepGaitemIns", 0x2C32488, ".?AVCSWepGaitemIns@CS@@")];
const ATQ_GAITEM_VTABLE: (&str, usize, &str) = ("CSAtqGaitemIns", 0x2C321D0, ".?AVCSAtqGaitemIns@CS@@");

const WORLD_CHR_MAN_STATIC_RVA: usize = 0x3C0F0A8;
/// CSMenuManImp 静态指针（运行时在 .data 中搜索指向该类实例的指针得到）
const MENU_MAN_STATIC_RVA: usize = 0x3C149D8;

const VTABLES: [(&str, usize, &str); 10] = [
    ("WorldChrManImp", 0x2C1B930, ".?AVWorldChrManImp@CS@@"),
    ("PlayerIns", 0x2C307F8, ".?AVPlayerIns@CS@@"),
    ("PlayerGameData", 0x2BE6E28, ".?AVPlayerGameData@CS@@"),
    ("CSTaskGroupImp", 0x2CDF868, ".?AVCSTaskGroupImp@CS@@"),
    ("CSMenuManImp", 0x2C3AD00, ".?AVCSMenuManImp@CS@@"),
    ("CSPopupMenu", 0x2C40B08, ".?AVCSPopupMenu@CS@@"),
    ("AntiqueEquipDialog", 0x2C4B078, ".?AVAntiqueEquipDialog@CS@@"),
    ("ParamRepositoryImp", 0x2CADC50, ".?AVParamRepositoryImp@CS@@"),
    ("ParamResCap", 0x2CADE48, ".?AVParamResCap@CS@@"),
    ("MainDialog", 0x2C5DEB8, ".?AVMainDialog@CS@@"),
];

fn resolve_fn(module: &GameModule, spec: &FnSpec) -> Option<usize> {
    let va = module.va(spec.rva);
    match read_bytes(va, spec.prologue.len()).map(|b| prologue_state(&b, spec.prologue)) {
        Some(Prologue::Original) => {
            info!("{} @ rva {:#x} verified", spec.name, spec.rva);
            return Some(va);
        }
        Some(Prologue::Hooked) => {
            info!("{} @ rva {:#x} verified (already hooked by another mod)", spec.name, spec.rva);
            return Some(va);
        }
        _ => {}
    }
    warn!("{} prologue mismatch at rva {:#x}, trying pattern fallback", spec.name, spec.rva);
    let (text, rel_offset) = spec.fallback?;
    let pattern = Pattern::parse(text)?;
    let hits = find_all(module, &pattern, 2);
    if hits.len() != 1 {
        warn!("{} fallback pattern hits = {}", spec.name, hits.len());
        return None;
    }
    let target = rel32_target(hits[0] + rel_offset)?;
    let ok = read_bytes(target, spec.prologue.len()).as_deref() == Some(spec.prologue);
    info!("{} fallback -> rva {:#x} prologue_ok={}", spec.name, target - module.base, ok);
    ok.then_some(target)
}

#[derive(Debug, PartialEq, Eq)]
enum Prologue {
    Original,
    /// 开头被别的模组换成 5 字节 jmp rel32，其余字节不变（如 MMV 的 nighter.dll 钩了 set_event_flag）。
    /// 照常调用 / 再钩都安全：MinHook 会把这条 jmp 改写成绝对跳转放进跳板，链式调用对方的钩子
    Hooked,
    Mismatch,
}

/// 被覆盖的指令若跨过第 5 字节，对方会用填充覆盖剩余部分，后面的字节就对不上，所以这个判断不会误认
fn prologue_state(actual: &[u8], expected: &[u8]) -> Prologue {
    const JMP_LEN: usize = 5;
    if actual == expected {
        Prologue::Original
    } else if actual.len() == expected.len() && expected.len() > JMP_LEN && actual[0] == 0xE9 && actual[JMP_LEN..] == expected[JMP_LEN..] {
        Prologue::Hooked
    } else {
        Prologue::Mismatch
    }
}

fn resolve_vtable(module: &GameModule, name: &str, rva: usize, expected: &str) -> Option<usize> {
    let va = module.va(rva);
    let actual = vtable_type_name(module, va);
    if actual.as_deref() == Some(expected) {
        info!("vtable {name} @ rva {rva:#x} verified");
        Some(va)
    } else {
        warn!("vtable {name} @ rva {rva:#x} mismatch: {actual:?}");
        None
    }
}

/// CSTaskGroupImp 静态指针：register_task 函数体 +0x19 处的 `mov rcx, [rip+rel32]`
fn task_group_static_from(register_task: usize) -> Option<usize> {
    let at = register_task + 0x19;
    let opcode: [u8; 3] = read(at)?;
    (opcode == [0x48, 0x8B, 0x0D]).then(|| rel32_target(at + 3)).flatten()
}

pub fn resolve(module: &GameModule) -> Result<Addresses, String> {
    let apply = resolve_fn(module, &APPLY_SPEFFECT).ok_or("apply_speffect not found")?;
    let remove = resolve_fn(module, &REMOVE_SPEFFECT).ok_or("remove_speffect not found")?;
    let register = resolve_fn(module, &REGISTER_TASK).ok_or("register_task not found")?;
    let task_group_static = task_group_static_from(register).ok_or("CSTaskGroupImp static not found")?;
    info!("CSTaskGroupImp static @ rva {:#x}", task_group_static - module.base);

    let mut vt = [0usize; VTABLES.len()];
    for (i, (name, rva, expected)) in VTABLES.iter().enumerate() {
        vt[i] = resolve_vtable(module, name, *rva, expected).ok_or(format!("vtable {name} not verified"))?;
    }

    Ok(Addresses {
        apply_speffect: apply,
        remove_speffect: remove,
        register_task: register,
        world_chr_man_static: module.va(WORLD_CHR_MAN_STATIC_RVA),
        task_group_static,
        vt_world_chr_man: vt[0],
        vt_player_ins: vt[1],
        vt_player_game_data: vt[2],
        vt_task_group_imp: vt[3],
        menu_man_static: module.va(MENU_MAN_STATIC_RVA),
        vt_menu_man: vt[4],
        vt_popup_menu: vt[5],
        vt_antique_equip_dialog: vt[6],
        vt_main_dialog: vt[9],
        open_reinforce: resolve_fn(module, &OPEN_REINFORCE),
        vt_mixcraft_module: resolve_vtable(module, "CSChrMixcraftMagicModule", 0x2C13448, ".?AVCSChrMixcraftMagicModule@CS@@"),
        vt_param_repository: vt[7],
        vt_param_res_cap: vt[8],
    })
}

pub fn resolve_gaitem(module: &GameModule) -> Result<GaitemAddresses, String> {
    let change_item_id = resolve_fn(module, &CHANGE_ITEM_ID).ok_or("change_item_id not found")?;
    let call = reinforce_call(module, change_item_id).ok_or("reinforce call site not found")?;
    info!("reinforce call site @ rva {:#x}", call - module.base);
    let mut vt = [0usize; GAITEM_VTABLES.len()];
    for (i, (name, rva, expected)) in GAITEM_VTABLES.iter().enumerate() {
        vt[i] = resolve_vtable(module, name, *rva, expected).ok_or(format!("vtable {name} not verified"))?;
    }
    let (name, rva, expected) = ATQ_GAITEM_VTABLE;
    Ok(GaitemAddresses {
        change_item_id,
        reinforce_return: call + REINFORCE_CALL_LEN,
        gaitem_imp_static: module.va(GAITEM_IMP_STATIC_RVA),
        vt_gaitem_imp: vt[0],
        vt_wep_gaitem_ins: vt[1],
        vt_atq_gaitem_ins: resolve_vtable(module, name, rva, expected),
    })
}

pub fn resolve_inherit(module: &GameModule) -> Result<InheritAddresses, String> {
    Ok(InheritAddresses { remove_item: resolve_fn(module, &REMOVE_ITEM).ok_or("add_item_count not found")? })
}

pub fn resolve_event_flags(module: &GameModule) -> EventFlagAddresses {
    EventFlagAddresses {
        set_event_flag: resolve_fn(module, &SET_EVENT_FLAG),
        queue_net_sync: resolve_fn(module, &QUEUE_NET_SYNC),
    }
}

pub fn resolve_private_item(module: &GameModule) -> Result<PrivateItemAddresses, String> {
    let find_map_item = resolve_fn(module, &FIND_MAP_ITEM).ok_or("MapItemManImpl::find not found")?;
    let mut vt = [0usize; PRIVATE_ITEM_VTABLES.len()];
    for (i, (name, rva, expected)) in PRIVATE_ITEM_VTABLES.iter().enumerate() {
        vt[i] = resolve_vtable(module, name, *rva, expected).ok_or(format!("vtable {name} not verified"))?;
    }
    Ok(PrivateItemAddresses {
        find_map_item,
        map_item_man_static: module.va(MAP_ITEM_MAN_STATIC_RVA),
        vt_map_item_man: vt[0],
        vt_choice_dialog: vt[1],
        vt_choice_item: vt[2],
        vt_grid_control: vt[3],
    })
}

/// 强化提交中调用 change_item_id 的指令地址：先查固定 RVA，不符时按特征码找调用目标为 change_item_id 的那一处
fn reinforce_call(module: &GameModule, change_item_id: usize) -> Option<usize> {
    let pattern = Pattern::parse(REINFORCE_CALL)?;
    let calls_target = |at: usize| {
        read_bytes(at, 7).as_deref() == Some(&[0x48, 0x81, 0xC1, 0xD8, 0x01, 0x00, 0x00][..])
            && rel32_target(at + 8) == Some(change_item_id)
    };
    let fixed = module.va(REINFORCE_CALL_RVA);
    if calls_target(fixed) {
        return Some(fixed);
    }
    warn!("reinforce call site mismatch at rva {REINFORCE_CALL_RVA:#x}, scanning");
    let hits: Vec<usize> = find_all(module, &pattern, 64).into_iter().filter(|&at| calls_target(at)).collect();
    (hits.len() == 1).then(|| hits[0])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_foreign_jmp_hook_only_when_rest_matches() {
        let expected = SET_EVENT_FLAG.prologue;
        assert_eq!(prologue_state(expected, expected), Prologue::Original);
        // 实机 MMV：nighter.dll 的钩子（2026-10-28 读取）
        let mut hooked = expected.to_vec();
        hooked[..5].copy_from_slice(&[0xE9, 0x83, 0x87, 0xFE, 0xBF]);
        assert_eq!(prologue_state(&hooked, expected), Prologue::Hooked);
        // jmp 后面跟着填充：被覆盖的指令跨过了第 5 字节
        hooked[5] = 0x90;
        assert_eq!(prologue_state(&hooked, expected), Prologue::Mismatch);
        assert_eq!(prologue_state(&expected[..10], expected), Prologue::Mismatch);
    }
}
