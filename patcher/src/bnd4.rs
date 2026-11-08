//! BND4 归档（regulation 解压后的内层）。只支持 regulation 用的格式 0x74（ID + 名字 + 大小字段），文件头每项 0x24 字节：
//! +0x08 压缩前大小、+0x10 压缩后大小（regulation 内不压缩，两者相等）、+0x18 数据偏移（u32）、+0x1C ID、+0x20 名字偏移。
//! 改写时保留数据区之前的全部内容（文件头、名字表、哈希表与文件名无关、不受数据偏移影响），只重排数据区，每个文件按 16 字节对齐。

const MAGIC: &[u8; 4] = b"BND4";
const FILE_HEADER_LEN: usize = 0x24;
const FORMAT: u8 = 0x74;
const ALIGN: usize = 16;

pub struct Bnd4 {
    bytes: Vec<u8>,
    entries: Vec<Entry>,
}

struct Entry {
    header: usize,
    offset: usize,
    len: usize,
    name: String,
}

impl Bnd4 {
    pub fn parse(bytes: Vec<u8>) -> Result<Self, String> {
        if bytes.len() < 0x40 || &bytes[..4] != MAGIC {
            return Err("not a BND4 archive".into());
        }
        let count = u32_at(&bytes, 0x0C) as usize;
        let header_len = u64_at(&bytes, 0x20) as usize;
        if header_len != FILE_HEADER_LEN || bytes[0x31] != FORMAT || bytes[0x30] != 1 {
            return Err(format!("unsupported BND4 layout (header {header_len:#x}, format {:#04x})", bytes[0x31]));
        }
        let mut entries = Vec::with_capacity(count);
        for i in 0..count {
            let header = 0x40 + i * FILE_HEADER_LEN;
            let len = u64_at(&bytes, header + 0x08) as usize;
            if len != u64_at(&bytes, header + 0x10) as usize {
                return Err(format!("file {i} is compressed inside BND4, unsupported"));
            }
            let offset = u32_at(&bytes, header + 0x18) as usize;
            if offset.checked_add(len).is_none_or(|end| end > bytes.len()) {
                return Err(format!("file {i} data out of bounds"));
            }
            let name = utf16_at(&bytes, u32_at(&bytes, header + 0x20) as usize);
            entries.push(Entry { header, offset, len, name });
        }
        if entries.windows(2).any(|w| w[0].offset + w[0].len > w[1].offset) {
            return Err("BND4 file data not in index order".into());
        }
        Ok(Self { bytes, entries })
    }

    /// 版本字符串（regulation 版本号，例如 "10350000"）
    pub fn version(&self) -> String {
        String::from_utf8_lossy(&self.bytes[0x18..0x20]).trim_end_matches('\0').to_string()
    }

    /// 文件名以 suffix 结尾（不区分大小写）的第一个文件：(序号, 数据)
    pub fn find(&self, suffix: &str) -> Option<(usize, &[u8])> {
        let suffix = suffix.to_ascii_lowercase();
        self.entries
            .iter()
            .position(|e| e.name.to_ascii_lowercase().ends_with(&suffix))
            .map(|i| (i, self.data(i)))
    }

    pub fn data(&self, index: usize) -> &[u8] {
        let e = &self.entries[index];
        &self.bytes[e.offset..e.offset + e.len]
    }

    /// 替换一个文件后重新排列数据区
    pub fn rebuild(&self, index: usize, replacement: &[u8]) -> Vec<u8> {
        self.rebuild_many(&[(index, replacement)])
    }

    /// 替换多个文件（序号, 新数据）后重新排列数据区
    pub fn rebuild_many(&self, replacements: &[(usize, &[u8])]) -> Vec<u8> {
        let data_start = self.entries.first().map_or(self.bytes.len(), |e| e.offset);
        let mut out = self.bytes[..data_start].to_vec();
        for (i, e) in self.entries.iter().enumerate() {
            let data = replacements.iter().find(|(r, _)| *r == i).map_or_else(|| self.data(i), |(_, d)| *d);
            out.resize(out.len().next_multiple_of(ALIGN), 0);
            let offset = out.len();
            out.extend_from_slice(data);
            let len = (data.len() as u64).to_le_bytes();
            out[e.header + 0x08..e.header + 0x10].copy_from_slice(&len);
            out[e.header + 0x10..e.header + 0x18].copy_from_slice(&len);
            out[e.header + 0x18..e.header + 0x1C].copy_from_slice(&(offset as u32).to_le_bytes());
        }
        out
    }
}

fn u32_at(bytes: &[u8], offset: usize) -> u32 {
    bytes.get(offset..offset + 4).map_or(0, |b| u32::from_le_bytes(b.try_into().unwrap()))
}

fn u64_at(bytes: &[u8], offset: usize) -> u64 {
    bytes.get(offset..offset + 8).map_or(0, |b| u64::from_le_bytes(b.try_into().unwrap()))
}

fn utf16_at(bytes: &[u8], offset: usize) -> String {
    let units: Vec<u16> = bytes
        .get(offset..)
        .unwrap_or(&[])
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .take_while(|&u| u != 0)
        .collect();
    String::from_utf16_lossy(&units)
}
