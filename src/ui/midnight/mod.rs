//! 午夜叠加层：深夜面板下的开关与入口、午夜主界面、右下队伍侧栏、拦准备提示。只读 MidnightBridge 快照、只发请求。

mod curse;
mod home;
mod roster;
pub mod text;
mod toggle;

use std::sync::Arc;
use std::time::{Duration, Instant};

use hudhook::imgui::{Condition, Ui};

use super::input_block::pad;
use super::skin::Skin;
use super::{sfx, theme};
use crate::midnight::MidnightBridge;
use crate::midnight::bridge::Notice;
use sfx::Cue;
use toggle::MidnightAction;

/// 拦准备提示显示多久
const NOTICE_FOR: Duration = Duration::from_secs(4);
const PAD_TOGGLE: u32 = pad::LB | pad::Y;
const PAD_HOME: u32 = pad::LB | pad::X;
const PAD_BACK: u32 = pad::B;

pub struct MidnightUi {
    bridge: Arc<MidnightBridge>,
    home_open: bool,
    notice: Option<(Notice, Instant)>,
    pad_prev: u32,
    /// 字体图集收录的字（判断队友名字能否显示）
    glyphs: &'static [u32],
}

impl MidnightUi {
    pub fn new(bridge: Arc<MidnightBridge>, glyphs: &'static [u32]) -> Self {
        Self { bridge, home_open: false, notice: None, pad_prev: 0, glyphs }
    }

    pub fn home_open(&self) -> bool {
        self.home_open
    }

    /// 手柄组合键（按下那一帧）：深夜面板显示时 LB+Y 切换、LB+X 打开主界面
    pub fn poll_pad(&mut self, bits: u32) {
        let pressed = |combo: u32| bits & combo == combo && self.pad_prev & combo != combo;
        let (toggle, home, back) = (pressed(PAD_TOGGLE), pressed(PAD_HOME), pressed(PAD_BACK));
        self.pad_prev = bits;
        if back && self.home_open {
            self.home_open = false;
            sfx::request(Cue::Cancel);
            return;
        }
        if !self.bridge.snapshot().deep_panel {
            return;
        }
        if toggle {
            self.bridge.request_toggle();
        }
        if home && !self.home_open {
            self.home_open = true;
            sfx::request(Cue::Confirm);
        }
    }

    pub fn draw(&mut self, ui: &Ui, skin: &Skin) {
        let s = self.bridge.snapshot();
        if !s.deep_panel && self.home_open {
            self.home_open = false;
        }
        if self.home_open {
            if !home::draw(ui, skin, &s) {
                self.home_open = false;
                sfx::request(Cue::Cancel);
            }
        } else if s.deep_panel {
            match toggle::draw(ui, skin, &s) {
                Some(MidnightAction::Toggle) => self.bridge.request_toggle(),
                Some(MidnightAction::OpenHome) => self.home_open = true,
                None => {}
            }
        }
        if s.active && !s.roster.is_empty() {
            roster::draw(ui, &s, self.glyphs);
        }
        if s.reveal {
            curse::draw_banner(ui, s.curse);
        }
        if s.in_expedition && s.active {
            curse::draw_badge(ui, s.curse);
        }
        self.draw_notice(ui);
    }

    fn draw_notice(&mut self, ui: &Ui) {
        if let Some(n) = self.bridge.take_notice() {
            self.notice = Some((n, Instant::now()));
        }
        let Some((n, at)) = self.notice else { return };
        if at.elapsed() > NOTICE_FOR {
            self.notice = None;
            return;
        }
        let [w, h] = ui.io().display_size;
        ui.window("##nr_midnight_notice")
            .position([w * 0.5, h * 0.18], Condition::Always)
            .position_pivot([0.5, 0.5])
            .no_decoration()
            .always_auto_resize(true)
            .no_inputs()
            .build(|| ui.text_colored(theme::WARN, notice_text(n)));
    }
}

fn notice_text(n: Notice) -> &'static str {
    match n {
        Notice::RelicsBlocked => text::tx().relics_blocked,
        Notice::TeammateUnmodded => text::tx().teammate_unmodded,
    }
}

#[cfg(test)]
mod tests {
    use hudhook::imgui::Context;
    use nr_stat_alloc_shared::midnight::{Curve, LevelProgress};

    use super::*;
    use crate::midnight::bridge::{MidnightSnapshot, RosterRow};
    use crate::midnight::mode::RosterStatus;

