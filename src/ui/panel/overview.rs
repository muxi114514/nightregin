//! 右栏：成长总览（1~15 级最终属性，与原版对比着色，点击等级切换）、成长权重、方案管理（复制到 / 删除）。

use hudhook::imgui::{MouseButton, StyleColor, Ui};

use crate::bridge::{SchemeCommand, SchemeSnapshot, UiBridge, UiCommand};
use crate::data::GameData;
use crate::editor::MAX_WEIGHT;
use crate::stats::growth::{GROWTH_STATS, MAX_LEVEL};
use crate::stats::{STAT_COUNT, stat_name};

use super::super::sfx::{self, Cue};
use super::super::skin::Skin;
use super::super::{text, theme, widgets};
use super::gamepad::Focus;
use super::{Panel, compare_color, text_right};

impl Panel {
    pub(super) fn overview(&mut self, ui: &Ui, skin: &Skin, snap: &SchemeSnapshot, data: &GameData, bridge: &UiBridge) {
        self.growth_table(ui, skin, snap.level);
        ui.spacing();
        self.weights(ui, skin);
        ui.spacing();
        self.scheme_actions(ui, skin, snap, data, bridge);
    }

    fn growth_table(&mut self, ui: &Ui, skin: &Skin, current_level: u32) {
        let Some(draft) = &self.draft else { return };
        widgets::section(ui, skin, text::tx().overview);
        ui.text_disabled(text::tx().overview_hint);

        let fs = ui.current_font_size();
        let level_w = fs * 2.6;
        let step = (ui.content_region_avail()[0] - level_w) / (STAT_COUNT as f32 + 1.2);
        let x0 = ui.cursor_pos()[0];
        ui.text_disabled(text::tx().col_level);
        for (i, short) in text::tx().stat_short.iter().enumerate() {
            ui.same_line_with_pos(x0 + level_w + step * i as f32);
            ui.text_disabled(short);
        }
        ui.same_line_with_pos(x0 + level_w + step * STAT_COUNT as f32);
        ui.text_disabled(text::tx().col_total);

        let mut picked = None;
        let row_w = ui.content_region_avail()[0];
        for level in 1..=MAX_LEVEL {
            let (stats, vanilla) = (draft.final_at(level), draft.vanilla_at(level));
            if level == self.level {
                let p0 = ui.cursor_screen_pos();
                widgets::row_highlight(ui, skin, p0, [p0[0] + row_w, p0[1] + ui.text_line_height()]);
            }
            {
                let color = if draft.is_manual(level) { theme::ACCENT } else { theme::TEXT };
                let _c = ui.push_style_color(StyleColor::Text, color);
                let mark = if level == current_level { "*" } else { "" };
                let label = format!("Lv{level}{mark}##nr_ov{level}");
                if ui.selectable_config(label).size([level_w - fs * 0.3, 0.0]).build() {
                    picked = Some(level);
                }
                sfx::track_hover(ui);
            }
            for i in 0..STAT_COUNT {
                ui.same_line_with_pos(x0 + level_w + step * i as f32);
                ui.text_colored(compare_color(stats[i], vanilla[i]), format!("{:>2}", stats[i]));
            }
            ui.same_line_with_pos(x0 + level_w + step * STAT_COUNT as f32);
            ui.text_disabled(format!("{:>3}", stats.iter().sum::<u32>()));
        }
        if let Some(level) = picked {
            self.change_level(level as i32 - self.level as i32);
        }
    }

