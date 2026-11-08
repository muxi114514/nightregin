//! 中文字体：运行时读取系统字体，只为界面实际用到的字生成字形，字体图集很小。
//! 字体数据由 ImGui 复制一份，图集上传到显卡后由叠加层调用 clear_input_data 释放。

use std::path::PathBuf;

use hudhook::imgui::{Context, FontConfig, FontGlyphRanges, FontSource};
use log::{info, warn};

/// 依次尝试：华文中宋、华文宋体（接近游戏菜单的宋体，随 Office 安装）、宋体，最后退回微软雅黑 / 黑体
const CANDIDATES: [&str; 6] = ["STZHONGS.TTF", "STSONG.TTF", "simsun.ttc", "msyh.ttc", "msyh.ttf", "simhei.ttf"];

/// 把若干段文字里出现的字符合并成 ImGui 字形范围（成对闭区间，以 0 结尾）。
/// 安装时调用一次，结果需在整个进程内有效，所以泄漏为 'static（几 KB）。
pub fn glyph_ranges(texts: &[&str]) -> &'static [u32] {
    let mut cps: Vec<u32> = (0x20..=0x7E).collect();
    cps.extend(texts.iter().flat_map(|t| t.chars()).map(|c| c as u32).filter(|&c| c > 0x7E));
    cps.sort_unstable();
    cps.dedup();
    let mut ranges: Vec<(u32, u32)> = Vec::new();
    for c in cps {
        match ranges.last_mut() {
            Some((_, end)) if *end + 1 == c => *end = c,
            _ => ranges.push((c, c)),
        }
    }
    let mut flat: Vec<u32> = ranges.into_iter().flat_map(|(a, b)| [a, b]).collect();
    flat.push(0);
    Box::leak(flat.into_boxed_slice())
}

/// 这段文字的每个字是否都在字形范围里（ranges 为 glyph_ranges 的结果）。
/// 范围首尾排成升序数组，二分后落在奇数位 = 在某个区间内，落在偶数位且相等 = 正好是区间起点
pub fn covers(ranges: &[u32], text: &str) -> bool {
    let bounds = ranges.strip_suffix(&[0]).unwrap_or(ranges);
    text.chars().all(|c| {
        let cp = c as u32;
        let i = bounds.partition_point(|&b| b < cp);
        i % 2 == 1 || bounds.get(i) == Some(&cp)
    })
}

/// 加入界面字体（作为默认字体）。找不到中文字体时返回 false，ImGui 会退回内置英文字体
pub fn add_ui_font(ctx: &mut Context, custom: &str, size_px: f32, glyphs: &'static [u32]) -> bool {
    let Some((path, data)) = read_font(custom) else {
        warn!("no CJK font found (set [ui] font), Chinese text will not render");
        return false;
    };
    let config = FontConfig {
        glyph_ranges: FontGlyphRanges::from_slice(glyphs),
        oversample_h: 2,
        pixel_snap_h: true,
        ..FontConfig::default()
    };
    ctx.fonts().add_font(&[FontSource::TtfData { data: &data, size_pixels: size_px, config: Some(config) }]);
    info!("font {} at {size_px:.0}px, {} glyph ranges", path.display(), glyphs.len() / 2);
    true
}

fn read_font(custom: &str) -> Option<(PathBuf, Vec<u8>)> {
    let mut paths = Vec::new();
    if !custom.trim().is_empty() {
        paths.push(PathBuf::from(custom.trim()));
    }
    let fonts_dir = std::env::var_os("WINDIR").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(r"C:\Windows")).join("Fonts");
    paths.extend(CANDIDATES.iter().map(|n| fonts_dir.join(n)));
    paths.into_iter().find_map(|p| std::fs::read(&p).ok().map(|d| (p, d)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn covers_checks_every_char() {
        let r = glyph_ranges(&["执行者"]);
        assert!(covers(r, "abc 123"));
        assert!(covers(r, "执行者"));
        assert!(covers(r, ""));
        assert!(!covers(r, "梅琳娜"));
        assert!(!covers(r, "abc梅"));
    }

    /// 单字范围的上下界相同，区间首尾都要算在内
    #[test]
    fn covers_handles_range_edges() {
        let r = glyph_ranges(&["\u{4e00}\u{4e01}\u{4e03}"]);
        for c in ['\u{4e00}', '\u{4e01}', '\u{4e03}', ' ', '~'] {
            assert!(covers(r, &c.to_string()), "{c:?}");
        }
        for c in ['\u{4e02}', '\u{4e04}', '\u{7f}'] {
            assert!(!covers(r, &c.to_string()), "{c:?}");
        }
    }

    #[test]
    fn ranges_merge_consecutive_and_end_with_zero() {
        let r = glyph_ranges(&["执行者", "执行"]);
        assert_eq!(r.last(), Some(&0));
        assert_eq!(r.len() % 2, 1);
        assert_eq!(&r[..2], &[0x20, 0x7E]);
        // 3 个不相邻的汉字各占一对
        assert_eq!(r.len(), 2 + 3 * 2 + 1);
    }
}
