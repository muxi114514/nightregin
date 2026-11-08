//! 左栏：右手、左手各 3 个武器槽。每行图标 + 按当前稀有度着色的武器名 + 槽位 + 词条；来源、目标、档位不足各有标记。

use hudhook::imgui::Ui;

use crate::data::{InheritData, weapon_name};
use crate::passive_inherit::{InheritView, InheritWeapon};

use super::super::skin::Skin;
use super::super::{sfx, theme, widgets};
use super::{HANDS, InheritPanel, tx};

/// 当前稀有度 0~3 的颜色：白、蓝、紫、金
pub(super) const RARITY_COLORS: [[f32; 4]; 4] = [[0.88, 0.88, 0.88, 1.0], [0.45, 0.65, 1.0, 1.0], [0.76, 0.52, 0.98, 1.0], [0.98, 0.80, 0.38, 1.0]];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RowState {
    Empty,
    Source,
    Target,
    /// 已选来源，这把档位比来源高
    Blocked,
    Selectable,
}

impl InheritPanel {
    pub(super) fn weapon_list(&mut self, ui: &Ui, skin: &Skin, view: &InheritView, data: &InheritData) {
        widgets::section(ui, skin, tx().section_weapons);
        let step = match (self.source, self.target) {
            (None, _) => tx().step_source,
            (Some(_), None) => tx().step_target,
            _ => tx().step_confirm,
        };
        ui.text_colored(theme::ACCENT, step);
        for (hand_index, slots) in HANDS.iter().enumerate() {
            let hand = if hand_index == 0 { tx().right_hand } else { tx().left_hand };
            ui.spacing();
            for (n, &slot) in slots.iter().enumerate() {
                let focus = hand_index * slots.len() + n;
                let state = self.row_state(slot, view.slots[slot]);
                let hot = self.pad.active && self.pad.focus == focus;
                if weapon_row(ui, skin, data, view.slots[slot], state, hot, &format!("{hand} {}", n + 1), slot) {
                    self.activate(slot);
                }
            }
        }
    }

    fn row_state(&self, slot: usize, weapon: Option<InheritWeapon>) -> RowState {
        match (weapon, self.source, self.target) {
            (None, ..) => RowState::Empty,
            (_, Some(s), _) if s.slot == slot => RowState::Source,
            (_, _, Some(t)) if t.slot == slot => RowState::Target,
            (_, Some(_), _) if !self.can_target(slot) => RowState::Blocked,
            _ => RowState::Selectable,
        }
    }
}

/// 一行武器；返回是否被点击（空槽不可点）
#[allow(clippy::too_many_arguments)]
fn weapon_row(ui: &Ui, skin: &Skin, data: &InheritData, weapon: Option<InheritWeapon>, state: RowState, hot: bool, slot_label: &str, slot: usize) -> bool {
    let fs = ui.current_font_size();
    let pad = fs * 0.35;
    let icon = fs * 2.6;
    let start = ui.cursor_pos();
    let p0 = ui.cursor_screen_pos();
    let width = ui.content_region_avail()[0];
    let text_x = pad * 2.0 + icon;
    let wrap = width - text_x - pad;
    let passives = weapon.map(|w| passive_line(data, &w.pick.passives)).unwrap_or_default();
    let passives_h = ui.calc_text_size_with_opts(&passives, false, wrap)[1];
    let height = icon.max(ui.text_line_height_with_spacing() + passives_h) + pad * 2.0;
    let pressed = ui.invisible_button(format!("##inherit_slot{slot}"), [width, height]);
    sfx::track_hover(ui);
    let enabled = state != RowState::Empty;
    let hovered = enabled && ui.is_item_hovered();
    let p1 = [p0[0] + width, p0[1] + height];
    if hovered || hot {
        ui.get_window_draw_list().add_rect(p0, p1, [0.30, 0.45, 0.75, 0.12]).filled(true).build();
    }
    let (border, thickness) = match state {
        RowState::Source => (theme::WARN, 2.0),
        RowState::Target => (theme::OK, 2.0),
        _ if hot => (theme::ACCENT, 2.0),
        _ => (theme::LINE, 1.0),
    };
    ui.get_window_draw_list().add_rect(p0, p1, border).thickness(thickness).build();
    let tex = weapon.and_then(|w| data.icon(w.item_id)).and_then(|id| skin.icon(id));
    let dimmed = matches!(state, RowState::Blocked);
    widgets::weapon_icon(ui, tex, [p0[0] + pad, p0[1] + (height - icon) * 0.5], icon, matches!(state, RowState::Source | RowState::Target), dimmed);
    // 第一行：武器名（按稀有度着色）+ 标记；槽位靠右
    ui.set_cursor_screen_pos([p0[0] + text_x, p0[1] + pad]);
    match weapon {
        Some(w) => {
            let color = if dimmed { theme::DIM } else { rarity_color(data, w.item_id) };
            ui.text_colored(color, weapon_name(w.item_id));
        }
        None => ui.text_disabled(tx().empty),
    }
    let tag = match state {
        RowState::Source => Some((theme::WARN, tx().tag_source)),
        RowState::Target => Some((theme::OK, tx().tag_target)),
        RowState::Blocked => Some((theme::DECREASE, tx().tag_lower)),
        _ => None,
    };
    if let Some((color, text)) = tag {
        ui.same_line_with_spacing(0.0, fs);
        ui.text_colored(color, text);
    }
    let label_w = ui.calc_text_size(slot_label)[0];
    ui.same_line_with_pos(p1[0] - ui.window_pos()[0] - pad - label_w);
    ui.text_disabled(slot_label);
    // 第二行：词条
    if weapon.is_some() {
        ui.set_cursor_screen_pos([p0[0] + text_x, p0[1] + pad + ui.text_line_height_with_spacing()]);
        let _wrap = ui.push_text_wrap_pos_with_pos(p1[0] - pad - ui.window_pos()[0]);
        ui.text_colored(if dimmed { theme::DIM } else { theme::TEXT }, &passives);
    }
    ui.set_cursor_pos([start[0], start[1] + height + ui.clone_style().item_spacing[1]]);
    ui.dummy([0.0, 0.0]);
    pressed && enabled
}

/// 词条名连成一行；全空时显示"无词条"
pub(super) fn passive_line(data: &InheritData, passives: &[i32]) -> String {
    let names: Vec<String> = passives.iter().filter(|&&p| p >= 0).map(|&p| data.passive_name(p)).collect();
    if names.is_empty() { tx().no_passives.to_string() } else { names.join(crate::lang::pick("、", ", ")) }
}

pub(super) fn rarity_color(data: &InheritData, item_id: u32) -> [f32; 4] {
    data.rarities.current(item_id).and_then(|r| RARITY_COLORS.get(r as usize)).copied().unwrap_or(theme::TEXT)
}