    /// 成长权重：法环设置菜单式的"< 3 >"选择器，两列排布
    fn weights(&mut self, ui: &Ui, skin: &Skin) {
        let Some(draft) = &self.draft else { return };
        widgets::section(ui, skin, text::tx().weights);
        ui.text_wrapped(text::tx().weights_hint);
        let x0 = ui.cursor_pos()[0];
        let col_w = ui.content_region_avail()[0] / 2.0;
        let weights = draft.weights();
        let mut changed = None;
        let _repeat = ui.push_button_repeat(true);
        for i in 0..GROWTH_STATS {
            let left = x0 + col_w * (i % 2) as f32;
            if i % 2 == 1 {
                ui.same_line_with_pos(left);
            }
            if self.pad.hot(Focus::Weight(i)) {
                let p0 = ui.cursor_screen_pos();
                widgets::row_highlight(ui, skin, p0, [p0[0] + col_w * 0.9, p0[1] + ui.text_line_height()]);
            }
            ui.text(stat_name(i));
            ui.same_line_with_pos(left + col_w * 0.42);
            if widgets::arrow(ui, skin, &format!("##nr_wd{i}"), false) && weights[i] > 0 {
                changed = Some((i, weights[i] - 1));
            }
            text_right(ui, x0, left - x0 + col_w * 0.68, theme::ACCENT, &weights[i].to_string());
            ui.same_line_with_pos(left + col_w * 0.74);
            if widgets::arrow(ui, skin, &format!("##nr_wi{i}"), true) && weights[i] < MAX_WEIGHT {
                changed = Some((i, weights[i] + 1));
            }
        }
        if let Some((i, w)) = changed {
            self.with_draft(|d| d.set_weight(i, w));
        }
    }

    fn scheme_actions(&mut self, ui: &Ui, skin: &Skin, snap: &SchemeSnapshot, data: &GameData, bridge: &UiBridge) {
        widgets::section(ui, skin, text::tx().section_schemes);
        let width = ui.content_region_avail()[0];
        let half = (width - ui.clone_style().item_spacing[0]) / 2.0;
        let (copy_hot, delete_hot) = (self.pad.hot(Focus::Copy), self.pad.hot(Focus::Delete));
        if widgets::button_hot(ui, skin, text::tx().copy_to, [half, 0.0], copy_hot) || std::mem::take(&mut self.pad.open_copy) {
            ui.open_popup("##nr_copy_popup");
        }
        if let Some(_popup) = ui.begin_popup("##nr_copy_popup") {
            let targets = self.copy_targets(snap, data);
            self.pad.copy_len = targets.len();
            for (i, (target, name)) in targets.into_iter().enumerate() {
                let picked = self.pad.copy_cursor == Some(i);
                // 鼠标点击或手柄按 A 选中都会写入该目标
                let clicked = ui.selectable_config(name).selected(picked).build();
                sfx::track_hover(ui);
                if clicked || (picked && self.pad.copy_accept) {
                    sfx::request(Cue::Confirm);
                    if let Some(d) = &self.draft {
                        bridge.send(UiCommand::Scheme(SchemeCommand::Save { hero: snap.hero, vessel: target, spec: d.spec().clone() }));
                    }
                    ui.close_current_popup();
                }
            }
            if self.pad.copy_cancel || self.pad.copy_accept {
                ui.close_current_popup();
                self.pad.copy_cursor = None;
            }
        } else {
            self.pad.copy_cursor = None;
        }
        self.pad.copy_accept = false;
        self.pad.copy_cancel = false;
        ui.same_line();
        let label = if self.confirm_delete { text::tx().confirm_delete } else { text::tx().delete };
        if widgets::button_hot(ui, skin, label, [half, 0.0], delete_hot) {
            self.request_delete(snap, bridge);
        } else if self.confirm_delete && !delete_hot && ui.is_mouse_clicked(MouseButton::Left) && !ui.is_item_hovered() {
            self.confirm_delete = false;
        }
    }

    /// "复制到"可选的目标：角色默认方案与该角色可用的所有器皿（不含当前目标）
    fn copy_targets<'a>(&self, snap: &SchemeSnapshot, data: &'a GameData) -> Vec<(Option<u32>, &'a str)> {
        let current = self.target(snap);
        let mut out = Vec::new();
        if current.is_some() {
            out.push((None, text::tx().copy_default));
        }
        out.extend(
            data.vessels
                .iter()
                .filter(|v| (v.hero.is_none() || v.hero == Some(snap.hero)) && Some(v.id) != current)
                .map(|v| (Some(v.id), v.display_name())),
        );
        out
    }
}
