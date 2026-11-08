//! 把 Rust 闭包注册为游戏的 FD4 每帧任务，保证业务逻辑在游戏主线程执行（避免与游戏线程竞争 SpEffect 链表）。
//! 任务对象布局与 ER 相同：vtable + 8 字节字段，vtable 为 [get_runtime_class, destructor, execute]。

use std::ffi::c_void;
use std::time::{Duration, Instant};

use log::{info, warn};

use super::addresses::Addresses;
use super::memory::{looks_like_pointer, read_ptr, read_wstr};
use super::rtti::has_vtable;

type TaskFn = Box<dyn FnMut() + Send + 'static>;

#[repr(C)]
struct TaskVmt {
    get_runtime_class: extern "C" fn(*const NativeTask) -> *const c_void,
    destructor: extern "C" fn(*mut NativeTask),
    execute: extern "C" fn(*mut NativeTask, *const c_void),
}

/// 交给游戏的任务对象。前两个字段由游戏读写，之后预留空间再放闭包。
#[repr(C)]
struct NativeTask {
    vftable: *const TaskVmt,
    unk8: usize,
    _reserved: [usize; 6],
    closure: TaskFn,
}

/// 游戏若查询运行时类信息，返回一块全零的占位数据（ER 上实测不会调用）
static DUMMY_RUNTIME_CLASS: [usize; 16] = [0; 16];

extern "C" fn get_runtime_class(_: *const NativeTask) -> *const c_void {
    DUMMY_RUNTIME_CLASS.as_ptr() as *const c_void
}

/// 任务与进程同寿命，游戏不应析构它
extern "C" fn destructor(_: *mut NativeTask) {}

extern "C" fn execute(task: *mut NativeTask, _data: *const c_void) {
    // 闭包内部已自行捕获 panic，这里不会有异常越过 FFI 边界
    unsafe { ((*task).closure)() }
}

static VMT: TaskVmt = TaskVmt { get_runtime_class, destructor, execute };

const WAIT_LOG_INTERVAL: Duration = Duration::from_secs(30);

/// 轮询等待游戏创建 CSTaskGroupImp（me3 在游戏初始化前就加载本 DLL），每 30 秒记录一次等待状态
pub fn wait_for_task_groups(addrs: &Addresses, timeout: Duration) -> Option<usize> {
    let start = Instant::now();
    let mut last_log = Instant::now();
    while start.elapsed() < timeout {
        let ptr = read_ptr(addrs.task_group_static).unwrap_or(0);
        if ptr != 0 && has_vtable(ptr, addrs.vt_task_group_imp) {
            return Some(ptr);
        }
        if last_log.elapsed() >= WAIT_LOG_INTERVAL {
            last_log = Instant::now();
            info!("still waiting for CSTaskGroupImp: static = {ptr:#x}, vtable = {:#x}", read_ptr(ptr).unwrap_or(0));
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    None
}

/// 注册每帧执行的闭包；任务对象被有意泄漏，与进程同寿命。
/// register_task 一开始就覆盖了第一个参数，实际使用的是它自己读取的 CSTaskGroupImp 静态指针。
pub fn register_recurring(addrs: &Addresses, task_groups: usize, group: u32, closure: TaskFn) {
    let task = Box::leak(Box::new(NativeTask { vftable: &VMT, unk8: 0, _reserved: [0; 6], closure }));
    let register: extern "C" fn(usize, u32, *mut NativeTask) = unsafe { std::mem::transmute(addrs.register_task) };
    register(task_groups, group, task);
}

const DEFAULT_FRAME_BEGIN: u32 = 0;
const MAX_GROUPS: usize = 256;
/// CSTimeLineTaskGroupIns -> 组名（UTF-16）指针
const GROUP_NAME_OFFSET: usize = 0x18;

/// 按名字查找 "FrameBegin" 任务组序号（实测为 0）；读不通时回退为 0 并告警
pub fn frame_begin_index(task_groups: usize) -> u32 {
    let groups = list_task_groups(task_groups);
    if groups.is_empty() {
        warn!("task group list unreadable, using FrameBegin = {DEFAULT_FRAME_BEGIN}");
        return DEFAULT_FRAME_BEGIN;
    }
    info!("task groups: {} total, first: {:?}", groups.len(), groups.iter().take(8).collect::<Vec<_>>());
    match groups.iter().find(|(_, name)| name == "FrameBegin") {
        Some((index, _)) => *index,
        None => {
            warn!("FrameBegin not found by name, using {DEFAULT_FRAME_BEGIN}");
            DEFAULT_FRAME_BEGIN
        }
    }
}

/// CSTaskGroupImp+0x8 起为各任务组实例的指针数组，下标即组序号；遇到类型不同的指针即结束
fn list_task_groups(task_groups: usize) -> Vec<(u32, String)> {
    let Some(group_vtable) = read_ptr(task_groups + 8).and_then(read_ptr) else { return Vec::new() };
    let mut out = Vec::new();
    for i in 0..MAX_GROUPS {
        let Some(group) = read_ptr(task_groups + 8 + i * 8) else { break };
        if !looks_like_pointer(group) || read_ptr(group) != Some(group_vtable) {
            break;
        }
        let name = read_ptr(group + GROUP_NAME_OFFSET).and_then(|p| read_wstr(p, 64)).unwrap_or_default();
        out.push((i as u32, name));
    }
    out
}
