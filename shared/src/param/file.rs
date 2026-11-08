//! PARAM 文件解析（黑夜君临格式：64 位数据偏移、类型名放在字符串区、行名为 UTF-16）。
//! 头 0x40 字节：+0x00 字符串区偏移、+0x0A 行数、+0x10 类型名偏移、+0x2D 格式标志、+0x30 数据区起点；
//! 之后是每行 0x18 字节的描述（ID、数据偏移、名字偏移），偏移都相对文件开头。
//! 游戏加载后会重排字符串与行数据的位置，但描述里的偏移始终有效，所以这里只按描述取数据，不假设布局顺序。

pub const HEADER_SIZE: usize = 0x40;
pub const DESCRIPTOR_SIZE: usize = 0x18;
/// +0x2D 格式标志：数据偏移为 64 位、类型名以偏移形式存放
const FORMAT_LONG_DATA_OFFSET: u8 = 0x04;
const FORMAT_OFFSET_PARAM_TYPE: u8 = 0x80;

pub struct ParamFile<'a> {
    bytes: &'a [u8],
    rows: usize,
    row_size: usize,
    /// 描述按 ID 升序（文件里总是如此；内存中的副本以防万一不作假设）
    sorted: bool,
}

pub struct Row<'a> {
    pub index: usize,
    pub id: u32,
    /// 行数据相对表开头的偏移（改写游戏内存中的表时用）
    pub offset: usize,
    pub data: &'a [u8],
}

impl<'a> ParamFile<'a> {
    /// row_size：该表的行大小（来自 paramdef），每一行的数据都必须完整落在 bytes 内
    pub fn parse(bytes: &'a [u8], row_size: usize) -> Result<Self, String> {
        if bytes.len() < HEADER_SIZE {
            return Err("param file too short".into());
        }
        let format = bytes[0x2D];
        if format & FORMAT_LONG_DATA_OFFSET == 0 || format & FORMAT_OFFSET_PARAM_TYPE == 0 {
            return Err(format!("unsupported param format flags {format:#04x}"));
        }
        let rows = u16::from_le_bytes([bytes[0x0A], bytes[0x0B]]) as usize;
        if HEADER_SIZE + rows * DESCRIPTOR_SIZE > bytes.len() {
            return Err(format!("{rows} row descriptors exceed file size {}", bytes.len()));
        }
        let mut file = Self { bytes, rows, row_size, sorted: false };
        for i in 0..rows {
            let offset = file.data_offset(i);
            if offset.checked_add(row_size).is_none_or(|end| end > bytes.len()) {
                return Err(format!("row {} data at {offset:#x} out of bounds", file.id(i)));
            }
        }
        file.sorted = (1..rows).all(|i| file.id(i - 1) < file.id(i));
        Ok(file)
    }

    /// 结构体类型名，例如 "SP_EFFECT_PARAM_ST"
    pub fn struct_name(&self) -> Option<&'a str> {
        let offset = u64_at(self.bytes, 0x10) as usize;
        let tail = self.bytes.get(offset..)?;
        let end = tail.iter().position(|&b| b == 0)?;
        std::str::from_utf8(&tail[..end]).ok()
    }

    pub fn len(&self) -> usize {
        self.rows
    }

    pub fn is_empty(&self) -> bool {
        self.rows == 0
    }

    pub fn row_size(&self) -> usize {
        self.row_size
    }

    pub fn bytes(&self) -> &'a [u8] {
        self.bytes
    }

    pub fn rows(&self) -> impl Iterator<Item = Row<'a>> + '_ {
        (0..self.rows).map(|i| self.row_at(i))
    }

    /// 按 ID 查找（有序时二分）
    pub fn row(&self, id: u32) -> Option<&'a [u8]> {
        if !self.sorted {
            return self.rows().find(|r| r.id == id).map(|r| r.data);
        }
        let (mut lo, mut hi) = (0, self.rows);
        while lo < hi {
            let mid = (lo + hi) / 2;
            match self.id(mid).cmp(&id) {
                std::cmp::Ordering::Less => lo = mid + 1,
                std::cmp::Ordering::Greater => hi = mid,
                std::cmp::Ordering::Equal => return Some(self.row_at(mid).data),
            }
        }
        None
    }

    /// 行名（UTF-16）；没有名字或偏移无效时返回 None
    pub fn name(&self, index: usize) -> Option<String> {
        let offset = u64_at(self.bytes, self.descriptor(index) + 0x10) as usize;
        let tail = self.bytes.get(offset..)?;
        let units: Vec<u16> =
            tail.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).take_while(|&u| u != 0).collect();
        (!units.is_empty()).then(|| String::from_utf16_lossy(&units))
    }

    fn row_at(&self, index: usize) -> Row<'a> {
        let offset = self.data_offset(index);
        Row { index, id: self.id(index), offset, data: &self.bytes[offset..offset + self.row_size] }
    }

    fn descriptor(&self, index: usize) -> usize {
        HEADER_SIZE + index * DESCRIPTOR_SIZE
    }

    fn id(&self, index: usize) -> u32 {
        let d = self.descriptor(index);
        u32::from_le_bytes(self.bytes[d..d + 4].try_into().unwrap())
    }

    fn data_offset(&self, index: usize) -> usize {
        u64_at(self.bytes, self.descriptor(index) + 8) as usize
    }
}

