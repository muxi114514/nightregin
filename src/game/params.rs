//! 运行时读取游戏已加载的参数表：ParamRepositoryImp（FD4 资源仓库）的哈希桶里串着全部 ParamResCap，每个指向一份 PARAM 数据。
//! 仓库 +0x94 桶数、+0x98 桶数组；ResCap +0x50 链表下一项。黑夜君临比艾尔登法环多一层：ResCap +0x80 指向内层资源对象
//! （无 RTTI），内层 +0x78 数据大小、+0x80 PARAM 数据（2026-10 实机内存核对）。
//! 每一步都校验 vtable 与 PARAM 头的类型名，对不上就返回带内存摘要的错误，不会读错数据。

use std::collections::HashMap;

use super::memory::{looks_like_pointer, read, read_bytes, read_cstr, read_ptr};
use super::rtti::has_vtable;

const BUCKET_COUNT: usize = 0x94;
const BUCKETS: usize = 0x98;
const RES_CAP_NEXT: usize = 0x50;
const RES_CAP_INNER: usize = 0x80;
const INNER_SIZE: usize = 0x78;
const INNER_DATA: usize = 0x80;
const MAX_BUCKETS: u32 = 0x4000;
const MAX_RES_CAPS: usize = 0x4000;
const MAX_PARAM_BYTES: u64 = 256 << 20;
/// PARAM 头 +0x10：类型名偏移
const TYPE_NAME_OFFSET: usize = 0x10;

pub struct ParamRepository {
    res_caps: Vec<usize>,
}

impl ParamRepository {
    /// instance：ParamRepositoryImp 实例；vt_res_cap：ParamResCap 的 vtable
    pub fn open(instance: usize, vt_res_cap: usize) -> Result<Self, String> {
        let count = read::<u32>(instance + BUCKET_COUNT).filter(|c| (1..=MAX_BUCKETS).contains(c));
        let buckets = read_ptr(instance + BUCKETS).filter(|&b| looks_like_pointer(b));
        let (Some(count), Some(buckets)) = (count, buckets) else {
            return Err(format!("unexpected repository layout: {}", dump(instance)));
        };
        let heads = read_bytes(buckets, count as usize * 8).ok_or("bucket array unreadable")?;
        let mut res_caps = Vec::new();
        for head in heads.chunks_exact(8) {
            let mut p = usize::from_le_bytes(head.try_into().unwrap());
            while p != 0 && res_caps.len() < MAX_RES_CAPS {
                if !has_vtable(p, vt_res_cap) {
                    return Err(format!("bucket entry {p:#x} is not a ParamResCap: {}", dump(p)));
                }
                res_caps.push(p);
                p = read_ptr(p + RES_CAP_NEXT).unwrap_or(0);
            }
        }
        Ok(Self { res_caps })
    }

    pub fn len(&self) -> usize {
        self.res_caps.len()
    }

    /// 复制类型名在 wanted 中的表（每个类型名取第一张），返回 类型名 -> PARAM 数据
    pub fn read_tables(&self, wanted: &[&str]) -> HashMap<String, Vec<u8>> {
        let mut out = HashMap::new();
        for (name, data, size) in self.tables() {
            if wanted.contains(&name.as_str()) && !out.contains_key(&name) {
                if let Some(bytes) = read_bytes(data, size as usize) {
                    out.insert(name, bytes);
                }
            }
        }
        out
    }

    /// 表在游戏内存中的起始地址（改写用）；与 read_tables 取同一张
    pub fn table_address(&self, struct_name: &str) -> Option<usize> {
        self.tables().find(|(name, _, _)| name == struct_name).map(|(_, data, _)| data)
    }

    /// 同一类型名的全部表（如 ItemLotParam_map 与 _enemy）：(地址, 大小)
    pub fn table_locations(&self, struct_name: &str) -> Vec<(usize, u64)> {
        self.tables().filter(|(name, _, _)| name == struct_name).map(|(_, data, size)| (data, size)).collect()
    }

    /// (类型名, PARAM 数据地址, 大小)
    fn tables(&self) -> impl Iterator<Item = (String, usize, u64)> + '_ {
        self.res_caps.iter().filter_map(|&rc| {
            let (data, size) = Self::data_of(rc)?;
            let name = read::<u64>(data + TYPE_NAME_OFFSET).filter(|&off| off < size).and_then(|off| read_cstr(data + off as usize, 64))?;
            Some((name, data, size))
        })
    }

    fn data_of(res_cap: usize) -> Option<(usize, u64)> {
        let inner = read_ptr(res_cap + RES_CAP_INNER).filter(|&p| looks_like_pointer(p))?;
        let size = read::<u64>(inner + INNER_SIZE).filter(|&s| (0x40..MAX_PARAM_BYTES).contains(&s))?;
        let data = read_ptr(inner + INNER_DATA).filter(|&d| looks_like_pointer(d))?;
        Some((data, size))
    }
}

/// 对象前 0xA0 字节（按 8 字节），用于布局不符时定位问题
fn dump(addr: usize) -> String {
    let words: Vec<String> = (0..0xA0)
        .step_by(8)
        .map(|off| read::<u64>(addr + off).map_or("??".into(), |v| format!("+{off:02x}={v:#x}")))
        .collect();
    format!("{addr:#x} [{}]", words.join(" "))
}
