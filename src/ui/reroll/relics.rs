//! 左栏上部：当前角色装备的 6 件遗物（上排普通、下排深夜），图标格底边按遗物颜色着色，点击选中。

use hudhook::imgui::Ui;
use nr_stat_alloc_shared::relic_reroll::RelicCatalog;

use crate::data::relic_names;
use crate::relic_reroll::RelicView;

use super::super::skin::Skin;
use super::super::{sfx, theme, widgets};
use super::{RerollPanel, tx};

/// 遗物颜色（EquipParamAntique 的颜色字段：火燃、水滴、光耀、幽静）
const COLORS: [[f32; 4]; 4] = [[0.86, 0.36, 0.30, 1.0], [0.36, 0.56, 0.95, 1.0], [0.95, 0.80, 0.36, 1.0], [0.46, 0.80, 0.52, 1.0]];
const COLUMNS: usize = 3;

impl RerollPanel {
    pub(super) fn relic_slots(&mut self, ui: &Ui, skin: &Skin, view: &RelicView, catalog: &RelicCatalog) {
        widgets::section(ui, skin, tx().section_relics);
        let fs = ui.current_font_size();
        let size = fs * 3.6;
        let gap = fs * 0.6;
        let label_w = [tx().row_normal, tx().row_deep].iter().map(|l| ui.calc_text_size(l)[0]).fold(0.0, f32::max) + fs;
        for (row, label) in [tx().row_normal, tx().row_deep].iter().enumerate() {
            let [x0, y0] = ui.cursor_pos();
            ui.set_cursor_pos([x0, y0 + (size - ui.text_line_height()) * 0.5]);
            ui.text_disabled(label);
            for col in 0..COLUMNS {
                let slot = row * COLUMNS + col;
                ui.set_cursor_pos([x0 + label_w + col as f32 * (size + gap), y0]);
                self.relic_tile(ui, skin, view, catalog, slot, size);
            }
            ui.set_cursor_pos([x0, y0 + size + gap]);
            ui.dummy([0.0, 0.0]);
        }
        if let Some(relic) = self.selected() {
            let name = relic_names::relic(relic.relic).map_or_else(|| format!("#{}", relic.relic), str::to_string);
            ui.text_colored(theme::ACCENT, name);
        }
    }

    fn relic_tile(&mut self, ui: &Ui, skin: &Skin, view: &RelicView, catalog: &RelicCatalog, slot: usize, size: f32) {
        let relic = view.slots.get(slot).and_then(Option::as_ref);
        let p0 = ui.cursor_screen_pos();
        let clicked = ui.invisible_button(format!("##relic{slot}"), [size, size]);
        sfx::track_hover(ui);
        let hovered = ui.is_item_hovered();
        let selected = slot == self.slot && relic.is_some();
        let row = relic.and_then(|r| catalog.relic(r.relic));
        let icon = row.and_then(|r| u16::try_from(r.icon_id).ok()).and_then(|id| skin.icon(id));
        let fixed = relic.is_some_and(|r| !catalog.rerollable(r.relic));
        widgets::weapon_icon(ui, icon, p0, size, selected || hovered, fixed);
        let p1 = [p0[0] + size, p0[1] + size];
        match row {
            Some(r) => {
                let color = COLORS.get(r.color as usize).copied().unwrap_or(theme::DIM);
                ui.get_window_draw_list().add_rect([p0[0] + 2.0, p1[1] - size * 0.06], [p1[0] - 2.0, p1[1] - 2.0], color).filled(true).build();
            }
            None => {
                let ts = ui.calc_text_size(tx().empty);
                ui.get_window_draw_list().add_text([p0[0] + (size - ts[0]) * 0.5, p0[1] + (size - ts[1]) * 0.5], theme::DIM, tx().empty);
            }
        }
        if let Some(r) = relic {
            if hovered {
                ui.tooltip_text(relic_names::relic(r.relic).unwrap_or_default());
            }
            if clicked && slot != self.slot {
                self.slot = slot;
                self.targets.clear();
                self.touch();
            }
        }
    }
}
