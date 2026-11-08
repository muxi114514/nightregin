//! 中栏：基本能力（本级三维"上一级 → 本级"）、初始武器（weapons.rs）、满级（15 级）预览。

use hudhook::imgui::Ui;

use crate::bridge::SchemeSnapshot;
use crate::data::{GameData, StartingData};
use crate::stats::curve::{VitalAdjust, VitalCurves};
use crate::stats::growth::{MAX_LEVEL, Stats};

use super::super::skin::Skin;
use super::super::{text, theme, widgets};
use super::{Panel, compare_color, text_right};

const COL_BEFORE: f32 = 0.46;
const COL_ARROW: f32 = 0.50;
const COL_AFTER: f32 = 0.76;

impl Panel {
    pub(super) fn vitals(&mut self, ui: &Ui, skin: &Skin, snap: &SchemeSnapshot, data: &GameData, starting: Option<&StartingData>) {
        let Some(draft) = &self.draft else { return };
        let level = self.level;
        let x0 = ui.cursor_pos()[0];
        let width = ui.content_region_avail()[0];
        let curves = data.vital_curves.as_ref();
        let adjust = curves.map(|c| self.vital_adjust(c, snap.hero)).unwrap_or_default();

        widgets::section(ui, skin, text::tx().section_vitals);
        match curves {
            None => ui.text_wrapped(text::tx().no_curves),
            Some(c) => {
                let before_stats = if level == 1 { draft.vanilla_at(1) } else { draft.final_at(level - 1) };
                let (before, after) = (c.vitals(&before_stats, &adjust), c.vitals(&draft.final_at(level), &adjust));
                for (i, name) in text::tx().vital_names.iter().enumerate() {
                    ui.text(name);
                    text_right(ui, x0, width * COL_BEFORE, theme::TEXT, &before[i].to_string());
                    ui.same_line_with_pos(x0 + width * COL_ARROW);
                    ui.text_disabled(text::tx().arrow);
                    text_right(ui, x0, width * COL_AFTER, compare_color(after[i], before[i]), &after[i].to_string());
                }
            }
        }
        ui.spacing();

        self.weapons(ui, skin, snap, starting);
        ui.spacing();
        let Some(draft) = &self.draft else { return };

        widgets::section(ui, skin, text::tx().section_max);
        let (scheme15, vanilla15) = (draft.final_at(MAX_LEVEL), draft.vanilla_at(MAX_LEVEL));
        if let Some(c) = curves {
            let (mine, base) = (c.vitals(&scheme15, &adjust), c.vitals(&vanilla15, &adjust));
            for (i, name) in text::tx().vital_names.iter().enumerate() {
                ui.text(name);
                text_right(ui, x0, width * COL_BEFORE, compare_color(mine[i], base[i]), &mine[i].to_string());
                vanilla_note(ui, x0, width, mine[i], base[i]);
            }
        }
        let (total, vanilla_total) = (sum(&scheme15), sum(&vanilla15));
        ui.text(text::tx().total_points);
        text_right(ui, x0, width * COL_BEFORE, compare_color(total, vanilla_total), &total.to_string());
        vanilla_note(ui, x0, width, total, vanilla_total);
    }

    /// 只有实时属性属于正在编辑的角色时才计入遗物修正
    fn vital_adjust(&self, curves: &VitalCurves, hero: usize) -> VitalAdjust {
        match self.live {
            Some(live) if live.hero == hero => curves.adjust(&live.effective, &live.target, live.max),
            _ => VitalAdjust::default(),
        }
    }
}

/// "原版 X  ±d"
fn vanilla_note(ui: &Ui, x0: f32, width: f32, value: u32, vanilla: u32) {
    ui.same_line_with_pos(x0 + width * COL_ARROW);
    ui.text_disabled(format!("{}{vanilla}", text::tx().vanilla_prefix));
    let diff = value as i64 - vanilla as i64;
    if diff != 0 {
        let label = if diff > 0 { format!("+{diff}") } else { diff.to_string() };
        text_right(ui, x0, width - ui.current_font_size() * 0.3, if diff > 0 { theme::INCREASE } else { theme::DECREASE }, &label);
    }
}

fn sum(stats: &Stats) -> u32 {
    stats.iter().sum()
}
