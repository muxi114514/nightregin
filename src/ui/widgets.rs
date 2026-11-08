//! 用原生素材绘制的控件：九宫格面板、三段拉伸按钮、箭头、菱形单选、方形勾选框、武器 / 遗物图标格、高亮条、分隔线、按键提示。
//! 素材缺失时退回 ImGui 自带绘制，行为不变。

use hudhook::imgui::{Direction, Ui};

use super::sfx::{self, Cue};
use super::skin::{Skin, Sprite, Texture};
use super::theme;

const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
/// 面板边框素材四角的像素宽度（MENU_In_bg_17）
const FRAME_SRC_BORDER: f32 = 40.0;
/// 括号按钮两端不拉伸部分占素材高度的比例（MENU_In_bg_21）
const BUTTON_CAP_RATIO: f32 = 0.28;

fn image(ui: &Ui, tex: Texture, p0: [f32; 2], p1: [f32; 2], uv0: [f32; 2], uv1: [f32; 2], tint: [f32; 4]) {
    ui.get_window_draw_list().add_image(tex.id, p0, p1).uv_min(uv0).uv_max(uv1).col(tint).build();
}

/// 九宫格：四角按 border 原样缩放，边与中间拉伸
fn nine_slice(ui: &Ui, tex: Texture, p0: [f32; 2], p1: [f32; 2], border: f32, src_border: f32) {
    let (u, v) = (src_border / tex.size[0], src_border / tex.size[1]);
    let xs = [p0[0], p0[0] + border, p1[0] - border, p1[0]];
    let ys = [p0[1], p0[1] + border, p1[1] - border, p1[1]];
    let us = [0.0, u, 1.0 - u, 1.0];
    let vs = [0.0, v, 1.0 - v, 1.0];
    for i in 0..3 {
        for j in 0..3 {
            image(ui, tex, [xs[i], ys[j]], [xs[i + 1], ys[j + 1]], [us[i], vs[j]], [us[i + 1], vs[j + 1]], WHITE);
        }
    }
}

/// 横向三段：两端按高度等比缩放，中间拉伸
fn three_slice(ui: &Ui, tex: Texture, p0: [f32; 2], p1: [f32; 2], cap_ratio: f32) {
    let h = p1[1] - p0[1];
    let src_cap = tex.size[1] * cap_ratio;
    let cap = (src_cap * h / tex.size[1]).min((p1[0] - p0[0]) * 0.5);
    let u = src_cap / tex.size[0];
    let xs = [p0[0], p0[0] + cap, p1[0] - cap, p1[0]];
    let us = [0.0, u, 1.0 - u, 1.0];
    for i in 0..3 {
        image(ui, tex, [xs[i], p0[1]], [xs[i + 1], p1[1]], [us[i], 0.0], [us[i + 1], 1.0], WHITE);
    }
}

/// 全屏背景：按"铺满并裁切"保持素材比例
pub fn background(ui: &Ui, skin: &Skin, size: [f32; 2]) {
    let Some(tex) = skin.get(Sprite::Background) else { return };
    let (screen, art) = (size[0] / size[1], tex.size[0] / tex.size[1]);
    let (uv0, uv1) = if screen > art {
        let v = (1.0 - art / screen) * 0.5;
        ([0.0, v], [1.0, 1.0 - v])
    } else {
        let u = (1.0 - screen / art) * 0.5;
        ([u, 0.0], [1.0 - u, 1.0])
    };
    let origin = ui.window_pos();
    image(ui, tex, origin, [origin[0] + size[0], origin[1] + size[1]], uv0, uv1, WHITE);
}

/// 面板底与边框（画在父窗口上，面板内容用子窗口叠在上面）；没有素材时返回 false，调用方用子窗口边框代替
pub fn panel(ui: &Ui, skin: &Skin, p0: [f32; 2], p1: [f32; 2]) -> bool {
    let Some(frame) = skin.get(Sprite::PanelFrame) else { return false };
    if let Some(fill) = skin.get(Sprite::PanelFill) {
        let pad = ui.current_font_size();
        image(ui, fill, [p0[0] - pad, p0[1] - pad], [p1[0] + pad, p1[1] + pad], [0.0; 2], [1.0; 2], WHITE);
    }
    nine_slice(ui, frame, p0, p1, ui.current_font_size() * 1.2, FRAME_SRC_BORDER);
    true
}

/// 标题后的光晕
pub fn title_glow(ui: &Ui, skin: &Skin, p0: [f32; 2], p1: [f32; 2]) {
    if let Some(tex) = skin.get(Sprite::TitleGlow) {
        image(ui, tex, p0, p1, [0.0; 2], [1.0; 2], WHITE);
    }
}

/// 括号按钮；悬停或按下时换成发光样式，点击播放确定音。label 可带 "##" 后缀区分 ID
pub fn button(ui: &Ui, skin: &Skin, label: &str, size: [f32; 2]) -> bool {
    button_hot(ui, skin, label, size, false)
}

