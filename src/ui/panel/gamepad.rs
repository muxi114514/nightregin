//! 手柄操作面板：方向键 / 左摇杆在可选项之间移动，在能力值与成长权重上左右加减；
//! A 确定、按下按钮或打开初始武器选择，B 返回，LB/RB 换等级，Y 切换编辑目标。方向按住 0.35 秒后连发。

use std::time::{Duration, Instant};

use hudhook::imgui::Ui;

use crate::bridge::{SchemeSnapshot, UiBridge};
use crate::data::GameData;
use crate::stats::STAT_COUNT;
use crate::stats::growth::GROWTH_STATS;

use super::super::input_block::{pad, pad_buttons};
use super::super::sfx::{self, Cue};
use super::Panel;
use super::weapons::PICKER_COLUMNS;

const REPEAT_DELAY: Duration = Duration::from_millis(350);
const REPEAT_EVERY: Duration = Duration::from_millis(70);

/// 左栏底部的按钮：确定、撤销修改、恢复原版、本级恢复自动、清空本级
pub(super) const BUTTONS: usize = 5;

/// 手柄当前停留的位置（能力值行用 Panel::stat 记录）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Focus {
    Stats,
    Button(usize),
    /// 中栏初始武器格（0 主手、1 副手）
    Weapon(usize),
    Weight(usize),
    Copy,
    Delete,
}

pub(super) struct PadNav {
    pub(super) focus: Focus,
    /// 最近一次操作来自手柄（显示手柄焦点与按键说明）；动鼠标后清除
    pub(super) active: bool,
    /// "复制到"列表打开时的选中项，以及本帧要执行的列表操作
    pub(super) copy_cursor: Option<usize>,
    pub(super) copy_len: usize,
    pub(super) open_copy: bool,
    pub(super) copy_accept: bool,
    pub(super) copy_cancel: bool,
    /// 当前角色的初始武器格数（1 或 2，绘制时更新）
    pub(super) weapon_slots: usize,
    prev: u32,
    held_dir: u32,
    held_since: Instant,
    last_repeat: Instant,
}

impl Default for PadNav {
    fn default() -> Self {
        let now = Instant::now();
        Self {
            focus: Focus::Stats,
            active: false,
            copy_cursor: None,
            copy_len: 0,
            open_copy: false,
            copy_accept: false,
            copy_cancel: false,
            weapon_slots: 1,
            prev: 0,
            held_dir: 0,
            held_since: now,
            last_repeat: now,
        }
    }
}

impl PadNav {
    /// 本帧"触发"的按键：新按下的键，加上按住方向键时的连发
    fn fired(&mut self) -> u32 {
        let now_bits = pad_buttons();
        let mut fired = now_bits & !self.prev;
        let dir = now_bits & (pad::UP | pad::DOWN | pad::LEFT | pad::RIGHT);
        let now = Instant::now();
        if dir == 0 {
            self.held_dir = 0;
        } else if dir != self.held_dir {
            self.held_dir = dir;
            self.held_since = now;
            self.last_repeat = now;
        } else if now - self.held_since >= REPEAT_DELAY && now - self.last_repeat >= REPEAT_EVERY {
            self.last_repeat = now;
            fired |= dir;
        }
        self.prev = now_bits;
        fired
    }

    pub(super) fn hot(&self, focus: Focus) -> bool {
        self.active && self.focus == focus
    }
}

impl Panel {
    /// 每帧处理手柄输入；返回 false 表示按 B 关闭面板
    pub(super) fn gamepad(&mut self, ui: &Ui, snap: &SchemeSnapshot, data: &GameData, bridge: &UiBridge) -> bool {
        let fired = self.pad.fired();
        if fired != 0 {
            self.pad.active = true;
        } else if ui.io().mouse_delta != [0.0, 0.0] {
            self.pad.active = false;
        }
        let on = |bit: u32| fired & bit != 0;

        // 初始武器选择网格打开时只在网格内移动
        if self.picker.slot.is_some() {
            self.pad_picker(fired);
            return true;
        }
        // "复制到"列表打开时只在列表内移动
        if let Some(cursor) = self.pad.copy_cursor {
            let last = self.pad.copy_len.saturating_sub(1);
            let mut next = cursor;
            if on(pad::UP) {
                next = next.saturating_sub(1);
            }
            if on(pad::DOWN) {
                next = (next + 1).min(last);
            }
            if next != cursor {
                self.pad.copy_cursor = Some(next);
                sfx::request(Cue::Cursor);
            }
            self.pad.copy_accept = on(pad::A);
            self.pad.copy_cancel = on(pad::B);
            if self.pad.copy_cancel {
                sfx::request(Cue::Cancel);
            }
            return true;
        }

        if on(pad::B) {
            return false;
        }
        if on(pad::LB) {
            self.change_level(-1);
        }
        if on(pad::RB) {
            self.change_level(1);
        }
        if on(pad::Y) && snap.vessel.is_some() {
            let to_default = !self.target_default;
            self.switch_target(to_default, data);
        }
        if on(pad::UP) {
            self.move_focus(-1);
        }
        if on(pad::DOWN) {
            self.move_focus(1);
        }
        for (bit, delta) in [(pad::LEFT, -1), (pad::RIGHT, 1)] {
            if on(bit) {
                self.pad_horizontal(delta);
            }
        }
        if on(pad::A) {
            self.pad_accept(snap, bridge);
        }
        true
    }

