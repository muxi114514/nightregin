//! 左栏（照法环升级界面）：等级切换、可分配 / 剩余点数、8 项能力值"上一级 → 本级"，选中行带 ◀ ▶，底部确定。

use hudhook::imgui::{SelectableFlags, StyleVar, Ui};

use crate::bridge::SchemeSnapshot;
use crate::bridge::UiBridge;
use crate::stats::growth::ARCANE;
use crate::stats::{STAT_COUNT, stat_name};

use super::super::skin::Skin;
use super::super::{sfx, text, theme, widgets};
use super::gamepad::{BUTTONS, Focus};
use super::{Panel, compare_color, text_right};

/// 能力值行内各列的位置（占栏宽的比例）
const COL_BEFORE: f32 = 0.46;
const COL_ARROW: f32 = 0.50;
const COL_DEC: f32 = 0.58;
const COL_AFTER: f32 = 0.76;
const COL_INC: f32 = 0.78;

impl Panel {
    pub(super) fn attributes(&mut self, ui: &Ui, skin: &Skin, snap: &SchemeSnapshot, bridge: &UiBridge) {
        let Some(draft) = &self.draft else { return };
        let level = self.level;
        let v = draft.view(level);
        let notes: Vec<String> = draft.notes().iter().take(2).cloned().collect();
        let fs = ui.current_font_size();
        let x0 = ui.cursor_pos()[0];
        let width = ui.content_region_avail()[0];

        // 等级：◀ N ▶
        let mut level_delta = 0;
        ui.text(text::tx().level);
        ui.same_line_with_pos(x0 + width * COL_DEC);
        if widgets::arrow(ui, skin, "##nr_lv_prev", false) {
            level_delta = -1;
        }
        text_right(ui, x0, width * COL_AFTER, theme::ACCENT, &level.to_string());
        ui.same_line_with_pos(x0 + width * COL_INC);
        if widgets::arrow(ui, skin, "##nr_lv_next", true) {
            level_delta = 1;
        }
        ui.same_line();
        ui.text_disabled(format!("{}{}", text::tx().current_level, snap.level));

        // 点数（法环"持有卢恩 / 所需卢恩"的位置）
        let remaining = v.budget.saturating_sub(v.assigned);
        ui.spacing();
        ui.text(text::tx().budget);
        text_right(ui, x0, width * COL_AFTER, theme::TEXT, &v.budget.to_string());
        ui.text(text::tx().remaining);
        text_right(ui, x0, width * COL_AFTER, if remaining > 0 { theme::WARN } else { theme::TEXT }, &remaining.to_string());
        if v.refund > 0 {
            ui.text(text::tx().refund);
            text_right(ui, x0, width * COL_AFTER, theme::INCREASE, &v.refund.to_string());
        }
        ui.text_disabled(if v.manual { text::tx().manual } else { text::tx().auto });
        ui.spacing();

        // 能力值
        widgets::section(ui, skin, text::tx().section_stats);
        let mut action: Option<(usize, i32)> = None;
        let mut picked = None;
        {
            // 行内箭头按钮去掉上下内边距，与文字同高，行距一致
            let _pad = ui.push_style_var(StyleVar::FramePadding([fs * 0.2, 0.0]));
            for i in 0..STAT_COUNT {
                let selected = i == self.stat;
                let locked = level > 1 && i == ARCANE;
                if selected {
                    let p0 = ui.cursor_screen_pos();
                    widgets::row_highlight(ui, skin, p0, [p0[0] + width, p0[1] + ui.text_line_height()]);
                }
                if ui
                    .selectable_config(format!("##nr_row{i}"))
                    .flags(SelectableFlags::ALLOW_ITEM_OVERLAP)
                    .size([width, 0.0])
                    .build()
                {
                    picked = Some(i);
                }
                sfx::track_hover(ui);
                ui.same_line_with_pos(x0 + fs * 0.3);
                ui.text_colored(if selected { theme::ACCENT } else { theme::TEXT }, stat_name(i));
                text_right(ui, x0, width * COL_BEFORE, theme::TEXT, &v.before[i].to_string());
                ui.same_line_with_pos(x0 + width * COL_ARROW);
                ui.text_disabled(text::tx().arrow);
                if selected && locked {
                    ui.same_line_with_pos(x0 + width * COL_DEC);
                    ui.text_disabled(text::tx().arcane_locked);
                    continue;
                }
                let _repeat = ui.push_button_repeat(true);
                if selected {
                    ui.same_line_with_pos(x0 + width * COL_DEC);
                    if widgets::arrow(ui, skin, &format!("##nr_dec{i}"), false) {
                        action = Some((i, -1));
                    }
                }
                text_right(ui, x0, width * COL_AFTER, compare_color(v.after[i], v.before[i]), &v.after[i].to_string());
                if selected {
                    ui.same_line_with_pos(x0 + width * COL_INC);
                    if widgets::arrow(ui, skin, &format!("##nr_inc{i}"), true) {
                        action = Some((i, 1));
                    }
                }
                let diff = v.after[i] as i64 - v.vanilla[i] as i64;
                if diff != 0 {
                    let label = if diff > 0 { format!("+{diff}") } else { diff.to_string() };
                    text_right(ui, x0, width - fs * 0.3, if diff > 0 { theme::INCREASE } else { theme::DECREASE }, &label);
                }
            }
        }

        // 确定与次要操作
        ui.spacing();
        ui.spacing();
        let hot: [bool; BUTTONS] = std::array::from_fn(|i| self.pad.hot(Focus::Button(i)));
        let confirm = widgets::button_hot(ui, skin, text::tx().confirm, [width, fs * 2.1], hot[0]);
        let half = (width - ui.clone_style().item_spacing[0]) / 2.0;
        let revert = widgets::button_hot(ui, skin, text::tx().revert, [half, 0.0], hot[1]);
        ui.same_line();
        let reset_all = widgets::button_hot(ui, skin, text::tx().reset_all, [half, 0.0], hot[2]);
        let reset = widgets::button_hot(ui, skin, text::tx().reset_level, [half, 0.0], hot[3]);
        ui.same_line();
        let clear = widgets::button_hot(ui, skin, text::tx().clear_level, [half, 0.0], hot[4]);

        // 提示：自动补点、规则层提示、感应规则
        ui.spacing();
        if remaining > 0 {
            ui.text_disabled(text::tx().auto_fill_hint);
        }
        for note in &notes {
            ui.text_wrapped(note);
        }
        if level == 1 {
            ui.text_wrapped(text::tx().arcane_rule);
        }

        if let Some(i) = picked {
            self.select_stat(i);
        }
        if level_delta != 0 {
            self.change_level(level_delta);
        }
        if let Some((i, delta)) = action {
            self.with_draft(|d| d.adjust(level, i, delta));
        }
        if confirm {
            self.save(snap, bridge);
        }
        if revert {
            self.with_draft(|d| {
                d.revert();
                Ok(())
            });
        }
        if reset_all {
            self.with_draft(|d| {
                d.reset_all();
                Ok(())
            });
        }
        if reset {
            self.with_draft(|d| d.reset_level(level));
        }
        if clear {
            self.with_draft(|d| d.clear_level(level));
        }
    }
}
