//! 崩溃转储分类：只读 minidump 的异常流和模块列表，分出「退出时的已知崩溃」和真崩溃。
//! 实测从游戏菜单正常退出时，游戏会在 ntdll 里读访问违例（0xC0000005）并留下一个转储。
//! 判定只看异常码、出错模块的文件名和访问类型，不认偏移（系统更新会变）；读不出来的一律算真崩溃

use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;
use std::time::SystemTime;

use crate::os::crash_dumps_since;
use crate::retry;

/// 访问违例
pub const ACCESS_VIOLATION: u32 = 0xC000_0005;
/// 退出时的已知崩溃所在的模块
const EXIT_CRASH_MODULE: &str = "ntdll.dll";
/// 访问违例的 ExceptionInformation[0]：0 读、1 写、8 执行
const ACCESS_READ: u64 = 0;
const ACCESS_WRITE: u64 = 1;
const ACCESS_EXECUTE: u64 = 8;

const SIGNATURE: &[u8] = b"MDMP";
/// 文件头读到 StreamDirectoryRva 为止
const HEADER_LEN: usize = 16;
/// 流目录项：type、size、rva
const DIR_ENTRY_LEN: usize = 12;
const MODULE_LIST_STREAM: u32 = 4;
const EXCEPTION_STREAM: u32 = 6;
/// 异常流读到 ExceptionInformation[1] 为止：ThreadId、对齐、code、flags、record、address、参数个数、对齐、info[0..2]
const EXCEPTION_LEN: usize = 56;
const MODULE_LEN: usize = 108;
/// 真实转储只有十几个流、几百个模块；垃圾数据里的大数不能拿去分配内存
const MAX_STREAMS: u32 = 1024;
const MAX_MODULES: u32 = 8192;
/// 模块名的字节数上限（32767 个 UTF-16 单元）
const MAX_NAME_BYTES: u32 = 65_534;

/// 出错地址所在的模块
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FaultModule {
    /// 文件名，不含目录
    pub name: String,
    pub base: u64,
    pub size: u32,
}