    /// 网格内移动：左右逐格，上下跨行；A 选中、B 取消由绘制时执行
    fn pad_picker(&mut self, fired: u32) {
        let on = |bit: u32| fired & bit != 0;
        let (last, cursor) = (self.picker.len.saturating_sub(1), self.picker.cursor);
        let mut next = cursor;
        if on(pad::LEFT) {
            next = next.saturating_sub(1);
        }
        if on(pad::RIGHT) {
            next = (next + 1).min(last);
        }
        if on(pad::UP) {
            next = next.saturating_sub(PICKER_COLUMNS);
        }
        if on(pad::DOWN) && next + PICKER_COLUMNS <= last {
            next += PICKER_COLUMNS;
        }
        if next != cursor {
            self.picker.cursor = next;
            sfx::request(Cue::Cursor);
        }
        self.picker.accept = on(pad::A);
        self.picker.cancel = on(pad::B);
        if self.picker.cancel {
            sfx::request(Cue::Cancel);
        }
    }

    /// 上下移动：能力值 8 行 -> 左栏按钮 -> 中栏初始武器 -> 成长权重 7 项 -> 复制到 -> 删除
    fn move_focus(&mut self, step: i32) {
        use Focus::*;
        let before = (self.pad.focus, self.stat);
        self.pad.focus = match (self.pad.focus, step > 0) {
            (Stats, true) if self.stat + 1 < STAT_COUNT => {
                self.stat += 1;
                Stats
            }
            (Stats, true) => Button(0),
            (Stats, false) => {
                self.stat = self.stat.saturating_sub(1);
                Stats
            }
            (Button(_), true) => Weapon(0),
            (Button(_), false) => {
                self.stat = STAT_COUNT - 1;
                Stats
            }
            (Weapon(i), true) if i + 1 < self.pad.weapon_slots => Weapon(i + 1),
            (Weapon(_), true) => Weight(0),
            (Weapon(0), false) => Button(0),
            (Weapon(i), false) => Weapon(i - 1),
            (Weight(i), true) if i + 1 < GROWTH_STATS => Weight(i + 1),
            (Weight(_), true) => Copy,
            (Weight(0), false) => Weapon(self.pad.weapon_slots - 1),
            (Weight(i), false) => Weight(i - 1),
            (Copy, true) => Delete,
            (Copy, false) => Weight(GROWTH_STATS - 1),
            (Delete, true) => Delete,
            (Delete, false) => Copy,
        };
        self.confirm_delete &= self.pad.focus == Delete;
        if (self.pad.focus, self.stat) != before {
            sfx::request(Cue::Cursor);
        }
    }

    /// 左右：加减能力值 / 权重，或在同一排按钮间移动
    fn pad_horizontal(&mut self, delta: i32) {
        let (level, focus) = (self.level, self.pad.focus);
        match self.pad.focus {
            Focus::Stats => {
                let stat = self.stat;
                self.with_draft(|d| d.adjust(level, stat, delta));
            }
            Focus::Weight(i) => {
                let Some(w) = self.draft.as_ref().map(|d| d.weights()[i]) else { return };
                let next = (w as i32 + delta).max(0) as u32;
                self.with_draft(|d| d.set_weight(i, next));
            }
            Focus::Button(i) => self.pad.focus = Focus::Button((i as i32 + delta).clamp(0, BUTTONS as i32 - 1) as usize),
            Focus::Weapon(i) => self.pad.focus = Focus::Weapon((i as i32 + delta).clamp(0, self.pad.weapon_slots as i32 - 1) as usize),
            Focus::Copy | Focus::Delete => self.pad.focus = if delta > 0 { Focus::Delete } else { Focus::Copy },
        }
        if self.pad.focus != focus {
            sfx::request(Cue::Cursor);
        }
    }

    fn pad_accept(&mut self, snap: &SchemeSnapshot, bridge: &UiBridge) {
        sfx::request(Cue::Confirm);
        let level = self.level;
        match self.pad.focus {
            Focus::Stats | Focus::Weight(_) | Focus::Button(0) => self.save(snap, bridge),
            Focus::Button(1) => self.with_draft(|d| {
                d.revert();
                Ok(())
            }),
            Focus::Button(2) => self.with_draft(|d| {
                d.reset_all();
                Ok(())
            }),
            Focus::Button(3) => self.with_draft(|d| d.reset_level(level)),
            Focus::Button(_) => self.with_draft(|d| d.clear_level(level)),
            Focus::Weapon(i) => self.picker.open = Some(i),
            Focus::Copy => {
                self.pad.open_copy = true;
                self.pad.copy_cursor = Some(0);
            }
            Focus::Delete => self.request_delete(snap, bridge),
        }
    }
}
