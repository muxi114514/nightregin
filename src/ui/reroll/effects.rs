//! 左栏中部：选中遗物的词条，每条一个方框、左侧圆圈表示是否选中；负面词条缩进排在同栏正面词条下方，可单独选。

use hudhook::imgui::Ui;
use nr_stat_alloc_shared::relic_reroll::{RelicCatalog, SLOTS, Target};

use crate::data::relic_names;
use crate::relic_reroll::EquippedRelic;

use super::super::skin::Skin;
use super::super::{sfx, theme, widgets};
use super::{RerollPanel, tx};

/// 词条显示名；不在名称表里时显示编号
pub(super) fn effect_name(catalog: &RelicCatalog, id: u32) -> String {
    catalog.effect(id).and_then(|e| relic_names::effect(e.text_id)).map_or_else(|| format!("#{id}"), str::to_string)
}

impl RerollPanel {
    pub(super) fn effect_list(&mut self, ui: &Ui, skin: &Skin, relic: &EquippedRelic, catalog: &RelicCatalog, rerollable: bool) {
        widgets::section(ui, skin, tx().section_effects);
        if rerollable {
            ui.text_disabled(tx().effects_hint);
        } else {
            ui.text_colored(theme::WARN, tx().fixed);
        }
        for i in 0..SLOTS {
            for target in [Target::Positive(i), Target::Curse(i)] {
                let Some(id) = relic.effects.get(target) else { continue };
                let curse = matches!(target, Target::Curse(_));
                let name = effect_name(catalog, id);
                let label = if curse { format!("{}{name}", tx().curse_prefix) } else { name };
                let selected = self.targets.contains(&target);
                let style = BoxStyle { selected, fresh: self.highlight.contains(&id), curse, enabled: rerollable };
                if effect_box(ui, &format!("##effect{i}{curse}"), &label, style) {
                    if selected {
                        self.targets.retain(|&t| t != target);
                    } else {
                        self.targets.push(target);
                    }
                    self.touch();
                }
            }
        }
    }
}

#[derive(Clone, Copy)]
struct BoxStyle {
    selected: bool,
    /// 上次重铸新出现的词条
    fresh: bool,
    curse: bool,
    enabled: bool,
}

/// 一条词条的方框：左侧圆圈（选中为实心）+ 自动换行的名称；返回是否被点击
fn effect_box(ui: &Ui, id: &str, label: &str, s: BoxStyle) -> bool {
    let fs = ui.current_font_size();
    let pad = fs * 0.45;
    let dot = fs * 0.9;
    let start = ui.cursor_pos();
    let indent = if s.curse { fs * 1.5 } else { 0.0 };
    ui.set_cursor_pos([start[0] + indent, start[1]]);
    let p0 = ui.cursor_screen_pos();
    let width = ui.content_region_avail()[0];
    let text_x = pad * 2.0 + dot;
    let ts = ui.calc_text_size_with_opts(label, false, width - text_x - pad);
    let size = [width, ts[1].max(fs) + pad * 2.0];
    let pressed = ui.invisible_button(id, size);
    sfx::track_hover(ui);
    let hovered = s.enabled && ui.is_item_hovered();
    let p1 = [p0[0] + size[0], p0[1] + size[1]];
    if hovered {
        ui.get_window_draw_list().add_rect(p0, p1, [0.30, 0.45, 0.75, 0.12]).filled(true).build();
    }
    let border = if s.selected { theme::ACCENT } else { theme::LINE };
    ui.get_window_draw_list().add_rect(p0, p1, border).thickness(if s.selected { 2.0 } else { 1.0 }).build();
    let center = [p0[0] + pad + dot * 0.5, p0[1] + size[1] * 0.5];
    let ring = if s.enabled { theme::ACCENT } else { theme::DIM };
    ui.get_window_draw_list().add_circle(center, dot * 0.4, ring).thickness(1.5).build();
    if s.selected {
        ui.get_window_draw_list().add_circle(center, dot * 0.25, theme::ACCENT).filled(true).build();
    }
    let color = if !s.enabled {
        theme::DIM
    } else if s.fresh {
        theme::OK
    } else if s.curse {
        theme::DECREASE
    } else if s.selected {
        theme::ACCENT
    } else {
        theme::TEXT
    };
    ui.set_cursor_screen_pos([p0[0] + text_x, p0[1] + pad]);
    {
        let _wrap = ui.push_text_wrap_pos_with_pos(p1[0] - pad - ui.window_pos()[0]);
        ui.text_colored(color, label);
    }
    ui.set_cursor_pos([start[0], start[1] + size[1] + ui.clone_style().item_spacing[1]]);
    ui.dummy([0.0, 0.0]);
    pressed && s.enabled
}