fn u64_at(bytes: &[u8], offset: usize) -> u64 {
    bytes.get(offset..offset + 8).map_or(0, |b| u64::from_le_bytes(b.try_into().unwrap()))
}

/// 行内字段读取（小端），偏移越界返回 None
pub mod field {
    fn take<const N: usize>(row: &[u8], offset: usize) -> Option<[u8; N]> {
        row.get(offset..offset + N).map(|b| b.try_into().unwrap())
    }

    pub fn u8(row: &[u8], offset: usize) -> Option<u8> {
        row.get(offset).copied()
    }

    pub fn i8(row: &[u8], offset: usize) -> Option<i8> {
        row.get(offset).map(|&b| b as i8)
    }

    pub fn i16(row: &[u8], offset: usize) -> Option<i16> {
        take(row, offset).map(i16::from_le_bytes)
    }

    pub fn u16(row: &[u8], offset: usize) -> Option<u16> {
        take(row, offset).map(u16::from_le_bytes)
    }

    pub fn i32(row: &[u8], offset: usize) -> Option<i32> {
        take(row, offset).map(i32::from_le_bytes)
    }

    pub fn f32(row: &[u8], offset: usize) -> Option<f32> {
        take(row, offset).map(f32::from_le_bytes)
    }

    /// 连续 N 个 f32
    pub fn f32s<const N: usize>(row: &[u8], offset: usize) -> Option<[f32; N]> {
        let mut out = [0.0; N];
        for (i, o) in out.iter_mut().enumerate() {
            *o = f32(row, offset + i * 4)?;
        }
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 两行、行大小 4 的最小 PARAM 文件
    fn sample() -> Vec<u8> {
        let mut b = vec![0u8; HEADER_SIZE];
        b[0x0A] = 2;
        b[0x2D] = 0x85;
        let data = HEADER_SIZE + 2 * DESCRIPTOR_SIZE;
        let type_offset = data + 8;
        b[0x10..0x18].copy_from_slice(&(type_offset as u64).to_le_bytes());
        for (i, id) in [10u32, 20].iter().enumerate() {
            b.extend_from_slice(&id.to_le_bytes());
            b.extend_from_slice(&[0; 4]);
            b.extend_from_slice(&((data + i * 4) as u64).to_le_bytes());
            b.extend_from_slice(&((type_offset + 5) as u64).to_le_bytes());
        }
        b.extend_from_slice(&7i32.to_le_bytes());
        b.extend_from_slice(&1.5f32.to_le_bytes());
        b.extend_from_slice(b"TEST\0\0\0");
        b
    }

    #[test]
    fn parses_rows_and_type() {
        let bytes = sample();
        let p = ParamFile::parse(&bytes, 4).unwrap();
        assert_eq!(p.struct_name(), Some("TEST"));
        assert_eq!(p.len(), 2);
        assert_eq!(p.row(10).and_then(|r| field::i32(r, 0)), Some(7));
        assert_eq!(p.row(20).and_then(|r| field::f32(r, 0)), Some(1.5));
        assert!(p.row(15).is_none());
        assert_eq!(p.name(0), None);
    }

    #[test]
    fn rejects_rows_out_of_bounds() {
        let bytes = sample();
        assert!(ParamFile::parse(&bytes, 64).is_err());
        assert!(ParamFile::parse(&bytes[..0x50], 4).is_err());
    }
}
