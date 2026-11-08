//! 属性分配面板（仿法环升级界面，套用黑夜君临原生菜单素材）：全屏覆盖遗物装配界面，与"预设"子页一样。
//! 左栏等级与能力值，中栏基本能力 / 初始武器 / 满级预览，右栏成长总览、成长权重与方案管理。
//! 编辑在本地草稿上进行，确定时把方案交给游戏线程写文件并重载。

mod attributes;
mod gamepad;
mod overview;
mod vitals;
mod weapons;

use std::sync::Arc;

use hudhook::imgui::{Condition, Key, StyleColor, StyleVar, Ui, WindowFlags, WindowFocusedFlags};

use crate::bridge::{LiveStats, SchemeCommand, SchemeSnapshot, UiBridge, UiCommand, UiMessage};
use crate::data::{GameData, StartingData};
use crate::editor::Draft;
use crate::stats::growth::MAX_LEVEL;
use crate::stats::heroes::hero_name;
use crate::stats::rules::{Limits, SchemeSpec};

use super::sfx::{self, Cue};
use super::skin::{Skin, Sprite};
use gamepad::PadNav;
use weapons::WeaponPicker;
use super::{text, theme, widgets};

/// 三栏的横向位置与宽度（占屏幕宽度的比例）
const COLUMNS: [(f32, f32); 3] = [(0.035, 0.30), (0.35, 0.29), (0.655, 0.31)];
const MARGIN_X: f32 = 0.035;
const TITLE_Y: f32 = 0.035;
const BODY_Y: f32 = 0.13;
const BODY_H: f32 = 0.75;

pub struct Panel {
    limits: Limits,
    snapshot: Option<Arc<SchemeSnapshot>>,
    snapshot_seen: u64,
    message_seen: u64,
    live: Option<LiveStats>,
    /// true：编辑角色默认方案；false：编辑当前器皿的专属方案
    target_default: bool,
    draft: Option<Draft>,
    /// 正在编辑的等级与选中的属性
    level: u32,
    stat: usize,
    /// 上一次操作被拒绝的原因
    error: Option<String>,
    message: Option<UiMessage>,
    confirm_delete: bool,
    pad: PadNav,
    picker: WeaponPicker,
}

impl Panel {
    pub fn new(limits: Limits) -> Self {
        Self {
            limits,
            snapshot: None,
            snapshot_seen: 0,
            message_seen: 0,
            live: None,
            target_default: false,
            draft: None,
            level: 1,
            stat: 0,
            error: None,
            message: None,
            confirm_delete: false,
            pad: PadNav::default(),
            picker: WeaponPicker::default(),
        }
    }

    /// 面板打开时每帧调用：取最新的方案快照、实时属性与命令结果
    pub fn sync(&mut self, bridge: &UiBridge, data: &GameData) {
        self.live = bridge.live();
        if let Some(m) = bridge.message_if_newer(&mut self.message_seen) {
            self.message = Some(m);
            self.error = None;
        }
        let Some(snap) = bridge.snapshot_if_newer(&mut self.snapshot_seen) else { return };
        let switched = self.snapshot.as_ref().is_none_or(|old| old.hero != snap.hero || old.vessel != snap.vessel);
        let dirty = self.draft.as_ref().is_some_and(Draft::dirty);
        if switched {
            if dirty {
                self.message = Some(UiMessage { text: text::tx().discarded.into(), ok: false });
            }
            self.level = snap.level.clamp(1, MAX_LEVEL);
            self.target_default = false;
            self.snapshot = Some(snap);
            self.rebuild_draft(data);
            return;
        }
        // 同一目标：草稿没改过，或刚保存成功（新快照与草稿一致）时，以快照为准重建
        let saved_now = self.draft.as_ref().is_some_and(|d| Some(d.spec()) == self.target_spec_of(&snap).as_ref());
        self.snapshot = Some(snap);
        if !dirty || saved_now {
            self.rebuild_draft(data);
        }
    }

