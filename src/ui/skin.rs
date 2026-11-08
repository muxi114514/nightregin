//! 原生界面素材：tools/extract_ui.ps1 从游戏菜单图集导出到 DLL 同目录 skin\ 的 PNG，叠加层初始化时上传为贴图。
//! 任何一张缺失或解码失败只影响用到它的控件（退回 ImGui 自带绘制），面板照常可用。

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use hudhook::RenderContext;
use hudhook::imgui::TextureId;
use log::{info, warn};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sprite {
    Background,
    PanelFrame,
    PanelFill,
    Button,
    ButtonHot,
    RowHighlight,
    Arrow,
    Line,
    TitleGlow,
    DiamondOn,
    DiamondOff,
    KeyUpDown,
    KeyLeftRight,
    KeyQ,
    KeyE,
    KeyEnter,
    KeyEsc,
}

/// 顺序必须与 Sprite 一致；文件名与 tools/extract_ui.py 的输出名一致
const FILES: [(Sprite, &str); 17] = [
    (Sprite::Background, "background"),
    (Sprite::PanelFrame, "panel_frame"),
    (Sprite::PanelFill, "panel_fill"),
    (Sprite::Button, "button"),
    (Sprite::ButtonHot, "button_hot"),
    (Sprite::RowHighlight, "row_highlight"),
    (Sprite::Arrow, "arrow"),
    (Sprite::Line, "line"),
    (Sprite::TitleGlow, "title_glow"),
    (Sprite::DiamondOn, "diamond_on"),
    (Sprite::DiamondOff, "diamond_off"),
    (Sprite::KeyUpDown, "key_updown"),
    (Sprite::KeyLeftRight, "key_leftright"),
    (Sprite::KeyQ, "key_q"),
    (Sprite::KeyE, "key_e"),
    (Sprite::KeyEnter, "key_enter"),
    (Sprite::KeyEsc, "key_esc"),
];

#[derive(Debug, Clone, Copy)]
pub struct Texture {
    pub id: TextureId,
    /// 原始像素尺寸
    pub size: [f32; 2],
}

#[derive(Default)]
pub struct Skin {
    textures: [Option<Texture>; FILES.len()],
    /// 武器图标（skin\icons\<5 位图标编号>.png）
    icons: HashMap<u16, Texture>,
    /// 按需加载时已尝试过的图标（缺文件的不再重试），以及所在目录
    attempted: HashSet<u16>,
    icon_dir: Option<PathBuf>,
}

impl Skin {
    /// 渲染管线（重新）创建时调用；贴图属于该管线，重建后要重新上传。icons：要加载的武器图标编号
    pub fn load(dir: &Path, render_context: &mut dyn RenderContext, icons: &[u16]) -> Self {
        let mut skin = Self::default();
        if !dir.is_dir() {
            info!("skin folder {} not found, using plain style", dir.display());
            return skin;
        }
        for (i, (_, name)) in FILES.iter().enumerate() {
            if let Some(t) = upload(render_context, &dir.join(format!("{name}.png"))) {
                skin.textures[i] = Some(t);
            }
        }
        skin.icon_dir = Some(dir.join("icons"));
        skin.ensure_icons(render_context, icons);
        let loaded = skin.textures.iter().flatten().count();
        info!("skin: {loaded}/{} sprites, {}/{} weapon icons loaded from {}", FILES.len(), skin.icons.len(), icons.len(), dir.display());
        skin
    }

    pub fn get(&self, sprite: Sprite) -> Option<Texture> {
        self.textures[sprite as usize]
    }

    pub fn icon(&self, id: u16) -> Option<Texture> {
        self.icons.get(&id).copied()
    }

    /// 按需加载图标（词条继承面板只加载背包里武器的图标）；每个编号只尝试一次。渲染管线重建后 Skin 整个重建，贴图随之重新上传
    pub fn ensure_icons(&mut self, render_context: &mut dyn RenderContext, ids: &[u16]) {
        let Some(dir) = &self.icon_dir else { return };
        for &id in ids {
            if self.attempted.insert(id)
                && let Some(t) = upload(render_context, &dir.join(format!("{id:05}.png")))
            {
                self.icons.insert(id, t);
            }
        }
    }
}

/// 解码并上传；失败时记日志返回 None
fn upload(render_context: &mut dyn RenderContext, path: &Path) -> Option<Texture> {
    let uploaded = decode_rgba(path).and_then(|(rgba, w, h)| {
        let id = render_context.load_texture(&rgba, w, h).map_err(|e| format!("upload failed: {e:?}"))?;
        Ok(Texture { id, size: [w as f32, h as f32] })
    });
    uploaded.map_err(|e| warn!("skin {}: {e}", path.display())).ok()
}

/// 解码为 8 位 RGBA
fn decode_rgba(path: &Path) -> Result<(Vec<u8>, u32, u32), String> {
    let file = File::open(path).map_err(|e| e.to_string())?;
    let mut decoder = png::Decoder::new(BufReader::new(file));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let frame = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
    buf.truncate(frame.buffer_size());
    let rgba = match frame.color_type {
        png::ColorType::Rgba => buf,
        png::ColorType::Rgb => buf.chunks_exact(3).flat_map(|p| [p[0], p[1], p[2], 255]).collect(),
        png::ColorType::GrayscaleAlpha => buf.chunks_exact(2).flat_map(|p| [p[0], p[0], p[0], p[1]]).collect(),
        png::ColorType::Grayscale => buf.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        other => return Err(format!("unsupported color type {other:?}")),
    };
    Ok((rgba, frame.width, frame.height))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_table_matches_enum_order() {
        for (i, (sprite, _)) in FILES.iter().enumerate() {
            assert_eq!(*sprite as usize, i);
        }
    }

    /// 部署目录里已导出素材时，校验每张都能解码
    #[test]
    fn deployed_sprites_decode() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../me3/NightreignPLUS/StatAlloc/skin");
        if !dir.is_dir() {
            return;
        }
        for (_, name) in FILES {
            let (rgba, w, h) = decode_rgba(&dir.join(format!("{name}.png"))).unwrap();
            assert_eq!(rgba.len(), (w * h * 4) as usize, "{name}");
        }
    }
}
