//! 深夜面板下方的午夜开关与「打开午夜界面」入口，锚在 16:9 界面区域的比例位置（取自深夜面板截图，实机微调）。

use hudhook::imgui::{Condition, Ui};

use super::text::tx;
use crate::midnight::bridge::MidnightSnapshot;
use crate::ui::skin::Skin;
use crate::ui::{theme, widgets};

const TOGGLE_POS: [f32; 2] = [0.48, 0.63];
const GAME_UI_ASPECT: f32 = 16.0 / 9.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MidnightAction {
    Toggle,
    OpenHome,
}

pub fn draw(ui: &Ui, skin: &Skin, s: &MidnightSnapshot) -> Option<MidnightAction> {
    let [w, h] = ui.io().display_size;
    let ui_w = w.min(h * GAME_UI_ASPECT);
    let pos = [(w - ui_w) * 0.5 + ui_w * TOGGLE_POS[0], h * TOGGLE_POS[1]];
    let t = tx();
    let mut action = None;
    ui.window("##nr_midnight_toggle")
        .position(pos, Condition::Always)
        .no_decoration()
        .draw_background(false)
        .always_auto_resize(true)
        .movable(false)
        .focus_on_appearing(false)
        .build(|| {
            let pad = ui.current_font_size() * 3.0;
            let label = if !s.is_host {
                t.host_decides
            } else if s.active {
                t.toggle_on
            } else {
                t.toggle_off
            };
            let toggle_w = ui.calc_text_size(t.host_decides)[0].max(ui.calc_text_size(t.toggle_off)[0]) + pad;
            let clicked = {
                let _d = ui.begin_disabled(!s.can_toggle);
                widgets::button_hot(ui, skin, &format!("{label}##nr_midnight_toggle_go"), [toggle_w, 0.0], s.active)
            };
            if clicked {
                action = Some(MidnightAction::Toggle);
            }
            ui.same_line();
            if widgets::button(ui, skin, &format!("{}##nr_midnight_home", t.open_home), [ui.calc_text_size(t.open_home)[0] + pad, 0.0]) {
                action = Some(MidnightAction::OpenHome);
            }
            ui.text_colored(theme::ACCENT, format!("{} {}　{} {}", t.level_prefix, s.level.level, t.points, s.points));
            if s.active && s.relics {
                ui.text_colored(theme::DECREASE, t.relics_warning);
            }
            ui.text_colored(theme::DIM, t.pad_hint);
        });
    action
}
