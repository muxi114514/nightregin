//! 右下角午夜队伍侧栏：最多 6 行（名字、午夜等级、状态配色）；出征中缩成一行角标。

use hudhook::imgui::{Condition, Ui};

use super::text::tx;
use crate::midnight::bridge::{MidnightSnapshot, RosterRow};
use crate::midnight::mode::RosterStatus;
use crate::ui::{fonts, theme};

fn status_index(s: RosterStatus) -> usize {
    match s {
        RosterStatus::Ready => 0,
        RosterStatus::NotReady => 1,
        RosterStatus::Relics => 2,
        RosterStatus::Uncounted => 3,
        RosterStatus::Unmodded => 4,
    }
}

fn status_color(s: RosterStatus) -> [f32; 4] {
    match s {
        RosterStatus::Ready => theme::OK,
        RosterStatus::Relics => theme::DECREASE,
        RosterStatus::Uncounted => theme::WARN,
        RosterStatus::NotReady | RosterStatus::Unmodded => theme::DIM,
    }
}

/// 字体图集里没有的字显示不出来，名字含这种字时退回「队友 N」
pub(super) fn shown_name(row: &RosterRow, glyphs: &[u32], teammate: &str) -> String {
    match &row.name {
        Some(name) if fonts::covers(glyphs, name) => name.clone(),
        _ => format!("{teammate} {}", row.slot + 1),
    }
}

pub fn draw(ui: &Ui, s: &MidnightSnapshot, glyphs: &[u32]) {
    let [w, h] = ui.io().display_size;
    let t = tx();
    ui.window("##nr_midnight_roster")
        .position([w * 0.985, h * 0.80], Condition::Always)
        .position_pivot([1.0, 1.0])
        .no_decoration()
        .always_auto_resize(true)
        .movable(false)
        .focus_on_appearing(false)
        .no_inputs()
        .build(|| {
            if s.in_expedition {
                ui.text_colored(theme::ACCENT, format!("{} · {}", t.roster_title, s.roster.len() + 1));
                return;
            }
            ui.text_colored(theme::ACCENT, t.roster_title);
            for row in s.roster.iter().take(6) {
                ui.text(format!("{}　Lv {}", shown_name(row, glyphs, t.teammate), row.level));
                ui.same_line();
                ui.text_colored(status_color(row.status), t.status[status_index(row.status)]);
            }
        });
}
