//! 游戏主模块（nightreign.exe）的基址与节表，从内存中的 PE 头解析。

use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;

use super::memory::read;

#[derive(Debug, Clone)]
pub struct Section {
    pub name: String,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone)]
pub struct GameModule {
    pub base: usize,
    pub size: usize,
    pub timestamp: u32,
    pub sections: Vec<Section>,
}

impl GameModule {
    pub fn current() -> Option<Self> {
        let base = unsafe { GetModuleHandleW(std::ptr::null()) } as usize;
        if base == 0 {
            return None;
        }
        let e_lfanew = read::<u32>(base + 0x3C)? as usize;
        let nt = base + e_lfanew;
        if read::<u32>(nt)? != 0x4550 {
            return None;
        }
        let section_count = read::<u16>(nt + 6)? as usize;
        let timestamp = read::<u32>(nt + 8)?;
        let optional_size = read::<u16>(nt + 20)? as usize;
        let size = read::<u32>(nt + 24 + 56)? as usize;

        let mut sections = Vec::with_capacity(section_count);
        let mut sh = nt + 24 + optional_size;
        for _ in 0..section_count {
            let raw: [u8; 8] = read(sh)?;
            let name = String::from_utf8_lossy(&raw).trim_end_matches('\0').to_string();
            let vsize = read::<u32>(sh + 8)? as usize;
            let va = read::<u32>(sh + 12)? as usize;
            sections.push(Section { name, start: base + va, end: base + va + vsize });
            sh += 40;
        }
        Some(Self { base, size, timestamp, sections })
    }

    pub fn va(&self, rva: usize) -> usize {
        self.base + rva
    }

    pub fn contains(&self, addr: usize) -> bool {
        addr >= self.base && addr < self.base + self.size
    }

    /// 同名节可能有多个（Arxan 会追加第二个 .text），返回第一个
    pub fn section(&self, name: &str) -> Option<&Section> {
        self.sections.iter().find(|s| s.name == name)
    }
}
