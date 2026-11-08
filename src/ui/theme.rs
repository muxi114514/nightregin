//! 黑夜君临菜单风格的幽蓝配色（与 skin 原生素材搭配）：冰蓝标题与选中项、冷白正文；素材缺失时这些颜色也构成退回样式。

use hudhook::imgui::{Style, StyleColor};

/// 标题、选中项、强调
pub const ACCENT: [f32; 4] = [0.75, 0.84, 1.00, 1.0];
pub const TEXT: [f32; 4] = [0.89, 0.90, 0.93, 1.0];
pub const DIM: [f32; 4] = [0.55, 0.59, 0.66, 1.0];
/// 高于原版 / 上一级
pub const INCREASE: [f32; 4] = [0.38, 0.68, 1.00, 1.0];
/// 低于原版、错误提示
pub const DECREASE: [f32; 4] = [0.95, 0.44, 0.44, 1.0];
pub const OK: [f32; 4] = [0.50, 0.85, 0.72, 1.0];
pub const WARN: [f32; 4] = [1.00, 0.74, 0.40, 1.0];
/// 分隔线与边框
pub const LINE: [f32; 4] = [0.27, 0.42, 0.66, 0.55];
/// 武器图标格的默认描边
pub const GRID: [f32; 4] = [0.30, 0.40, 0.58, 0.25];

/// scale：以 1080p 为 1.0 的界面缩放
pub fn apply(style: &mut Style, scale: f32) {
    style.window_rounding = 0.0;
    style.child_rounding = 0.0;
    style.frame_rounding = 0.0;
    style.popup_rounding = 0.0;
    style.grab_rounding = 0.0;
    style.window_border_size = 0.0;
    style.child_border_size = 1.0;
    style.frame_border_size = 1.0;
    style.window_padding = [16.0, 12.0];
    style.frame_padding = [12.0, 5.0];
    style.item_spacing = [10.0, 7.0];
    style.window_title_align = [0.5, 0.5];
    style.scale_all_sizes(scale);

    let colors = [
        (StyleColor::Text, TEXT),
        (StyleColor::TextDisabled, DIM),
        // 全屏底色：盖住后面的遗物装配界面，与"预设"子页一样
        (StyleColor::WindowBg, [0.012, 0.022, 0.050, 0.95]),
        (StyleColor::ChildBg, [0.030, 0.055, 0.110, 0.55]),
        (StyleColor::PopupBg, [0.025, 0.045, 0.095, 0.98]),
        (StyleColor::Border, LINE),
        (StyleColor::FrameBg, [0.05, 0.09, 0.17, 1.0]),
        (StyleColor::FrameBgHovered, [0.10, 0.18, 0.32, 1.0]),
        (StyleColor::FrameBgActive, [0.14, 0.25, 0.44, 1.0]),
        (StyleColor::Button, [0.05, 0.09, 0.17, 1.0]),
        (StyleColor::ButtonHovered, [0.12, 0.22, 0.40, 1.0]),
        (StyleColor::ButtonActive, [0.18, 0.32, 0.56, 1.0]),
        (StyleColor::Header, [0.12, 0.22, 0.40, 0.75]),
        // 选中行用素材高亮条，悬停只给一点淡底
        (StyleColor::HeaderHovered, [0.30, 0.45, 0.75, 0.12]),
        (StyleColor::HeaderActive, [0.30, 0.45, 0.75, 0.20]),
        (StyleColor::SliderGrab, ACCENT),
        (StyleColor::SliderGrabActive, [0.80, 0.92, 1.0, 1.0]),
        (StyleColor::CheckMark, ACCENT),
        (StyleColor::Separator, LINE),
        (StyleColor::ScrollbarBg, [0.02, 0.04, 0.08, 0.6]),
        (StyleColor::ScrollbarGrab, [0.18, 0.30, 0.50, 1.0]),
        (StyleColor::NavHighlight, ACCENT),
    ];
    for (slot, color) in colors {
        style[slot] = color;
    }
}
