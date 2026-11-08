//! 诊断（cargo 特性 tiered-diag）：硬件数据断点。给本进程所有线程的调试寄存器设「读写 4 字节」断点，
//! 向量异常处理只记下触发的指令地址与次数、不改变执行，用来找引擎读某个内存字段（如 PlayerCommonParam 里的第六感效果号）的代码。

use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

use windows_sys::Win32::Foundation::{CloseHandle, EXCEPTION_SINGLE_STEP, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::Diagnostics::Debug::{
    AddVectoredExceptionHandler, CONTEXT, CONTEXT_DEBUG_REGISTERS_AMD64, EXCEPTION_POINTERS, GetThreadContext, SetThreadContext,
};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{CreateToolhelp32Snapshot, TH32CS_SNAPTHREAD, THREADENTRY32, Thread32First, Thread32Next};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::{
    GetCurrentProcessId, GetCurrentThreadId, OpenThread, ResumeThread, SuspendThread, THREAD_GET_CONTEXT, THREAD_SET_CONTEXT, THREAD_SUSPEND_RESUME,
};

const EXCEPTION_CONTINUE_EXECUTION: i32 = -1;
const EXCEPTION_CONTINUE_SEARCH: i32 = 0;
const SLOTS: usize = 64;

/// 触发过的指令地址（断点是陷阱型，记下的是访问指令的下一条）、次数、哪个断点（DR6 低 4 位）
static RIPS: [AtomicUsize; SLOTS] = [const { AtomicUsize::new(0) }; SLOTS];
static HITS: [AtomicU32; SLOTS] = [const { AtomicU32::new(0) }; SLOTS];
static WHICH: [AtomicU32; SLOTS] = [const { AtomicU32::new(0) }; SLOTS];
static HANDLER: AtomicUsize = AtomicUsize::new(0);

/// GetThreadContext 要求 16 字节对齐
#[repr(C, align(16))]
struct AlignedContext(CONTEXT);

/// 给本进程除调用线程外的所有线程装上断点（最多 4 个地址），返回成功的线程数。
/// 不能改自己正在跑的线程，所以要在单独的线程里调用；之后新建的线程没有断点，可再次调用补上
pub fn install(addrs: &[usize]) -> usize {
    if HANDLER.load(Ordering::Acquire) == 0 {
        let h = unsafe { AddVectoredExceptionHandler(1, Some(handler)) };
        HANDLER.store(h as usize, Ordering::Release);
    }
    let (me, pid) = unsafe { (GetCurrentThreadId(), GetCurrentProcessId()) };
    let snap = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) };
    if snap == INVALID_HANDLE_VALUE {
        return 0;
    }
    let mut entry: THREADENTRY32 = unsafe { std::mem::zeroed() };
    entry.dwSize = size_of::<THREADENTRY32>() as u32;
    let mut n = 0;
    let mut more = unsafe { Thread32First(snap, &mut entry) } != 0;
    while more {
        if entry.th32OwnerProcessID == pid && entry.th32ThreadID != me && set_thread(entry.th32ThreadID, addrs) {
            n += 1;
        }
        more = unsafe { Thread32Next(snap, &mut entry) } != 0;
    }
    unsafe { CloseHandle(snap) };
    n
}

/// (相对 exe 的 RVA, 次数, 哪个断点)
pub fn hits() -> Vec<(usize, u32, u32)> {
    let base = unsafe { GetModuleHandleW(std::ptr::null()) } as usize;
    (0..SLOTS)
        .filter_map(|i| {
            let rip = RIPS[i].load(Ordering::Acquire);
            (rip != 0).then(|| (rip.wrapping_sub(base), HITS[i].load(Ordering::Relaxed), WHICH[i].load(Ordering::Relaxed)))
        })
        .collect()
}

fn set_thread(tid: u32, addrs: &[usize]) -> bool {
    let h = unsafe { OpenThread(THREAD_GET_CONTEXT | THREAD_SET_CONTEXT | THREAD_SUSPEND_RESUME, 0, tid) };
    if h == 0 {
        return false;
    }
    let mut ctx: AlignedContext = unsafe { std::mem::zeroed() };
    ctx.0.ContextFlags = CONTEXT_DEBUG_REGISTERS_AMD64;
    let ok = unsafe {
        if SuspendThread(h) == u32::MAX {
            false
        } else {
            let set = GetThreadContext(h, &mut ctx.0) != 0 && {
                set_registers(&mut ctx.0, addrs);
                SetThreadContext(h, &ctx.0) != 0
            };
            ResumeThread(h);
            set
        }
    };
    unsafe { CloseHandle(h) };
    ok
}

/// DR0~DR3 填地址，DR7 对应位：本地启用、RW = 11（读写）、LEN = 11（4 字节）
fn set_registers(ctx: &mut CONTEXT, addrs: &[usize]) {
    let mut dr7 = ctx.Dr7;
    for (i, (reg, &addr)) in [&mut ctx.Dr0, &mut ctx.Dr1, &mut ctx.Dr2, &mut ctx.Dr3].into_iter().zip(addrs).enumerate() {
        *reg = addr as u64;
        dr7 &= !(0b11 << (i * 2)) & !(0b1111 << (16 + i * 4));
        dr7 |= (1 << (i * 2)) | (0b1111 << (16 + i * 4));
    }
    ctx.Dr7 = dr7;
}

unsafe extern "system" fn handler(info: *mut EXCEPTION_POINTERS) -> i32 {
    let (record, ctx) = unsafe { (&*(*info).ExceptionRecord, &mut *(*info).ContextRecord) };
    let which = (ctx.Dr6 & 0xF) as u32;
    if record.ExceptionCode != EXCEPTION_SINGLE_STEP || which == 0 {
        return EXCEPTION_CONTINUE_SEARCH;
    }
    record_hit(ctx.Rip as usize, which);
    ctx.Dr6 = 0;
    EXCEPTION_CONTINUE_EXECUTION
}

/// 异常处理里不分配、不加锁：固定槽位按地址去重计数，满了就丢弃
fn record_hit(rip: usize, which: u32) {
    for i in 0..SLOTS {
        let cur = RIPS[i].load(Ordering::Acquire);
        let mine = cur == rip || (cur == 0 && RIPS[i].compare_exchange(0, rip, Ordering::AcqRel, Ordering::Acquire).map_or_else(|v| v == rip, |_| true));
        if mine {
            WHICH[i].fetch_or(which, Ordering::Relaxed);
            HITS[i].fetch_add(1, Ordering::Relaxed);
            return;
        }
    }
}