    /// 全屏绘制；返回 false 表示用户关闭了面板
    pub fn draw(&mut self, ui: &Ui, bridge: &UiBridge, data: Option<&GameData>, starting: Option<&StartingData>, skin: &Skin) -> bool {
        let [w, h] = ui.io().display_size;
        let mut keep = true;
        let flags = WindowFlags::NO_DECORATION | WindowFlags::NO_MOVE | WindowFlags::NO_SAVED_SETTINGS;
        ui.window(text::tx().panel_title).position([0.0, 0.0], Condition::Always).size([w, h], Condition::Always).flags(flags).build(|| {
            widgets::background(ui, skin, [w, h]);
            keep = self.title_bar(ui, skin, data, w, h);
            let (Some(data), Some(snap)) = (data, self.snapshot.clone()) else {
                ui.set_cursor_pos([w * MARGIN_X, h * BODY_Y]);
                ui.text_wrapped(if data.is_none() { text::tx().no_data } else { text::tx().waiting });
                return;
            };
            if self.draft.is_none() {
                return;
            }
            // 手柄输入不依赖窗口焦点，放在画各栏之前，本帧的焦点与列表操作即可生效
            keep &= self.gamepad(ui, &snap, data, bridge);
            let ids = ["##nr_left", "##nr_mid", "##nr_right"];
            let origin = ui.window_pos();
            let fs = ui.current_font_size();
            for (col, ((x, cw), id)) in COLUMNS.iter().zip(ids).enumerate() {
                let p0 = [origin[0] + w * x, origin[1] + h * BODY_Y];
                let p1 = [p0[0] + w * cw, p0[1] + h * BODY_H];
                // 有原生边框时由父窗口画框，子窗口透明并向内缩进
                let framed = widgets::panel(ui, skin, p0, p1);
                let _bg = framed.then(|| ui.push_style_color(StyleColor::ChildBg, [0.0; 4]));
                let _pad = framed.then(|| ui.push_style_var(StyleVar::WindowPadding([fs * 1.5, fs * 1.2])));
                ui.set_cursor_pos([w * x, h * BODY_Y]);
                ui.child_window(id).size([w * cw, h * BODY_H]).border(!framed).always_use_window_padding(framed).build(|| match col {
                    0 => self.attributes(ui, skin, &snap, bridge),
                    1 => self.vitals(ui, skin, &snap, data, starting),
                    _ => self.overview(ui, skin, &snap, data, bridge),
                });
            }
            ui.set_cursor_pos([w * MARGIN_X, h * (BODY_Y + BODY_H + 0.02)]);
            self.footer(ui, skin);
            if ui.is_window_focused_with_flags(WindowFocusedFlags::ROOT_AND_CHILD_WINDOWS) {
                self.keyboard(ui, &snap, bridge);
            }
        });
        keep && !ui.is_key_pressed(Key::Escape)
    }

