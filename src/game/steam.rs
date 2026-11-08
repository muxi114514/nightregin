//! 游戏自带 steam_api64.dll 的平面接口（大厅、用户、好友）。导出按名字解析，任一缺失则整体不可用。
//! Steam 没有「我当前在哪个大厅」的查询，只能靠 LobbyEnter 回调：回调跑在 Steam 分发线程，只把大厅 ID 放进有界队列，由每帧任务取走。

use std::collections::VecDeque;
use std::ffi::{CStr, CString, c_char, c_void};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use log::{info, warn};
use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleA, GetProcAddress};

/// k_iSteamMatchmakingCallbacks + 4
const LOBBY_ENTER_ID: i32 = 504;
/// k_EChatRoomEnterResponseSuccess
const ENTER_SUCCESS: u32 = 1;
const QUEUE_CAP: usize = 32;
const MAX_MEMBER_SCAN: i32 = 64;

/// LobbyEnter_t（Windows 下按 8 字节对齐打包）
#[repr(C)]
#[allow(dead_code)] // 字段由 Steam 写入，只为占位
struct LobbyEnter {
    lobby: u64,
    permissions: u32,
    /// 原类型 bool，用 u8 避免读到非 0 / 1 时的未定义行为
    locked: u8,
    response: u32,
}

/// CCallbackBase 的内存布局（虚表 + 标志 + 回调号，后两者由 SteamAPI_RegisterCallback 写入）
#[repr(C)]
#[allow(dead_code)] // 字段由 Steam 读写
struct CallbackBase {
    vtable: *const CallbackVtable,
    flags: u8,
    id: i32,
}

/// 两个 Run 重载在虚表里的先后取决于编译器，这里都填同一个函数（只用前两个参数，x64 下多传的参数无影响）
#[repr(C)]
#[allow(dead_code)] // 由 Steam 经虚表调用
struct CallbackVtable {
    run_a: unsafe extern "C" fn(*mut CallbackBase, *const LobbyEnter),
    run_b: unsafe extern "C" fn(*mut CallbackBase, *const LobbyEnter),
    size: unsafe extern "C" fn(*mut CallbackBase) -> i32,
}

/// 回调线程写、每帧任务取，有上限防止长期不取时无限增长
struct EnteredQueue(Mutex<VecDeque<u64>>);

impl EnteredQueue {
    const fn new() -> Self {
        Self(Mutex::new(VecDeque::new()))
    }

    fn record(&self, event: &LobbyEnter) {
        if event.response != ENTER_SUCCESS {
            return;
        }
        let mut q = self.0.lock().unwrap_or_else(|e| e.into_inner());
        if q.len() >= QUEUE_CAP {
            q.pop_front();
        }
        q.push_back(event.lobby);
    }

    fn drain(&self) -> Vec<u64> {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).drain(..).collect()
    }
}

static ENTERED: EnteredQueue = EnteredQueue::new();
static VTABLE: CallbackVtable = CallbackVtable { run_a: on_lobby_enter, run_b: on_lobby_enter, size: callback_size };

unsafe extern "C" fn on_lobby_enter(_this: *mut CallbackBase, param: *const LobbyEnter) {
    // panic 不能穿过 FFI 边界
    let _ = catch_unwind(AssertUnwindSafe(|| {
        let Some(event) = (unsafe { param.as_ref() }) else { return };
        info!("steam: entered lobby ..{} (response {})", event.lobby % 1_000_000, event.response);
        ENTERED.record(event);
    }));
}

unsafe extern "C" fn callback_size(_this: *mut CallbackBase) -> i32 {
    size_of::<LobbyEnter>() as i32
}

type Iface = *mut c_void;
type RegisterFn = unsafe extern "C" fn(*mut CallbackBase, i32);

fn register_lobby_enter(register: RegisterFn) {
    static DONE: AtomicBool = AtomicBool::new(false);
    if DONE.swap(true, Ordering::AcqRel) {
        return;
    }
    // Steam 一直持有这个对象，所以泄漏（进程内只有一个）
    let object = Box::leak(Box::new(CallbackBase { vtable: &VTABLE, flags: 0, id: 0 }));
    unsafe { register(object, LOBBY_ENTER_ID) };
    info!("steam: LobbyEnter callback registered");
}

fn export<T: Copy>(module: isize, name: &CStr) -> Option<T> {
    const { assert!(size_of::<T>() == size_of::<usize>()) };
    let Some(p) = (unsafe { GetProcAddress(module, name.as_ptr().cast()) }) else {
        warn!("steam_api64 export missing: {name:?}");
        return None;
    };
    Some(unsafe { std::mem::transmute_copy(&p) })
}

fn text(p: *const c_char) -> Option<String> {
    if p.is_null() {
        return None;
    }
    let s = unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned();
    (!s.is_empty()).then_some(s)
}

fn iface(getter: unsafe extern "C" fn() -> Iface) -> Option<Iface> {
    let p = unsafe { getter() };
    (!p.is_null()).then_some(p)
}

pub struct SteamApi {
    register: RegisterFn,
    matchmaking: unsafe extern "C" fn() -> Iface,
    user: unsafe extern "C" fn() -> Iface,
    friends: unsafe extern "C" fn() -> Iface,
    release_memory: unsafe extern "C" fn(),
    get_owner: unsafe extern "C" fn(Iface, u64) -> u64,
    member_count: unsafe extern "C" fn(Iface, u64) -> i32,
    member_at: unsafe extern "C" fn(Iface, u64, i32) -> u64,
    get_member_data: unsafe extern "C" fn(Iface, u64, u64, *const c_char) -> *const c_char,
    put_member_data: unsafe extern "C" fn(Iface, u64, *const c_char, *const c_char),
    get_steam_id: unsafe extern "C" fn(Iface) -> u64,
    get_persona: unsafe extern "C" fn(Iface, u64) -> *const c_char,
}

