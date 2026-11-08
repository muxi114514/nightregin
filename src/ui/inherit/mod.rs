//! 词条继承面板：出征中在锻造台打开（游戏线程置开），全屏显示，与其它面板用同一套原生素材。
//! 左栏：右手、左手各 3 个武器槽，依次点选来源（继承后消失）与目标；右栏：继承前后的词条预览与确认按钮（点两次才执行）。
//! 面板只读游戏线程发布的武器槽快照；请求带上面板看到的武器与词条，由游戏线程重新核对后执行。

mod preview;
mod rows;
pub mod text;

use std::sync::Arc;

use hudhook::imgui::{Condition, Key, StyleColor, StyleVar, Ui, WindowFlags};

use crate::bridge::UiBridge;
use crate::data::InheritData;
use crate::game::inventory::WEAPON_SLOT_COUNT;
use crate::passive_inherit::{InheritOutcome, InheritRequest, InheritView, InheritWeapon};

use super::input_block::{pad, pad_buttons};
use super::sfx::{self, Cue};
use super::skin::Skin;
use super::{theme, widgets};
use text::tx;

/// 两栏的横向位置与宽度（占屏幕宽度的比例），纵向与其它面板一致
const COLUMNS: [(f32, f32); 2] = [(0.035, 0.43), (0.49, 0.475)];
const MARGIN_X: f32 = 0.035;
const TITLE_Y: f32 = 0.035;
const BODY_Y: f32 = 0.13;
const BODY_H: f32 = 0.75;
/// 显示顺序：右手 1~3、左手 1~3（装备槽 0 左手 1、1 右手 1、2 左手 2…）
const HANDS: [[usize; 3]; 2] = [[1, 3, 5], [0, 2, 4]];
/// 手柄焦点：0..6 为按显示顺序的武器行，6 为确认按钮
const CONFIRM_FOCUS: usize = WEAPON_SLOT_COUNT;

/// 选中的武器：装备槽 + 选中时看到的武器（快照变了就取消选择）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Choice {
    slot: usize,
    weapon: InheritWeapon,
}

#[derive(Default)]
struct PadFocus {
    focus: usize,
    /// 最近一次操作来自手柄（显示焦点）；动鼠标后清除
    active: bool,
    prev: u32,
}

pub struct InheritPanel {
    data: Option<Arc<InheritData>>,
    view: Option<Arc<InheritView>>,
    view_seen: u64,
    outcome: Option<InheritOutcome>,
    outcome_seen: u64,
    source: Option<Choice>,
    target: Option<Choice>,
    /// 第一次点击继承后等待再次确认
    confirm: bool,
    /// 已发出请求、等待游戏线程的结果
    pending: bool,
    pad: PadFocus,
}

impl InheritPanel {
    pub fn new(data: Option<Arc<InheritData>>) -> Self {
        Self { data, view: None, view_seen: 0, outcome: None, outcome_seen: 0, source: None, target: None, confirm: false, pending: false, pad: PadFocus::default() }
    }

    pub fn available(&self) -> bool {
        self.data.is_some()
    }

    /// 每次打开时清空上一次的选择与结果
    pub fn reset(&mut self) {
        self.source = None;
        self.target = None;
        self.confirm = false;
        self.pending = false;
        self.outcome = None;
        self.pad = PadFocus { prev: pad_buttons(), ..PadFocus::default() };
    }

    /// 当前武器的图标编号（叠加层据此按需加载）
    pub fn icon_ids(&self) -> Vec<u16> {
        let (Some(data), Some(view)) = (&self.data, &self.view) else { return Vec::new() };
        view.slots.iter().flatten().filter_map(|w| data.icon(w.item_id)).collect()
    }

    /// 每帧调用：取最新的武器槽快照与继承结果
    pub fn sync(&mut self, bridge: &UiBridge) {
        if let Some(o) = bridge.inherit_outcome_if_newer(&mut self.outcome_seen) {
            self.pending = false;
            self.confirm = false;
            if o.ok {
                self.source = None;
                self.target = None;
            }
            self.outcome = Some(o);
        }
        let Some(view) = bridge.inherit_view_if_newer(&mut self.view_seen) else { return };
        // 选中的武器被换掉或词条变了时取消选择，避免按旧状态操作
        if !self.pending {
            let still = |c: &Choice| view.slots[c.slot] == Some(c.weapon);
            if !self.source.as_ref().is_some_and(still) {
                self.source = None;
                self.target = None;
            }
            if !self.target.as_ref().is_some_and(still) {
                self.target = None;
            }
        }
        self.view = Some(view);
    }