    /// 顶部：标题、角色与器皿、编辑目标、保存状态、返回按钮
    fn title_bar(&mut self, ui: &Ui, skin: &Skin, data: Option<&GameData>, w: f32, h: f32) -> bool {
        let fs = ui.current_font_size();
        let origin = ui.window_pos();
        let glow0 = [origin[0] + w * MARGIN_X - fs, origin[1] + h * TITLE_Y - fs * 0.6];
        widgets::title_glow(ui, skin, glow0, [glow0[0] + fs * 22.0, glow0[1] + fs * 3.6]);
        ui.set_cursor_pos([w * MARGIN_X, h * TITLE_Y]);
        ui.text_colored(theme::ACCENT, text::tx().title);
        let back_w = ui.calc_text_size(text::tx().back)[0] + fs * 4.0;
        ui.same_line_with_pos(w * (1.0 - MARGIN_X) - back_w);
        let back = widgets::button(ui, skin, text::tx().back, [back_w, 0.0]);
        let (Some(data), Some(snap)) = (data, self.snapshot.clone()) else { return !back };

        ui.set_cursor_pos([w * MARGIN_X, h * TITLE_Y + ui.frame_height_with_spacing()]);
        let vessel = snap.vessel.and_then(|v| data.vessel(v)).map_or(text::tx().unknown_vessel, |v| v.display_name());
        ui.text(format!("{}{}{vessel}", hero_name(snap.hero), text::tx().sep));
        ui.same_line();
        ui.text_disabled(text::tx().editing);
        ui.same_line();
        let no_vessel = snap.vessel.is_none();
        {
            let _d = ui.begin_disabled(no_vessel);
            if widgets::diamond(ui, skin, text::tx().target_vessel, !self.target_default && !no_vessel) && self.target_default {
                self.switch_target(false, data);
            }
        }
        ui.same_line();
        if widgets::diamond(ui, skin, text::tx().target_default, self.target_default || no_vessel) && !self.target_default {
            self.switch_target(true, data);
        }
        ui.same_line();
        if self.target(&snap).is_some() && snap.vessel_spec.is_none() {
            ui.text_colored(theme::WARN, text::tx().no_vessel_scheme);
        } else if self.draft.as_ref().is_some_and(Draft::dirty) {
            ui.text_colored(theme::WARN, text::tx().unsaved);
        } else {
            ui.text_disabled(text::tx().saved);
        }
        ui.same_line();
        ui.text_disabled(if snap.vessel_spec.is_some() {
            text::tx().active_vessel
        } else if snap.default_spec.is_some() {
            text::tx().active_default
        } else {
            text::tx().active_vanilla
        });
        if !data.components_ready {
            ui.same_line();
            ui.text_colored(theme::DECREASE, text::tx().no_components);
        }
        !back
    }

    /// 底部：操作结果或提示语，以及按键说明（有按键图标时用游戏的按键图标）
    fn footer(&self, ui: &Ui, skin: &Skin) {
        match (&self.error, &self.message) {
            (Some(e), _) => ui.text_colored(theme::DECREASE, e),
            (None, Some(m)) => ui.text_colored(if m.ok { theme::OK } else { theme::DECREASE }, &m.text),
            (None, None) => ui.text(text::tx().prompt),
        }
        if self.pad.active {
            ui.text_disabled(text::tx().pad_hint);
            return;
        }
        if skin.get(Sprite::KeyUpDown).is_none() {
            ui.text_disabled(text::tx().keys_hint);
            return;
        }
        let hints: [(&[Sprite], &str); 5] = [
            (&[Sprite::KeyUpDown], text::tx().hint_select),
            (&[Sprite::KeyLeftRight], text::tx().hint_adjust),
            (&[Sprite::KeyQ, Sprite::KeyE], text::tx().hint_level),
            (&[Sprite::KeyEnter], text::tx().hint_confirm),
            (&[Sprite::KeyEsc], text::tx().hint_back),
        ];
        for (i, (keys, label)) in hints.iter().enumerate() {
            if i > 0 {
                ui.same_line_with_spacing(0.0, ui.current_font_size() * 1.5);
            }
            widgets::key_hint(ui, skin, keys, label);
        }
    }

    fn keyboard(&mut self, ui: &Ui, snap: &SchemeSnapshot, bridge: &UiBridge) {
        if ui.is_key_pressed(Key::UpArrow) {
            self.select_stat((self.stat + 7) % 8);
        }
        if ui.is_key_pressed(Key::DownArrow) {
            self.select_stat((self.stat + 1) % 8);
        }
        for (key, delta) in [(Key::LeftArrow, -1), (Key::RightArrow, 1)] {
            if ui.is_key_pressed(key) {
                let (level, stat) = (self.level, self.stat);
                self.with_draft(|d| d.adjust(level, stat, delta));
            }
        }
        if ui.is_key_pressed(Key::PageUp) || ui.is_key_pressed(Key::Q) {
            self.change_level(-1);
        }
        if ui.is_key_pressed(Key::PageDown) || ui.is_key_pressed(Key::E) {
            self.change_level(1);
        }
        if ui.is_key_pressed(Key::Enter) || ui.is_key_pressed(Key::KeypadEnter) {
            self.save(snap, bridge);
        }
    }

