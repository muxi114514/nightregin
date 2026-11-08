//! DCX 压缩容器（黑夜君临 regulation 用 ZSTD）。头 0x4C 字节，数值为大端：
//! +0x1C 解压后大小、+0x20 压缩后大小、+0x28 压缩算法 "ZSTD"、+0x30 压缩等级；之后是 ZSTD 数据。
//! 写回时沿用原文件的头，只改大小与等级。ZSTD 参数照搬 SoulsFormats 的 WriteZstd（不写内容大小、windowLog 16），
//! DCX 末尾零填充到 16 字节：游戏内置的是 1.1 时代的旧版 zstd，默认窗口（22）的帧实机解压时破坏堆并触发 DL_PANIC。

const HEADER_LEN: usize = 0x4C;
/// 与 SoulsFormats 写 regulation 时相同的等级与窗口
const LEVEL: i32 = 15;
const WINDOW_LOG: u32 = 16;
const ALIGN: usize = 16;

pub struct Dcx {
    header: [u8; HEADER_LEN],
}

impl Dcx {
    /// 返回容器头与解压后的数据
    pub fn decompress(bytes: &[u8]) -> Result<(Self, Vec<u8>), String> {
        if bytes.len() < HEADER_LEN || &bytes[..4] != b"DCX\0" || &bytes[0x24..0x28] != b"DCP\0" {
            return Err("not a DCX container".into());
        }
        let algorithm = &bytes[0x28..0x2C];
        if algorithm != b"ZSTD" {
            return Err(format!("unsupported DCX compression {:?} (only ZSTD)", String::from_utf8_lossy(algorithm)));
        }
        let raw_len = be_u32(bytes, 0x1C) as usize;
        let packed_len = be_u32(bytes, 0x20) as usize;
        let packed = bytes.get(HEADER_LEN..HEADER_LEN + packed_len).ok_or("DCX data truncated")?;
        let data = zstd::bulk::decompress(packed, raw_len).map_err(|e| format!("ZSTD decompress: {e}"))?;
        if data.len() != raw_len {
            return Err(format!("DCX size mismatch: {} != {raw_len}", data.len()));
        }
        Ok((Self { header: bytes[..HEADER_LEN].try_into().unwrap() }, data))
    }

    pub fn compress(&self, data: &[u8]) -> Result<Vec<u8>, String> {
        let packed = zstd_frame(data).map_err(|e| format!("ZSTD compress: {e}"))?;
        let mut out = Vec::with_capacity((HEADER_LEN + packed.len()).next_multiple_of(ALIGN));
        out.extend_from_slice(&self.header);
        out[0x1C..0x20].copy_from_slice(&(data.len() as u32).to_be_bytes());
        out[0x20..0x24].copy_from_slice(&(packed.len() as u32).to_be_bytes());
        out[0x30] = LEVEL as u8;
        out.extend_from_slice(&packed);
        out.resize(out.len().next_multiple_of(ALIGN), 0);
        Ok(out)
    }
}

/// 不带内容大小字段、窗口 64KB 的 ZSTD 帧（游戏用 DCX 头里的解压后大小）
fn zstd_frame(data: &[u8]) -> std::io::Result<Vec<u8>> {
    let mut compressor = zstd::bulk::Compressor::new(LEVEL)?;
    compressor.include_contentsize(false)?;
    compressor.window_log(WINDOW_LOG)?;
    compressor.compress(data)
}

fn be_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    const ZSTD_MAGIC: [u8; 4] = [0x28, 0xB5, 0x2F, 0xFD];

    fn dcx() -> Dcx {
        let mut header = [0u8; HEADER_LEN];
        header[..4].copy_from_slice(b"DCX\0");
        header[0x24..0x28].copy_from_slice(b"DCP\0");
        header[0x28..0x2C].copy_from_slice(b"ZSTD");
        Dcx { header }
    }

    #[test]
    fn frame_matches_soulsformats_layout() {
        let data: Vec<u8> = (0..200_000u32).map(|i| (i % 251) as u8).collect();
        let out = dcx().compress(&data).unwrap();
        assert_eq!(out[HEADER_LEN..HEADER_LEN + 4], ZSTD_MAGIC);
        // 帧头描述符高 2 位为内容大小字段长度标记，第 5 位为单段标记（需要内容大小），都应为 0
        assert_eq!(out[HEADER_LEN + 4] & 0xE0, 0);
        // 窗口描述符：高 5 位为 windowLog - 10，低 3 位尾数为 0
        assert_eq!(out[HEADER_LEN + 5], ((WINDOW_LOG - 10) << 3) as u8);
        assert_eq!(out.len() % ALIGN, 0);
        let end = HEADER_LEN + be_u32(&out, 0x20) as usize;
        assert!(out[end..].iter().all(|&b| b == 0));
        assert_eq!(Dcx::decompress(&out).unwrap().1, data);
    }
}
