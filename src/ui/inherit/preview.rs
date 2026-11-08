//! 右栏：来源与目标的武器、目标继承前后的词条、确认按钮与规则说明。

use hudhook::imgui::Ui;

use crate::bridge::UiBridge;
use crate::data::{InheritData, weapon_name};
use crate::passive_inherit::InheritWeapon;

use super::super::skin::Skin;
use super::super::{theme, widgets};
use super::rows::{passive_line, rarity_color};
use super::text::BULLET;
use super::{CONFIRM_FOCUS, InheritPanel, tx};

impl InheritPanel {
    pub(super) fn preview(&mut self, ui: &Ui, skin: &Skin, bridge: &UiBridge, data: &InheritData) {
        widgets::section(ui, skin, tx().section_preview);
        match self.source {
            None => ui.text_disabled(tx().pick_source_first),
            Some(source) => {
                ui.text_colored(theme::WARN, tx().source_label);
                weapon_line(ui, skin, data, &source.weapon);
                passive_list(ui, data, &source.weapon.pick.passives, theme::TEXT);
                ui.spacing();
                ui.text_colored(theme::OK, tx().target_label);
                match self.target {
                    None => ui.text_disabled(tx().pick_target),
                    Some(target) => {
                        weapon_line(ui, skin, data, &target.weapon);
                        ui.text_disabled(tx().current);
                        passive_list(ui, data, &target.weapon.pick.passives, theme::DIM);
                        ui.text_disabled(tx().after);
                        passive_list(ui, data, &source.weapon.pick.passives, theme::OK);
                        if target.weapon.pick.passives.iter().any(|&p| p >= 0) {
                            ui.text_colored(theme::WARN, tx().replaced);
                        }
                        ui.spacing();
                        self.inherit_button(ui, skin, bridge);
                    }
                }
            }
        }
        ui.spacing();
        ui.spacing();
        ui.text_disabled(tx().rule);
    }

    fn inherit_button(&mut self, ui: &Ui, skin: &Skin, bridge: &UiBridge) {
        let label = if self.pending {
            tx().pending
        } else if self.confirm {
            tx().confirm_again
        } else {
            tx().inherit_button
        };
        let hot = self.pad.active && self.pad.focus == CONFIRM_FOCUS;
        let width = ui.content_region_avail()[0] * 0.62;
        let clicked = {
            let _d = ui.begin_disabled(self.pending);
            widgets::button_hot(ui, skin, &format!("{label}##inherit_go"), [width, 0.0], hot)
        };
        if clicked {
            self.press_inherit(bridge);
        }
    }
}

/// 小图标 + 按稀有度着色的武器名 + 稀有度
fn weapon_line(ui: &Ui, skin: &Skin, data: &InheritData, weapon: &InheritWeapon) {
    let size = ui.text_line_height() * 1.6;
    let p0 = ui.cursor_screen_pos();
    let tex = data.icon(weapon.item_id).and_then(|id| skin.icon(id));
    widgets::weapon_icon(ui, tex, p0, size, false, false);
    ui.dummy([size, size]);
    ui.same_line();
    let [x, y] = ui.cursor_pos();
    ui.set_cursor_pos([x, y + (size - ui.text_line_height()) * 0.5]);
    ui.text_colored(rarity_color(data, weapon.item_id), weapon_name(weapon.item_id));
    if let Some(r) = data.rarities.current(weapon.item_id).and_then(|r| tx().rarity.get(r as usize)) {
        ui.same_line();
        ui.text_disabled(r);
    }
}

/// 每条词条一行，前面一个菱形点
fn passive_list(ui: &Ui, data: &InheritData, passives: &[i32], color: [f32; 4]) {
    let ids: Vec<i32> = passives.iter().copied().filter(|&p| p >= 0).collect();
    if ids.is_empty() {
        ui.text_colored(theme::DIM, passive_line(data, &ids));
        return;
    }
    for id in ids {
        ui.text_colored(color, format!("{BULLET}{}", data.passive_name(id)));
    }
}