    /// 全屏绘制；返回 false 表示关闭面板
    pub fn draw(&mut self, ui: &Ui, bridge: &UiBridge, skin: &Skin) -> bool {
        let mut keep = self.gamepad(ui, bridge);
        let [w, h] = ui.io().display_size;
        let flags = WindowFlags::NO_DECORATION | WindowFlags::NO_MOVE | WindowFlags::NO_SAVED_SETTINGS;
        ui.window(tx().window).position([0.0, 0.0], Condition::Always).size([w, h], Condition::Always).flags(flags).build(|| {
            widgets::background(ui, skin, [w, h]);
            keep &= self.title_bar(ui, skin, w, h);
            let (Some(data), Some(view)) = (self.data.clone(), self.view.clone()) else {
                ui.set_cursor_pos([w * MARGIN_X, h * BODY_Y]);
                ui.text_disabled(tx().waiting);
                return;
            };
            let origin = ui.window_pos();
            let fs = ui.current_font_size();
            for (col, (x, cw)) in COLUMNS.iter().enumerate() {
                let p0 = [origin[0] + w * x, origin[1] + h * BODY_Y];
                let p1 = [p0[0] + w * cw, p0[1] + h * BODY_H];
                let framed = widgets::panel(ui, skin, p0, p1);
                let _bg = framed.then(|| ui.push_style_color(StyleColor::ChildBg, [0.0; 4]));
                let _pad = framed.then(|| ui.push_style_var(StyleVar::WindowPadding([fs * 1.5, fs * 1.2])));
                ui.set_cursor_pos([w * x, h * BODY_Y]);
                let id = if col == 0 { "##inherit_left" } else { "##inherit_right" };
                ui.child_window(id).size([w * cw, h * BODY_H]).border(!framed).always_use_window_padding(framed).build(|| {
                    if col == 0 {
                        self.weapon_list(ui, skin, &view, &data);
                    } else {
                        self.preview(ui, skin, bridge, &data);
                    }
                });
            }
            ui.set_cursor_pos([w * MARGIN_X, h * (BODY_Y + BODY_H + 0.02)]);
            self.footer(ui);
        });
        keep && !ui.is_key_pressed(Key::Escape)
    }

    fn title_bar(&self, ui: &Ui, skin: &Skin, w: f32, h: f32) -> bool {
        let fs = ui.current_font_size();
        let origin = ui.window_pos();
        let glow0 = [origin[0] + w * MARGIN_X - fs, origin[1] + h * TITLE_Y - fs * 0.6];
        widgets::title_glow(ui, skin, glow0, [glow0[0] + fs * 22.0, glow0[1] + fs * 3.6]);
        ui.set_cursor_pos([w * MARGIN_X, h * TITLE_Y]);
        ui.text_colored(theme::ACCENT, tx().title);
        let back = super::text::tx().back;
        let back_w = ui.calc_text_size(back)[0] + fs * 4.0;
        ui.same_line_with_pos(w * (1.0 - MARGIN_X) - back_w);
        !widgets::button(ui, skin, back, [back_w, 0.0])
    }

    fn footer(&self, ui: &Ui) {
        match &self.outcome {
            Some(o) => ui.text_colored(if o.ok { theme::OK } else { theme::DECREASE }, &o.text),
            None => ui.text(if self.pad.active { tx().pad_prompt } else { tx().prompt }),
        }
    }

    /// 点选武器：先来源后目标；再点已选的取消
    fn activate(&mut self, slot: usize) {
        let Some(weapon) = self.view.as_ref().and_then(|v| v.slots[slot]) else { return };
        if self.pending {
            return;
        }
        match (self.source, self.target) {
            (Some(s), _) if s.slot == slot => {
                self.source = None;
                self.target = None;
            }
            (Some(_), Some(t)) if t.slot == slot => self.target = None,
            (None, _) => self.source = Some(Choice { slot, weapon }),
            (Some(_), _) if self.can_target(slot) => {
                self.target = Some(Choice { slot, weapon });
                self.pad.focus = CONFIRM_FOCUS;
            }
            _ => return,
        }
        self.confirm = false;
        self.outcome = None;
        sfx::request(Cue::Confirm);
    }

    /// 该槽的武器能否作为当前来源的目标（档位规则在游戏线程执行时还会再核对一次）
    fn can_target(&self, slot: usize) -> bool {
        let (Some(data), Some(source)) = (&self.data, self.source) else { return false };
        let target = self.view.as_ref().and_then(|v| v.slots[slot]);
        source.slot != slot && target.is_some_and(|t| data.rarities.check(source.weapon.item_id, t.item_id).is_ok())
    }

    /// 继承按钮：第一次点击进入确认，再点一次才发出请求
    fn press_inherit(&mut self, bridge: &UiBridge) {
        let (Some(source), Some(target)) = (self.source, self.target) else { return };
        if self.pending {
            return;
        }
        if !self.confirm {
            self.confirm = true;
            sfx::request(Cue::Confirm);
            return;
        }
        self.confirm = false;
        self.pending = true;
        self.outcome = None;
        bridge.send_inherit(InheritRequest { source: source.weapon.pick, target: target.weapon.pick });
        sfx::request(Cue::Confirm);
    }

