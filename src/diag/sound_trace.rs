//! 音效诊断（配置 [diag] sound_trace = true 时启用）：钩住 Wwise 的 PostEvent 与 GetIDFromString，
//! 把游戏播放的每个事件（ID、游戏对象、调用位置）和按下的导航键写进日志，用来找出菜单光标 / 确认音效。
//! F11：在本模组的游戏对象上重放"最近一次按键后游戏播放的第一个事件"，验证自己播放能出声。
//! 钩子跑在游戏的各个线程，只往队列里放记录；格式化与写日志在每帧任务（游戏主线程）里做。

use std::collections::{HashMap, HashSet};
use std::ffi::{CStr, c_char, c_void};
use std::fmt::Write;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use log::{info, warn};
use windows_sys::Win32::System::Diagnostics::Debug::RtlCaptureStackBackTrace;

use crate::game::module::GameModule;
use crate::game::sound::{self, GetIdFn, PostEventFn};
use crate::hook;
use crate::hotkey::HotkeyState;

const QUEUE_CAP: usize = 8192;
const NAME_CAP: usize = 65536;
/// 按键后这段时间内游戏播放的事件标记为"按键引起"
const AFTER_KEY_MS: u64 = 400;
const CALLERS: usize = 3;
const VK_F11: u32 = 0x7A;
const KEYS: [(u32, &str); 19] = [
    (0x26, "Up"),
    (0x28, "Down"),
    (0x25, "Left"),
    (0x27, "Right"),
    (0x57, "W"),
    (0x53, "S"),
    (0x41, "A"),
    (0x44, "D"),
    (0x45, "E"),
    (0x51, "Q"),
    (0x46, "F"),
    (0x52, "R"),
    (0x0D, "Enter"),
    (0x1B, "Esc"),
    (0x20, "Space"),
    (0x09, "Tab"),
    (0x08, "Backspace"),
    (0x01, "MouseL"),
    (0x02, "MouseR"),
];

static INSTALLED: AtomicBool = AtomicBool::new(false);
static POST_TRAMPOLINE: AtomicUsize = AtomicUsize::new(0);
static ID_TRAMPOLINE: AtomicUsize = AtomicUsize::new(0);
static QUEUE: Mutex<Vec<Record>> = Mutex::new(Vec::new());
static DROPPED: AtomicU32 = AtomicU32::new(0);
static SEEN_NAMES: OnceLock<Mutex<HashSet<u32>>> = OnceLock::new();

enum Record {
    Post { t: u64, event: u32, object: u64, playing: u32, callers: [usize; CALLERS] },
    Name { id: u32, name: String },
}

/// 需在 hudhook 初始化 MinHook 之后调用
pub fn install() {
    let (Some(post), Some(get_id)) = (sound::export(sound::POST_EVENT), sound::export(sound::GET_ID_FROM_STRING)) else {
        warn!("sound trace: Wwise exports not found");
        return;
    };
    let hooks: Result<Vec<_>, String> = [
        hook::create(post, post_event as *mut c_void, &POST_TRAMPOLINE),
        hook::create(get_id, get_id_from_string as *mut c_void, &ID_TRAMPOLINE),
    ]
    .into_iter()
    .collect();
    match hooks {
        Ok(h) if hook::enable(&h) => {
            INSTALLED.store(true, Ordering::Release);
            info!("sound trace hooks installed (PostEvent {post:#x}, GetIDFromString {get_id:#x}); F11 replays the last key-triggered event");
        }
        Ok(_) => warn!("sound trace: enabling hooks failed"),
        Err(e) => warn!("sound trace unavailable: {e}"),
    }
}

fn now_ms() -> u64 {
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_millis() as u64
}

fn push(record: Record) {
    match QUEUE.lock() {
        Ok(mut q) if q.len() < QUEUE_CAP => q.push(record),
        _ => {
            DROPPED.fetch_add(1, Ordering::Relaxed);
        }
    }
}

/// 调用栈上的返回地址（跳过本函数与钩子函数自身，第一个就是游戏里调用 PostEvent 的位置）
#[inline(never)]
fn callers() -> [usize; CALLERS] {
    let mut frames = [std::ptr::null_mut::<c_void>(); CALLERS];
    let n = unsafe { RtlCaptureStackBackTrace(2, CALLERS as u32, frames.as_mut_ptr(), std::ptr::null_mut()) } as usize;
    let mut out = [0; CALLERS];
    for (o, f) in out.iter_mut().zip(&frames[..n]) {
        *o = *f as usize;
    }
    out
}