/// 转储里的异常信息
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DumpInfo {
    pub code: u32,
    pub flags: u32,
    /// 出错的指令地址
    pub address: u64,
    /// ExceptionInformation[0]，访问违例时是访问类型；参数不足时为 None
    pub access_kind: Option<u64>,
    /// ExceptionInformation[1]，访问违例时是被访问的地址
    pub access_address: Option<u64>,
    /// address 所在的模块；不在任何模块里、或转储没有模块列表时为 None
    pub module: Option<FaultModule>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DumpKind {
    /// 退出时的已知崩溃：ntdll 里的读访问违例
    ExitCrash,
    RealCrash,
}

#[derive(Debug, thiserror::Error)]
pub enum DumpError {
    #[error("{0}")]
    Io(#[from] io::Error),
    #[error("文件不完整")]
    Truncated,
    #[error("不是 minidump")]
    NotMinidump,
    #[error("没有异常流")]
    NoException,
    #[error("{0}不合理")]
    Malformed(&'static str),
}

impl DumpInfo {
    /// 读出 path 的异常信息：只按偏移读用得到的几段，不读整个文件
    pub fn read(path: &Path) -> Result<DumpInfo, DumpError> {
        let mut file = retry::transient(|| File::open(path))?;
        DumpInfo::parse(&mut file)
    }

    /// 每一处偏移和长度都先核对，截断或垃圾数据交出 Err，不会 panic
    pub fn parse<R: Read + Seek>(r: &mut R) -> Result<DumpInfo, DumpError> {
        let header = read_at(r, 0, HEADER_LEN)?;
        if header.get(..SIGNATURE.len()) != Some(SIGNATURE) {
            return Err(DumpError::NotMinidump);
        }
        let streams = u32_at(&header, 8)?;
        if streams > MAX_STREAMS {
            return Err(DumpError::Malformed("流的个数"));
        }
        let dir = read_at(
            r,
            u64::from(u32_at(&header, 12)?),
            streams as usize * DIR_ENTRY_LEN,
        )?;
        let (size, rva) = find_stream(&dir, EXCEPTION_STREAM)?.ok_or(DumpError::NoException)?;
        if (size as usize) < EXCEPTION_LEN {
            return Err(DumpError::Malformed("异常流长度"));
        }
        let e = read_at(r, u64::from(rva), EXCEPTION_LEN)?;
        let params = u32_at(&e, 32)?;
        let address = u64_at(&e, 24)?;
        let module = match find_stream(&dir, MODULE_LIST_STREAM)? {
            Some((size, rva)) => find_module(r, size, rva, address)?,
            None => None,
        };
        Ok(DumpInfo {
            code: u32_at(&e, 8)?,
            flags: u32_at(&e, 12)?,
            address,
            access_kind: if params >= 1 {
                Some(u64_at(&e, 40)?)
            } else {
                None
            },
            access_address: if params >= 2 {
                Some(u64_at(&e, 48)?)
            } else {
                None
            },
            module,
        })
    }

    /// 异常码是访问违例、出错模块是 ntdll.dll（不分大小写）、访问类型是读，才算退出时的已知崩溃
    pub fn kind(&self) -> DumpKind {
        let in_ntdll = self
            .module
            .as_ref()
            .is_some_and(|m| m.name.eq_ignore_ascii_case(EXIT_CRASH_MODULE));
        if self.code == ACCESS_VIOLATION && in_ntdll && self.access_kind == Some(ACCESS_READ) {
            DumpKind::ExitCrash
        } else {
            DumpKind::RealCrash
        }
    }

    /// 日志用：异常码（访问违例带访问类型和地址）、模块 + 偏移
    fn describe(&self) -> String {
        let mut text = format!("异常码 {:#x}", self.code);
        if self.code == ACCESS_VIOLATION {
            let kind = match self.access_kind {
                Some(ACCESS_READ) => "读".to_owned(),
                Some(ACCESS_WRITE) => "写".to_owned(),
                Some(ACCESS_EXECUTE) => "执行".to_owned(),
                Some(other) => format!("访问类型 {other}"),
                None => "访问类型未知".to_owned(),
            };
            let at = self
                .access_address
                .map(|a| format!(" {a:#x}"))
                .unwrap_or_default();
            text += &format!("（{kind}{at}）");
        }
        match &self.module {
            Some(m) => {
                text += &format!("，位于 {}+{:#x}", m.name, self.address.wrapping_sub(m.base))
            }
            None => text += &format!("，位于 {:#x}（不在任何模块里）", self.address),
        }
        text
    }
}

/// 给一个转储分类并记一行日志；读不出来的算真崩溃
pub fn classify(path: &Path) -> DumpKind {
    let name = path
        .file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy();
    match DumpInfo::read(path) {
        Ok(info) => {
            let kind = info.kind();
            let verdict = match kind {
                DumpKind::ExitCrash => "退出时的已知崩溃",
                DumpKind::RealCrash => "真崩溃",
            };
            log::info!("崩溃转储 {name}：{}，判为{verdict}", info.describe());
            kind
        }
        Err(e) => {
            log::info!("崩溃转储 {name}：读不出异常信息（{e}），按真崩溃算");
            DumpKind::RealCrash
        }
    }
}

/// 一批转储按分类计数
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DumpSummary {
    pub exit_crashes: usize,
    pub real_crashes: usize,
}

impl DumpSummary {
    /// dumps_dir 里修改时间不早于 since 的转储逐个分类，各记一行日志；没有目录时都是 0
    pub fn since(dumps_dir: Option<&Path>, since: SystemTime) -> DumpSummary {
        let dumps = dumps_dir
            .map(|dir| crash_dumps_since(dir, since))
            .unwrap_or_default();
        let mut summary = DumpSummary::default();
        for path in &dumps {
            match classify(path) {
                DumpKind::ExitCrash => summary.exit_crashes += 1,
                DumpKind::RealCrash => summary.real_crashes += 1,
            }
        }
        summary
    }
}

/// 从 offset 读 len 字节；文件不够长为 Truncated。len 都已按上限核过
fn read_at<R: Read + Seek>(r: &mut R, offset: u64, len: usize) -> Result<Vec<u8>, DumpError> {
    r.seek(SeekFrom::Start(offset))?;
    let mut buf = vec![0; len];
    r.read_exact(&mut buf).map_err(|e| match e.kind() {
        io::ErrorKind::UnexpectedEof => DumpError::Truncated,
        _ => DumpError::Io(e),
    })?;
    Ok(buf)
}

/// 流目录里第一个 kind 类型的流：(size, rva)
fn find_stream(dir: &[u8], kind: u32) -> Result<Option<(u32, u32)>, DumpError> {
    for entry in dir.as_chunks::<DIR_ENTRY_LEN>().0 {
        if u32_at(entry, 0)? == kind {
            return Ok(Some((u32_at(entry, 4)?, u32_at(entry, 8)?)));
        }
    }
    Ok(None)
}

/// 模块列表里含 address 的模块；没有为 None
fn find_module<R: Read + Seek>(
    r: &mut R,
    size: u32,
    rva: u32,
    address: u64,
) -> Result<Option<FaultModule>, DumpError> {
    let count = u32_at(&read_at(r, u64::from(rva), 4)?, 0)?;
    if count > MAX_MODULES {
        return Err(DumpError::Malformed("模块个数"));
    }
    let len = count as usize * MODULE_LEN;
    if (size as usize) < 4 + len {
        return Err(DumpError::Malformed("模块列表长度"));
    }
    let list = read_at(r, u64::from(rva) + 4, len)?;
    for m in list.as_chunks::<MODULE_LEN>().0 {
        let (base, size) = (u64_at(m, 0)?, u32_at(m, 8)?);
        if address
            .checked_sub(base)
            .is_some_and(|off| off < u64::from(size))
        {
            let path = read_name(r, u32_at(m, 20)?)?;
            let name = path.rsplit(['\\', '/']).next().unwrap_or(&path).to_owned();
            return Ok(Some(FaultModule { name, base, size }));
        }
    }
    Ok(None)
}

/// MINIDUMP_STRING：字节数 + UTF-16LE
fn read_name<R: Read + Seek>(r: &mut R, rva: u32) -> Result<String, DumpError> {
    let len = u32_at(&read_at(r, u64::from(rva), 4)?, 0)?;
    if len > MAX_NAME_BYTES {
        return Err(DumpError::Malformed("模块名长度"));
    }
    let bytes = read_at(r, u64::from(rva) + 4, len as usize)?;
    let units: Vec<u16> = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|c| u16::from_le_bytes(*c))
        .collect();
    Ok(String::from_utf16_lossy(&units))
}

fn u32_at(buf: &[u8], at: usize) -> Result<u32, DumpError> {
    let bytes = buf
        .get(at..at + 4)
        .and_then(|b| b.try_into().ok())
        .ok_or(DumpError::Truncated)?;
    Ok(u32::from_le_bytes(bytes))
}

fn u64_at(buf: &[u8], at: usize) -> Result<u64, DumpError> {
    let bytes = buf
        .get(at..at + 8)
        .and_then(|b| b.try_into().ok())
        .ok_or(DumpError::Truncated)?;
    Ok(u64::from_le_bytes(bytes))
}