    fn frames(mut f: impl FnMut(&hudhook::imgui::Ui)) {
        let _imgui = crate::ui::imgui_test_lock();
        let mut ctx = Context::create();
        ctx.set_ini_filename(None);
        ctx.io_mut().display_size = [1920.0, 1080.0];
        ctx.fonts().build_rgba32_texture();
        for _ in 0..3 {
            let ui = ctx.new_frame();
            f(ui);
            ctx.render();
        }
    }

    fn snap(points: u64) -> MidnightSnapshot {
        MidnightSnapshot { deep_panel: true, is_host: true, can_toggle: true, level: Curve::default().progress(points), points, ..Default::default() }
    }

    #[test]
    fn toggle_draws_without_renderer() {
        let skin = Skin::default();
        frames(|ui| {
            assert_eq!(toggle::draw(ui, &skin, &snap(0)), None);
            let guest = MidnightSnapshot { is_host: false, can_toggle: false, ..snap(0) };
            assert_eq!(toggle::draw(ui, &skin, &guest), None);
        });
    }

    #[test]
    fn home_draws_locked_barren_below_level_6() {
        let skin = Skin::default();
        assert!(home::barren_locked(&snap(600)));
        assert!(!home::barren_locked(&snap(730)));
        frames(|ui| {
            assert!(home::draw(ui, &skin, &snap(600)));
            assert!(home::draw(ui, &skin, &MidnightSnapshot { level: LevelProgress { level: 100, into_level: 0, needed: 0 }, ..snap(57600) }));
        });
    }

    /// 提示文字要在本模块的文字表里（字体图集只收录 GLYPH_SOURCES 列出的文件）
    #[test]
    fn notice_uses_ui_text() {
        assert_eq!(notice_text(crate::midnight::bridge::Notice::RelicsBlocked), text::tx().relics_blocked);
        assert!(include_str!("text.rs").contains(text::tx().relics_blocked));
        assert_eq!(notice_text(crate::midnight::bridge::Notice::TeammateUnmodded), text::tx().teammate_unmodded);
        assert!(include_str!("text.rs").contains(text::tx().teammate_unmodded));
    }

    #[test]
    fn every_pool_curse_has_text() {
        for c in &nr_stat_alloc_shared::midnight::curse::POOL {
            let t = curse::curse_text(c.id).unwrap_or_else(|| panic!("curse {} has no text", c.id));
            assert!(!t.name.is_empty() && !t.hint.is_empty(), "curse {}", c.id);
        }
    }

    #[test]
    fn unknown_curse_has_no_text() {
        assert!(curse::curse_text(0).is_none());
        assert!(curse::curse_text(200).is_none());
    }

    #[test]
    fn banner_and_badge_draw_without_renderer() {
        frames(|ui| {
            curse::draw_banner(ui, 1);
            curse::draw_badge(ui, 14);
            curse::draw_banner(ui, 200);
            curse::draw_badge(ui, 0);
        });
    }

    /// 诅咒文字要进字体图集，否则显示成问号
    #[test]
    fn curse_text_is_in_glyph_source() {
        assert!(crate::ui::GLYPH_SOURCES.iter().any(|s| s.contains("受伤时累积出血")));
    }

    #[test]
    fn roster_draws_six_rows() {
        let statuses = [RosterStatus::Ready, RosterStatus::NotReady, RosterStatus::Relics, RosterStatus::Uncounted, RosterStatus::Unmodded, RosterStatus::Ready];
        let roster = statuses.iter().enumerate().map(|(i, &status)| RosterRow { slot: i as u8, name: Some(format!("p{i}")), level: i as u32, status }).collect();
        let s = MidnightSnapshot { active: true, roster, ..snap(0) };
        let glyphs = crate::ui::fonts::glyph_ranges(&[]);
        frames(|ui| {
            roster::draw(ui, &s, glyphs);
            roster::draw(ui, &MidnightSnapshot { in_expedition: true, ..s.clone() }, glyphs);
        });
    }

    #[test]
    fn roster_name_falls_back_when_unrenderable() {
        let glyphs = crate::ui::fonts::glyph_ranges(&[]);
        let row = |name: Option<&str>| RosterRow { slot: 2, name: name.map(str::to_string), level: 1, status: RosterStatus::Ready };
        assert_eq!(roster::shown_name(&row(Some("Melina")), glyphs, "队友"), "Melina");
        assert_eq!(roster::shown_name(&row(Some("梅琳娜")), glyphs, "队友"), "队友 3");
        assert_eq!(roster::shown_name(&row(None), glyphs, "队友"), "队友 3");
    }

    /// 回退文字要能显示（字体图集只收录 GLYPH_SOURCES 列出的文件）
    #[test]
    fn teammate_label_is_in_ui_text() {
        assert!(include_str!("text.rs").contains(text::tx().teammate));
    }
}