/// 同 button；hot 为 true 时（手柄焦点停在这里）也显示发光样式
pub fn button_hot(ui: &Ui, skin: &Skin, label: &str, size: [f32; 2], hot: bool) -> bool {
    let (normal, glow) = (skin.get(Sprite::Button), skin.get(Sprite::ButtonHot));
    let Some(normal) = normal else {
        let clicked = ui.button_with_size(label, size);
        sfx::track_hover(ui);
        if clicked {
            sfx::request(Cue::Confirm);
        }
        return clicked;
    };
    let size = [size[0], if size[1] > 0.0 { size[1] } else { ui.frame_height() * 1.25 }];
    let p0 = ui.cursor_screen_pos();
    let clicked = ui.invisible_button(label, size);
    sfx::track_hover(ui);
    if clicked {
        sfx::request(Cue::Confirm);
    }
    let active = hot || ui.is_item_hovered() || ui.is_item_active();
    let p1 = [p0[0] + size[0], p0[1] + size[1]];
    three_slice(ui, if active { glow.unwrap_or(normal) } else { normal }, p0, p1, BUTTON_CAP_RATIO);
    let text = label.split("##").next().unwrap_or(label);
    let ts = ui.calc_text_size(text);
    let pos = [p0[0] + (size[0] - ts[0]) * 0.5, p0[1] + (size[1] - ts[1]) * 0.5];
    ui.get_window_draw_list().add_text(pos, if active { theme::ACCENT } else { theme::TEXT }, text);
    clicked
}

/// 左右箭头按钮（游戏的 < > 箭头，右箭头为水平翻转）；外层可用 push_button_repeat 实现按住连发
pub fn arrow(ui: &Ui, skin: &Skin, id: &str, right: bool) -> bool {
    let Some(tex) = skin.get(Sprite::Arrow) else {
        return ui.arrow_button(id, if right { Direction::Right } else { Direction::Left });
    };
    let h = ui.text_line_height();
    let size = [h * tex.size[0] / tex.size[1] + h * 0.2, h];
    let p0 = ui.cursor_screen_pos();
    let clicked = ui.invisible_button(id, size);
    let tint = if ui.is_item_hovered() { theme::ACCENT } else { theme::TEXT };
    let (uv0, uv1) = if right { ([1.0, 0.0], [0.0, 1.0]) } else { ([0.0, 0.0], [1.0, 1.0]) };
    let x = p0[0] + h * 0.1;
    image(ui, tex, [x, p0[1]], [x + size[0] - h * 0.2, p0[1] + h], uv0, uv1, tint);
    clicked
}

