//! 在主模块 .data 节中查找指向某类实例的静态指针（FD4 单例都存放在这样的静态变量里），用于没有固定 RVA 的单例。
//! 整节扫描较慢（数十万次读取），只在启动阶段的初始化线程里调用，找到后由调用方缓存。

use super::memory::{looks_like_pointer, read_bytes, read_ptr};
use super::module::GameModule;

const CHUNK: usize = 0x10000;

/// 返回 (静态变量地址, 实例地址)
pub fn find_instance(module: &GameModule, vtable: usize) -> Option<(usize, usize)> {
    let data = module.section(".data")?;
    let mut cursor = data.start;
    while cursor < data.end {
        let len = CHUNK.min(data.end - cursor);
        if let Some(bytes) = read_bytes(cursor, len) {
            for (i, q) in bytes.chunks_exact(8).enumerate() {
                let p = usize::from_le_bytes(q.try_into().unwrap());
                if looks_like_pointer(p) && !module.contains(p) && read_ptr(p) == Some(vtable) {
                    return Some((cursor + i * 8, p));
                }
            }
        }
        cursor += len;
    }
    None
}