impl SteamApi {
    pub fn load() -> Option<Self> {
        let module = unsafe { GetModuleHandleA(c"steam_api64.dll".as_ptr().cast()) };
        if module == 0 {
            warn!("steam_api64.dll not loaded");
            return None;
        }
        Some(Self {
            register: export(module, c"SteamAPI_RegisterCallback")?,
            matchmaking: export(module, c"SteamAPI_SteamMatchmaking_v009")?,
            user: export(module, c"SteamAPI_SteamUser_v021")?,
            friends: export(module, c"SteamAPI_SteamFriends_v017")?,
            release_memory: export(module, c"SteamAPI_ReleaseCurrentThreadMemory")?,
            get_owner: export(module, c"SteamAPI_ISteamMatchmaking_GetLobbyOwner")?,
            member_count: export(module, c"SteamAPI_ISteamMatchmaking_GetNumLobbyMembers")?,
            member_at: export(module, c"SteamAPI_ISteamMatchmaking_GetLobbyMemberByIndex")?,
            get_member_data: export(module, c"SteamAPI_ISteamMatchmaking_GetLobbyMemberData")?,
            put_member_data: export(module, c"SteamAPI_ISteamMatchmaking_SetLobbyMemberData")?,
            get_steam_id: export(module, c"SteamAPI_ISteamUser_GetSteamID")?,
            get_persona: export(module, c"SteamAPI_ISteamFriends_GetFriendPersonaName")?,
        })
    }

    /// 取走新进入的大厅（按进入顺序）。第一次调用时注册回调：放在游戏线程上做，不与 SteamAPI_RunCallbacks 并发；
    /// 每帧任务从启动起就在跑，早于玩家进大厅
    pub fn take_entered(&self) -> Vec<u64> {
        register_lobby_enter(self.register);
        ENTERED.drain()
    }

    pub fn self_id(&self) -> Option<u64> {
        let id = unsafe { (self.get_steam_id)(iface(self.user)?) };
        (id != 0).then_some(id)
    }

    /// 不在该大厅时为 None
    pub fn owner(&self, lobby: u64) -> Option<u64> {
        let id = unsafe { (self.get_owner)(iface(self.matchmaking)?, lobby) };
        (id != 0).then_some(id)
    }

    /// 不在该大厅时为空
    pub fn members(&self, lobby: u64) -> Vec<u64> {
        let Some(mm) = iface(self.matchmaking) else { return Vec::new() };
        // Steam 要求先取人数再按序号取成员
        let count = unsafe { (self.member_count)(mm, lobby) }.clamp(0, MAX_MEMBER_SCAN);
        (0..count).map(|i| unsafe { (self.member_at)(mm, lobby, i) }).filter(|&id| id != 0).collect()
    }

    /// 键没设置或不在大厅时为 None
    pub fn member_data(&self, lobby: u64, user: u64, key: &str) -> Option<String> {
        let key = CString::new(key).ok()?;
        text(unsafe { (self.get_member_data)(iface(self.matchmaking)?, lobby, user, key.as_ptr()) })
    }

    /// 只能写自己的数据；Steam 会合并短时间内的多次写入
    pub fn set_member_data(&self, lobby: u64, key: &str, value: &str) {
        let (Some(mm), Ok(key), Ok(value)) = (iface(self.matchmaking), CString::new(key), CString::new(value)) else { return };
        unsafe { (self.put_member_data)(mm, lobby, key.as_ptr(), value.as_ptr()) };
    }

    pub fn persona_name(&self, user: u64) -> Option<String> {
        text(unsafe { (self.get_persona)(iface(self.friends)?, user) }).filter(|n| n != "[unknown]")
    }

    /// 释放本线程上 Steam 返回字符串占用的内存（返回的字符串都已复制）
    pub fn release_thread_memory(&self) {
        unsafe { (self.release_memory)() };
    }
}

#[cfg(test)]
mod tests {
    use std::mem::offset_of;

    use super::*;

    fn enter(lobby: u64, response: u32) -> LobbyEnter {
        LobbyEnter { lobby, permissions: 0, locked: 0, response }
    }

    #[test]
    fn lobby_enter_matches_steam_layout() {
        assert_eq!(size_of::<LobbyEnter>(), 24);
        assert_eq!(offset_of!(LobbyEnter, locked), 12);
        assert_eq!(offset_of!(LobbyEnter, response), 16);
    }

    #[test]
    fn callback_base_matches_steam_layout() {
        assert_eq!(size_of::<CallbackBase>(), 16);
        assert_eq!(offset_of!(CallbackBase, flags), 8);
        assert_eq!(offset_of!(CallbackBase, id), 12);
    }

    #[test]
    fn queue_keeps_only_successful_entries_in_order() {
        let q = EnteredQueue::new();
        q.record(&enter(1, ENTER_SUCCESS));
        q.record(&enter(2, 5));
        q.record(&enter(3, ENTER_SUCCESS));
        assert_eq!(q.drain(), vec![1, 3]);
        assert!(q.drain().is_empty());
    }

    #[test]
    fn queue_is_bounded_and_keeps_newest() {
        let q = EnteredQueue::new();
        for i in 0..(QUEUE_CAP as u64 + 10) {
            q.record(&enter(i, ENTER_SUCCESS));
        }
        let got = q.drain();
        assert_eq!(got.len(), QUEUE_CAP);
        assert_eq!(got.last(), Some(&(QUEUE_CAP as u64 + 9)));
    }
}