/// 菱形单选（实心为选中）
pub fn diamond(ui: &Ui, skin: &Skin, label: &str, active: bool) -> bool {
    let sprite = skin.get(if active { Sprite::DiamondOn } else { Sprite::DiamondOff });
    let Some(tex) = sprite else {
        let clicked = ui.radio_button_bool(label, active);
        sfx::track_hover(ui);
        return clicked;
    };
    let h = ui.text_line_height();
    let ts = ui.calc_text_size(label);
    let p0 = ui.cursor_screen_pos();
    let clicked = ui.invisible_button(label, [h * 1.1 + ts[0], h]);
    sfx::track_hover(ui);
    let hovered = ui.is_item_hovered();
    let d = h * 0.7;
    let y = p0[1] + (h - d) * 0.5;
    image(ui, tex, [p0[0], y], [p0[0] + d, y + d], [0.0; 2], [1.0; 2], WHITE);
    let color = if active || hovered { theme::ACCENT } else { theme::DIM };
    ui.get_window_draw_list().add_text([p0[0] + h * 1.1, p0[1]], color, label);
    clicked
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Check {
    Off,
    Partial,
    On,
}

/// 方形勾选框（原版遗物筛选的样式）：实心为全选，中间横条为部分选中；返回是否被点击。label 可带 "##" 后缀
pub fn check(ui: &Ui, label: &str, state: Check) -> bool {
    let h = ui.text_line_height();
    let text = label.split("##").next().unwrap_or(label);
    let p0 = ui.cursor_screen_pos();
    let clicked = ui.invisible_button(label, [h * 1.3 + ui.calc_text_size(text)[0], h]);
    sfx::track_hover(ui);
    let hovered = ui.is_item_hovered();
    let s = h * 0.8;
    let y = p0[1] + (h - s) * 0.5;
    let (b0, b1) = ([p0[0], y], [p0[0] + s, y + s]);
    ui.get_window_draw_list().add_rect(b0, b1, [0.03, 0.05, 0.10, 0.9]).filled(true).build();
    ui.get_window_draw_list().add_rect(b0, b1, if hovered { theme::ACCENT } else { theme::LINE }).thickness(1.5).build();
    let inset = s * 0.22;
    match state {
        Check::On => ui.get_window_draw_list().add_rect([b0[0] + inset, b0[1] + inset], [b1[0] - inset, b1[1] - inset], theme::ACCENT).filled(true).build(),
        Check::Partial => ui.get_window_draw_list().add_rect([b0[0] + inset, y + s * 0.42], [b1[0] - inset, y + s * 0.58], theme::ACCENT).filled(true).build(),
        Check::Off => {}
    }
    let color = if hovered { theme::ACCENT } else if state == Check::Off { theme::DIM } else { theme::TEXT };
    ui.get_window_draw_list().add_text([p0[0] + h * 1.3, p0[1]], color, text);
    clicked
}

/// 武器图标格：深色底 + 图标，hot 时冰蓝描边；dimmed 时图标变暗（不可选）。
/// 每次取窗口绘制列表都只在一条语句内使用：imgui-rs 不允许同时持有两份，否则 panic
pub fn weapon_icon(ui: &Ui, icon: Option<Texture>, p0: [f32; 2], size: f32, hot: bool, dimmed: bool) {
    let p1 = [p0[0] + size, p0[1] + size];
    ui.get_window_draw_list().add_rect(p0, p1, [0.02, 0.03, 0.06, 0.85]).filled(true).build();
    if let Some(tex) = icon {
        image(ui, tex, p0, p1, [0.0; 2], [1.0; 2], if dimmed { [0.35, 0.35, 0.35, 0.6] } else { WHITE });
    }
    let (color, thickness) = if hot { (theme::ACCENT, 2.0) } else { (theme::GRID, 1.0) };
    ui.get_window_draw_list().add_rect(p0, p1, color).thickness(thickness).build();
}

/// 选中行的高亮条（画在行内容之前）
pub fn row_highlight(ui: &Ui, skin: &Skin, p0: [f32; 2], p1: [f32; 2]) {
    let pad = ui.text_line_height() * 0.18;
    let (p0, p1) = ([p0[0] - pad * 2.0, p0[1] - pad], [p1[0] + pad, p1[1] + pad]);
    match skin.get(Sprite::RowHighlight) {
        Some(tex) => image(ui, tex, p0, p1, [0.0; 2], [1.0; 2], WHITE),
        None => ui.get_window_draw_list().add_rect(p0, p1, [0.12, 0.22, 0.40, 0.75]).filled(true).build(),
    }
}

/// 栏目标题：冰蓝文字，下方一条分隔线
pub fn section(ui: &Ui, skin: &Skin, title: &str) {
    ui.text_colored(theme::ACCENT, title);
    let [x, y] = ui.cursor_screen_pos();
    let width = ui.content_region_avail()[0];
    let y = y - ui.clone_style().item_spacing[1] * 0.5;
    match skin.get(Sprite::Line) {
        Some(tex) => image(ui, tex, [x - 4.0, y - 2.0], [x + width + 4.0, y + 2.0], [0.0; 2], [1.0; 2], WHITE),
        None => ui.get_window_draw_list().add_line([x, y], [x + width, y], theme::LINE).build(),
    }
    ui.spacing();
}

/// 底部按键提示：按键图标 + 说明（没有图标时只写文字）
pub fn key_hint(ui: &Ui, skin: &Skin, keys: &[Sprite], label: &str) {
    let h = ui.text_line_height() * 1.25;
    for &key in keys {
        if let Some(tex) = skin.get(key) {
            let w = h * tex.size[0] / tex.size[1];
            let p0 = ui.cursor_screen_pos();
            image(ui, tex, [p0[0], p0[1] - h * 0.12], [p0[0] + w, p0[1] + h * 0.88], [0.0; 2], [1.0; 2], WHITE);
            ui.dummy([w, ui.text_line_height()]);
            ui.same_line();
        }
    }
    ui.text_disabled(label);
}

#[cfg(test)]
mod tests {
    use hudhook::imgui::{Context, TextureId};

    use super::*;

    /// 无渲染器的 imgui 帧里实际画一次：带图标、高亮、变暗各种组合都不能 panic（曾因同时持有两份绘制列表而 panic）
    #[test]
    fn weapon_icon_draws_without_nested_draw_lists() {
        let _imgui = crate::ui::imgui_test_lock();
        let mut ctx = Context::create();
        ctx.set_ini_filename(None);
        ctx.io_mut().display_size = [800.0, 600.0];
        ctx.fonts().build_rgba32_texture();
        let ui = ctx.new_frame();
        ui.window("smoke").build(|| {
            let tex = Texture { id: TextureId::new(1), size: [160.0, 160.0] };
            for (icon, hot, dimmed) in [(Some(tex), true, false), (Some(tex), false, true), (None, true, true)] {
                weapon_icon(ui, icon, [10.0, 10.0], 48.0, hot, dimmed);
            }
        });
        ctx.render();
    }
}
