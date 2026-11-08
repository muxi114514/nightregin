//! 午夜主界面：等级、本级进度、积分与胜负；静默 / 荒芜两个入口（内容由后续子项目填，荒芜 6 级前锁定）。

use hudhook::imgui::{Condition, Key, ProgressBar, Ui};
use nr_stat_alloc_shared::midnight::barren_unlocked;

use super::text::tx;
use crate::midnight::bridge::MidnightSnapshot;
use crate::ui::skin::Skin;
use crate::ui::{theme, widgets};

pub fn barren_locked(s: &MidnightSnapshot) -> bool {
    !barren_unlocked(s.level.level)
}

/// 返回 false = 关闭
pub fn draw(ui: &Ui, skin: &Skin, s: &MidnightSnapshot) -> bool {
    let [w, h] = ui.io().display_size;
    let t = tx();
    let mut keep = true;
    ui.window("##nr_midnight_home")
        .position([w * 0.5, h * 0.5], Condition::Always)
        .position_pivot([0.5, 0.5])
        .size([w * 0.42, 0.0], Condition::Always)
        .no_decoration()
        .movable(false)
        .build(|| {
            let fs = ui.current_font_size();
            widgets::background(ui, skin, ui.window_size());
            ui.text_colored(theme::ACCENT, t.home_title);
            ui.text_colored(theme::ACCENT, format!("{} {}", t.level_prefix, s.level.level));
            let fraction = if s.level.needed == 0 { 1.0 } else { s.level.into_level as f32 / s.level.needed as f32 };
            let overlay = if s.level.needed == 0 { t.max_level.to_string() } else { format!("{} / {}", s.level.into_level, s.level.needed) };
            ProgressBar::new(fraction).overlay_text(&overlay).size([-1.0, fs * 1.2]).build(ui);
            ui.text(format!("{} {}　{} {} / {}", t.points, s.points, t.record, s.wins, s.losses));
            ui.separator();
            entry(ui, t.silent, t.silent_desc, t.coming_soon, false);
            let locked = barren_locked(s);
            entry(ui, t.barren, t.barren_desc, if locked { t.barren_locked } else { t.coming_soon }, locked);
            ui.separator();
            if widgets::button(ui, skin, &format!("{}##nr_midnight_close", t.close), [ui.calc_text_size(t.close)[0] + fs * 4.0, 0.0]) || ui.is_key_pressed(Key::Escape) {
                keep = false;
            }
        });
    keep
}

fn entry(ui: &Ui, name: &str, desc: &str, state: &str, locked: bool) {
    ui.text_colored(if locked { theme::DIM } else { theme::TEXT }, name);
    ui.same_line();
    ui.text_colored(if locked { theme::WARN } else { theme::DIM }, format!("（{state}）"));
    ui.text_colored(theme::DIM, desc);
}