#[allow(clippy::too_many_arguments)]
unsafe extern "system" fn post_event(
    event: u32,
    object: u64,
    flags: u32,
    callback: *const c_void,
    cookie: *mut c_void,
    externals: u32,
    sources: *const c_void,
    playing: u32,
) -> u32 {
    let orig: PostEventFn = unsafe { std::mem::transmute(POST_TRAMPOLINE.load(Ordering::Acquire)) };
    let id = unsafe { orig(event, object, flags, callback, cookie, externals, sources, playing) };
    push(Record::Post { t: now_ms(), event, object, playing: id, callers: callers() });
    id
}

unsafe extern "system" fn get_id_from_string(name: *const c_char) -> u32 {
    let orig: GetIdFn = unsafe { std::mem::transmute(ID_TRAMPOLINE.load(Ordering::Acquire)) };
    let id = unsafe { orig(name) };
    if !name.is_null() && first_seen(id) {
        let name = unsafe { CStr::from_ptr(name) }.to_string_lossy().into_owned();
        push(Record::Name { id, name });
    }
    id
}

fn first_seen(id: u32) -> bool {
    let seen = SEEN_NAMES.get_or_init(|| Mutex::new(HashSet::new()));
    seen.lock().is_ok_and(|mut s| s.len() < NAME_CAP && s.insert(id))
}

/// 每帧任务持有的诊断状态
pub struct SoundTrace {
    keys: HotkeyState,
    module: (usize, usize),
    names: HashMap<u32, String>,
    last_key: Option<u64>,
    /// 按键后还没遇到游戏播放的事件
    armed: bool,
    replay: Option<u32>,
}

impl SoundTrace {
    pub fn new(module: &GameModule) -> Self {
        let mut keys: Vec<u32> = KEYS.iter().map(|k| k.0).collect();
        keys.push(VK_F11);
        Self {
            keys: HotkeyState::new(&keys),
            module: (module.base, module.size),
            names: HashMap::new(),
            last_key: None,
            armed: false,
            replay: None,
        }
    }

    /// 每帧调用：记录按键、把钩子收集的记录合并成一条日志写出
    pub fn flush(&mut self) {
        if !INSTALLED.load(Ordering::Acquire) {
            return;
        }
        let mut out = String::new();
        let now = now_ms();
        for vk in self.keys.poll() {
            if vk == VK_F11 {
                self.replay(&mut out);
                continue;
            }
            let name = KEYS.iter().find(|k| k.0 == vk).map_or("?", |k| k.1);
            let _ = write!(out, "\n  key {name} t={now}");
            self.last_key = Some(now);
            self.armed = true;
        }
        let records = QUEUE.lock().map(|mut q| std::mem::take(&mut *q)).unwrap_or_default();
        for record in records {
            match record {
                Record::Name { id, name } => {
                    let _ = write!(out, "\n  name {id:#010x} {name}");
                    if self.names.len() < NAME_CAP {
                        self.names.insert(id, name);
                    }
                }
                Record::Post { t, event, object, playing, callers } => self.post_line(&mut out, t, event, object, playing, &callers),
            }
        }
        let dropped = DROPPED.swap(0, Ordering::Relaxed);
        if dropped > 0 {
            let _ = write!(out, "\n  {dropped} records dropped (queue full)");
        }
        if !out.is_empty() {
            info!("snd:{out}");
        }
    }

    fn post_line(&mut self, out: &mut String, t: u64, event: u32, object: u64, playing: u32, callers: &[usize]) {
        let after_key = self.last_key.is_some_and(|k| t + 50 >= k && t <= k + AFTER_KEY_MS);
        if after_key && self.armed && object != sound::OWN_OBJECT {
            self.armed = false;
            self.replay = Some(event);
        }
        let _ = write!(out, "\n  post t={t} ev={event:#010x} obj={object:#x} pid={playing} at");
        for &c in callers.iter().filter(|&&c| c != 0) {
            let (base, size) = self.module;
            match c.checked_sub(base).filter(|&r| r < size) {
                Some(rva) => _ = write!(out, " +{rva:#x}"),
                None => _ = write!(out, " {c:#x}"),
            }
        }
        if let Some(name) = self.names.get(&event) {
            let _ = write!(out, " {name}");
        }
        if after_key {
            out.push_str(" <key");
        }
    }

    fn replay(&self, out: &mut String) {
        match self.replay {
            Some(event) => {
                let pid = sound::post(event);
                let _ = write!(out, "\n  F11 replay ev={event:#010x} on own object -> pid={pid}");
            }
            None => out.push_str("\n  F11: no key-triggered event yet"),
        }
    }
}