    /// B：先退确认，再退目标、来源，都没有时关闭面板
    fn back(&mut self) -> bool {
        if self.confirm {
            self.confirm = false;
        } else if self.target.is_some() {
            self.target = None;
        } else if self.source.is_some() {
            self.source = None;
        } else {
            return false;
        }
        sfx::request(Cue::Cancel);
        true
    }

    /// 手柄：方向键上下在 6 个武器行与确认按钮间移动，A 选择 / 确认，B 退一步；返回 false 表示关闭面板
    fn gamepad(&mut self, ui: &Ui, bridge: &UiBridge) -> bool {
        let bits = pad_buttons();
        let fired = bits & !self.pad.prev;
        self.pad.prev = bits;
        if fired != 0 {
            self.pad.active = true;
        } else if ui.io().mouse_delta != [0.0, 0.0] {
            self.pad.active = false;
        }
        let on = |bit: u32| fired & bit != 0;
        let before = self.pad.focus;
        if on(pad::UP) {
            self.pad.focus = self.pad.focus.saturating_sub(1);
        }
        if on(pad::DOWN) {
            self.pad.focus = (self.pad.focus + 1).min(CONFIRM_FOCUS);
        }
        if self.pad.focus != before {
            sfx::request(Cue::Cursor);
        }
        if on(pad::A) {
            match focus_slot(self.pad.focus) {
                Some(slot) => self.activate(slot),
                None => self.press_inherit(bridge),
            }
        }
        !on(pad::B) || self.back()
    }
}

/// 手柄焦点对应的装备槽（确认按钮为 None）
fn focus_slot(focus: usize) -> Option<usize> {
    HANDS.as_flattened().get(focus).copied()
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use hudhook::imgui::Context;
    use nr_stat_alloc_shared::passive_inherit::Rarities;
    use nr_stat_alloc_shared::weapons::WeaponInfo;

    use super::*;
    use crate::passive_inherit::WeaponPick;

    fn weapon(index: u32, item_id: u32, passives: [i32; 3]) -> Option<InheritWeapon> {
        Some(InheritWeapon { pick: WeaponPick { index, handle: 0x8080_0000 + index, passives }, item_id })
    }

    fn data() -> Arc<InheritData> {
        let rarities = Rarities::from_rows([(9040000, 13, 2), (3140000, 5, 2), (30750000, 65, 0), (3750000, 5, 0)]);
        let weapons = WeaponInfo::from_rows([(9040000, 13, 10813), (30750000, 65, 12021)]);
        Arc::new(InheritData::from_parts(rarities, weapons, HashMap::from([(8670002, 310000)])))
    }

    /// 无渲染器的 imgui 帧里实际画几帧：选来源、选目标、进入确认、发出请求都不能 panic，且请求带上选中的两把武器
    #[test]
    fn panel_selects_confirms_and_sends() {
        let bridge = UiBridge::default();
        let slots = [None, weapon(396, 9040000, [8880200, 8670102, 8813100]), None, weapon(1947, 3140000, [8670002, -1, -1]), weapon(397, 30750000, [-1; 3]), None];
        bridge.publish_inherit_view(InheritView { slots });
        let mut panel = InheritPanel::new(Some(data()));
        panel.reset();
        let _imgui = crate::ui::imgui_test_lock();
        let mut ctx = Context::create();
        ctx.set_ini_filename(None);
        ctx.io_mut().display_size = [1920.0, 1080.0];
        ctx.fonts().build_rgba32_texture();
        let skin = Skin::default();
        for frame in 0..6 {
            panel.sync(&bridge);
            match frame {
                1 => panel.activate(3),
                2 => {
                    // 紫给白可以；白给紫不行
                    assert!(panel.can_target(4) && panel.can_target(1) && !panel.can_target(3));
                    panel.activate(1);
                }
                3 => panel.press_inherit(&bridge),
                4 => panel.press_inherit(&bridge),
                _ => {}
            }
            let ui = ctx.new_frame();
            assert!(panel.draw(ui, &bridge, &skin));
            ctx.render();
        }
        let req = bridge.take_inherit_request().expect("request sent");
        assert_eq!((req.source.index, req.target.index), (1947, 396));
        assert!(panel.pending);
        // 白色不能当紫色的来源
        panel.source = None;
        panel.target = None;
        panel.pending = false;
        panel.activate(4);
        assert!(!panel.can_target(1) && !panel.can_target(4));
    }

    #[test]
    fn focus_order_is_right_hand_then_left_then_confirm() {
        assert_eq!((0..=CONFIRM_FOCUS).map(focus_slot).collect::<Vec<_>>(), [Some(1), Some(3), Some(5), Some(0), Some(2), Some(4), None]);
    }
}
