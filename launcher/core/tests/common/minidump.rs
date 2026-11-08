//! 合成的 minidump：文件头、三项流目录（线程列表占位、模块列表、异常流）、模块名。
//! 默认两个模块 nightreign.exe、ntdll.dll；ntdll 的模块名排在文件最后，截掉末尾任何一段都读不全它

use std::fs::{self, File};
use std::path::Path;
use std::time::SystemTime;

pub const AV: u32 = 0xC000_0005;
/// 访问违例的访问类型
pub const READ: u64 = 0;
pub const WRITE: u64 = 1;
pub const EXECUTE: u64 = 8;
pub const GAME_BASE: u64 = 0x7FF6_1000_0000;
pub const NTDLL_BASE: u64 = 0x7FFA_2000_0000;
/// 实测退出时崩溃的被读地址
pub const EXIT_READ_AT: u64 = 0xB0_1FFF;

const GAME_PATH: &str =
    r"D:\Steam Library\steamapps\common\ELDEN RING NIGHTREIGN\Game\nightreign.exe";
const NTDLL_PATH: &str = r"C:\WINDOWS\SYSTEM32\ntdll.dll";
/// (路径, 基址, 大小)
pub type Module<'a> = (&'a str, u64, u32);
pub const MODULES: [Module<'static>; 2] = [
    (GAME_PATH, GAME_BASE, 0x0600_0000),
    (NTDLL_PATH, NTDLL_BASE, 0x0021_8000),
];

const HEADER_LEN: usize = 32;
const EXCEPTION_LEN: usize = 168;
const MODULE_LEN: usize = 108;

/// 一次异常：异常码、ExceptionInformation[0]（访问类型）、出错地址
#[derive(Clone, Copy, Debug)]
pub struct Fault {
    pub code: u32,
    pub access: u64,
    pub address: u64,
}

/// 实测的退出时崩溃：ntdll+0x1cc5d 读访问违例
pub fn exit_crash() -> Vec<u8> {
    minidump(Fault {
        code: AV,
        access: READ,
        address: NTDLL_BASE + 0x1cc5d,
    })
}

/// 游戏自己的读访问违例（实测的真崩溃之一）
pub fn game_read_av() -> Vec<u8> {
    minidump(Fault {
        code: AV,
        access: READ,
        address: GAME_BASE + 0x1dbdefa,
    })
}

/// 跳到野地址执行：不在任何模块里
pub fn exec_av_no_module() -> Vec<u8> {
    minidump(Fault {
        code: AV,
        access: EXECUTE,
        address: 0x0000_0123_4567_8000,
    })
}

/// ntdll 里的堆损坏（0xC0000374），不是访问违例
pub fn ntdll_heap_corruption() -> Vec<u8> {
    minidump(Fault {
        code: 0xC000_0374,
        access: READ,
        address: NTDLL_BASE + 0x1cc5d,
    })
}

pub fn minidump(f: Fault) -> Vec<u8> {
    minidump_in(f, &MODULES)
}

/// modules 依次写进模块列表，模块名按同样顺序排在文件末尾
pub fn minidump_in(f: Fault, modules: &[Module]) -> Vec<u8> {
    let threads = HEADER_LEN + 3 * 12;
    let exception = threads + 4;
    let list = exception + EXCEPTION_LEN;
    let list_len = 4 + modules.len() * MODULE_LEN;
    let mut d = Vec::new();
    d.extend_from_slice(b"MDMP");
    for v in [0xA793, 3, HEADER_LEN as u32, 0, 0, 0, 0] {
        put32(&mut d, v);
    }
    // 异常流不排第一：按类型找流
    for (kind, size, rva) in [
        (3, 4, threads),
        (4, list_len, list),
        (6, EXCEPTION_LEN, exception),
    ] {
        for v in [kind, size as u32, rva as u32] {
            put32(&mut d, v);
        }
    }
    put32(&mut d, 0);
    assert_eq!(d.len(), exception);
    // ThreadId、对齐、code、flags、record、address、参数个数、对齐、info[15]、线程上下文位置
    for v in [1234, 0, f.code, 0] {
        put32(&mut d, v);
    }
    put64(&mut d, 0);
    put64(&mut d, f.address);
    put32(&mut d, 2);
    put32(&mut d, 0);
    put64(&mut d, f.access);
    put64(&mut d, EXIT_READ_AT);
    d.resize(list, 0);
    put32(&mut d, modules.len() as u32);
    let mut name_rva = list + list_len;
    for (path, base, size) in modules {
        let start = d.len();
        put64(&mut d, *base);
        for v in [*size, 0, 0, name_rva as u32] {
            put32(&mut d, v);
        }
        d.resize(start + MODULE_LEN, 0);
        name_rva += 4 + path.encode_utf16().count() * 2;
    }
    for (path, _, _) in modules {
        let units: Vec<u16> = path.encode_utf16().collect();
        put32(&mut d, (units.len() * 2) as u32);
        units
            .iter()
            .for_each(|u| d.extend_from_slice(&u.to_le_bytes()));
    }
    assert_eq!(d.len(), name_rva);
    d
}

/// 流目录第 2 项（模块列表）的类型改成 0（未用的流）：转储里没有模块列表
pub fn without_module_list(mut d: Vec<u8>) -> Vec<u8> {
    put32_at(&mut d, HEADER_LEN + 12, 0);
    d
}

/// 异常的参数个数（NumberParameters）改成 n
pub fn with_params(mut d: Vec<u8>, n: u32) -> Vec<u8> {
    put32_at(&mut d, HEADER_LEN + 3 * 12 + 4 + 32, n);
    d
}

/// dir 里写一个转储，修改时间为 modified
pub fn write_dump(dir: &Path, name: &str, bytes: &[u8], modified: SystemTime) {
    let path = dir.join(name);
    fs::write(&path, bytes).unwrap();
    File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_modified(modified)
        .unwrap();
}

fn put32(d: &mut Vec<u8>, v: u32) {
    d.extend_from_slice(&v.to_le_bytes());
}

fn put32_at(d: &mut [u8], at: usize, v: u32) {
    d[at..at + 4].copy_from_slice(&v.to_le_bytes());
}

fn put64(d: &mut Vec<u8>, v: u64) {
    d.extend_from_slice(&v.to_le_bytes());
}