    /// 删除当前编辑目标的方案：第一次只进入确认状态，再按一次才真正删除
    fn request_delete(&mut self, snap: &SchemeSnapshot, bridge: &UiBridge) {
        if self.confirm_delete {
            bridge.send(UiCommand::Scheme(SchemeCommand::Delete { hero: snap.hero, vessel: self.target(snap) }));
        }
        self.confirm_delete = !self.confirm_delete;
        sfx::request(Cue::Confirm);
    }

    /// 改变选择的操作只在真的变了时播放光标音（到头了再按不响，与游戏一致）
    fn select_stat(&mut self, stat: usize) {
        if stat != self.stat {
            self.stat = stat;
            sfx::request(Cue::Cursor);
        }
    }

    fn change_level(&mut self, delta: i32) {
        let level = (self.level as i32 + delta).clamp(1, MAX_LEVEL as i32) as u32;
        if level != self.level {
            self.level = level;
            sfx::request(Cue::Cursor);
        }
        self.error = None;
    }

    fn save(&mut self, snap: &SchemeSnapshot, bridge: &UiBridge) {
        sfx::request(Cue::Confirm);
        if let Some(d) = &self.draft {
            bridge.send(UiCommand::Scheme(SchemeCommand::Save { hero: snap.hero, vessel: self.target(snap), spec: d.spec().clone() }));
        }
    }

    /// 对草稿做一次操作，记录被拒绝的原因；成功时播放光标音（由按钮触发时会被确定音取代）
    fn with_draft(&mut self, f: impl FnOnce(&mut Draft) -> Result<(), String>) {
        if let Some(d) = self.draft.as_mut() {
            self.error = f(d).err();
            self.confirm_delete = false;
            if self.error.is_none() {
                sfx::request(Cue::Cursor);
            }
        }
    }

    fn switch_target(&mut self, to_default: bool, data: &GameData) {
        if self.draft.as_ref().is_some_and(Draft::dirty) {
            self.message = Some(UiMessage { text: text::tx().target_switched.into(), ok: false });
        }
        self.target_default = to_default;
        self.rebuild_draft(data);
        sfx::request(Cue::Cursor);
    }

    /// 保存目标：None 为角色默认方案
    fn target(&self, snap: &SchemeSnapshot) -> Option<u32> {
        if self.target_default { None } else { snap.vessel }
    }

    /// 当前编辑目标已保存的方案
    fn target_spec_of(&self, snap: &SchemeSnapshot) -> Option<SchemeSpec> {
        match self.target(snap) {
            Some(_) => snap.vessel_spec.clone(),
            None => snap.default_spec.clone(),
        }
    }

    /// 器皿还没有专属方案时，以角色默认方案为起点
    fn rebuild_draft(&mut self, data: &GameData) {
        let Some(snap) = self.snapshot.clone() else { return };
        let spec = self.target_spec_of(&snap).or_else(|| snap.default_spec.clone()).unwrap_or_default();
        let mut draft = Draft::new(data.growth[snap.hero].clone(), self.limits, spec);
        if self.target(&snap).is_some() && snap.vessel_spec.is_none() {
            draft.mark_unsaved();
        }
        self.draft = Some(draft);
        self.error = None;
        self.confirm_delete = false;
        self.picker = WeaponPicker::default();
    }
}

/// 在当前行、相对行首 x0 的 right 处右对齐输出文字
fn text_right(ui: &Ui, x0: f32, right: f32, color: [f32; 4], s: &str) {
    let width = ui.calc_text_size(s)[0];
    ui.same_line_with_pos(x0 + right - width);
    ui.text_colored(color, s);
}

/// 数值相对参照值的颜色：高于为蓝、低于为红
fn compare_color(value: u32, reference: u32) -> [f32; 4] {
    match value.cmp(&reference) {
        std::cmp::Ordering::Greater => theme::INCREASE,
        std::cmp::Ordering::Less => theme::DECREASE,
        std::cmp::Ordering::Equal => theme::TEXT,
    }
}
