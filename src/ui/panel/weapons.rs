//! 中栏"初始武器"：当前编辑方案的主手（原本有副手的角色另有副手）图标与名字；点一格打开选择网格，
//! 列出各角色的初始武器，不合初始装备规则的变暗不可选。选择写进草稿，与属性一起"确定"保存（按角色 + 器皿）。

use hudhook::imgui::Ui;

use crate::bridge::SchemeSnapshot;
use crate::data::{StartingData, weapon_name};
use crate::starting_weapons::pick::{self, PoolItem};
use crate::stats::rules::WeaponPick;

use super::super::sfx::{self, Cue};
use super::super::skin::{Skin, Texture};
use super::super::{text, theme, widgets};
use super::Panel;
use super::gamepad::Focus;

/// 选择网格每行几格（手柄上下移动按它换行）
pub(super) const PICKER_COLUMNS: usize = 5;
const POPUP: &str = "##nr_weapon_picker";

/// 选择网格的状态：手柄在 gamepad.rs 里移动光标、置位 accept / cancel，绘制时在这里执行
#[derive(Default)]
pub(super) struct WeaponPicker {
    /// 本帧要打开的格子（0 主手、1 副手）
    pub(super) open: Option<usize>,
    /// 正在为哪一格选择
    pub(super) slot: Option<usize>,
    pub(super) cursor: usize,
    pub(super) len: usize,
    pub(super) accept: bool,
    pub(super) cancel: bool,
}

/// 网格里的一项：某把初始武器，或"无"（只在副手格出现）
#[derive(Clone, Copy)]
enum Choice {
    Weapon(PoolItem),
    Empty,
}

impl Panel {
    pub(super) fn weapons(&mut self, ui: &Ui, skin: &Skin, snap: &SchemeSnapshot, starting: Option<&StartingData>) {
        widgets::section(ui, skin, text::tx().section_weapons);
        let (Some(data), Some(draft)) = (starting, self.draft.as_ref()) else {
            ui.text_disabled(text::tx().weapons_unavailable);
            return;
        };
        let pick = draft.weapons().clone();
        // 方案里手写了认不出的写法时按原版显示（日志里有说明）
        let current = pick::resolve(data, snap.hero, &pick).or_else(|_| pick::resolve(data, snap.hero, &WeaponPick::default())).ok();
        self.pad.weapon_slots = if pick::has_off_hand(data, snap.hero) { 2 } else { 1 };
        if matches!(self.pad.focus, Focus::Weapon(i) if i >= self.pad.weapon_slots) {
            self.pad.focus = Focus::Weapon(0);
        }
        let labels = [text::tx().main_hand, text::tx().off_hand];
        for (slot, label) in labels.iter().enumerate().take(self.pad.weapon_slots) {
            let weapon = current.and_then(|(main, off)| if slot == 0 { Some(main) } else { off });
            if self.weapon_slot(ui, skin, data, slot, label, weapon) {
                self.picker.open = Some(slot);
                sfx::request(Cue::Confirm);
            }
        }
        ui.text_disabled(text::tx().weapons_note);
        self.weapon_picker(ui, skin, data, snap.hero, &pick);
    }

    /// 一格：图标 + "主手 / 副手" + 武器名；返回是否被点击
    fn weapon_slot(&self, ui: &Ui, skin: &Skin, data: &StartingData, slot: usize, label: &str, weapon: Option<u32>) -> bool {
        let fs = ui.current_font_size();
        let size = fs * 3.6;
        let p0 = ui.cursor_screen_pos();
        let clicked = ui.invisible_button(format!("##nr_weapon_slot{slot}"), [ui.content_region_avail()[0], size]);
        sfx::track_hover(ui);
        let hot = ui.is_item_hovered() || self.pad.hot(Focus::Weapon(slot));
        widgets::weapon_icon(ui, icon_of(data, skin, weapon), p0, size, hot, false);
        let dl = ui.get_window_draw_list();
        let x = p0[0] + size + fs * 0.7;
        dl.add_text([x, p0[1] + size * 0.5 - fs * 1.15], theme::DIM, label);
        let name = weapon.map_or_else(|| text::tx().no_weapon.to_string(), weapon_name);
        dl.add_text([x, p0[1] + size * 0.5 + fs * 0.1], if hot { theme::ACCENT } else { theme::TEXT }, name);
        clicked
    }

    fn weapon_picker(&mut self, ui: &Ui, skin: &Skin, data: &StartingData, hero: usize, pick: &WeaponPick) {
        if let Some(slot) = self.picker.open.take() {
            self.picker.slot = Some(slot);
            self.picker.cursor = 0;
            ui.open_popup(POPUP);
        }
        let Some(slot) = self.picker.slot else { return };
        let Some(_popup) = ui.begin_popup(POPUP) else {
            // 点到弹窗外面关闭了
            self.picker.slot = None;
            return;
        };
        ui.text_colored(theme::ACCENT, text::tx().picker_title);
        let mut choices: Vec<Choice> = pick::pool(data).into_iter().map(Choice::Weapon).collect();
        if slot == 1 {
            choices.push(Choice::Empty);
        }
        self.picker.len = choices.len();
        self.picker.cursor = self.picker.cursor.min(choices.len() - 1);
        let fs = ui.current_font_size();
        let size = fs * 3.2;
        let mut chosen = None;
        let mut focused_name = None;
        for (i, &choice) in choices.iter().enumerate() {
            if i % PICKER_COLUMNS != 0 {
                ui.same_line();
            }
            let candidate = with_choice(pick, hero, slot, choice);
            let allowed = pick::validate(data, hero, &candidate).is_ok();
            let p0 = ui.cursor_screen_pos();
            let clicked = ui.invisible_button(format!("##nr_weapon_choice{i}"), [size, size]);
            sfx::track_hover(ui);
            let cursor = self.pad.active && self.picker.cursor == i;
            let hovered = ui.is_item_hovered();
            let weapon = match choice {
                Choice::Weapon(p) => Some(p.weapon),
                Choice::Empty => None,
            };
            widgets::weapon_icon(ui, icon_of(data, skin, weapon), p0, size, hovered || cursor, !allowed);
            let name = weapon.map_or_else(|| text::tx().no_weapon.to_string(), weapon_name);
            if weapon.is_none() {
                let w = ui.calc_text_size(&name)[0];
                ui.get_window_draw_list().add_text([p0[0] + (size - w) * 0.5, p0[1] + (size - fs) * 0.5], theme::TEXT, &name);
            }
            if hovered || cursor {
                focused_name = Some(name);
            }
            if allowed && (clicked || (cursor && self.picker.accept)) {
                chosen = Some(candidate);
            }
        }
        // 光标 / 悬停项的名字（手柄看不到悬停提示）
        ui.text(focused_name.unwrap_or_default());
        let picked = chosen.is_some();
        if let Some(candidate) = chosen {
            sfx::request(Cue::Confirm);
            if let Some(d) = self.draft.as_mut() {
                d.set_weapons(candidate);
            }
        }
        if picked || self.picker.cancel {
            ui.close_current_popup();
            self.picker.slot = None;
        }
        self.picker.accept = false;
        self.picker.cancel = false;
    }
}

/// 在当前选择上替换一格；选回该角色自己的默认武器时写 None（保持原版）
fn with_choice(pick: &WeaponPick, hero: usize, slot: usize, choice: Choice) -> WeaponPick {
    let off_hand = slot == 1;
    let token = match choice {
        Choice::Weapon(p) if p.owner == hero && p.off_hand == off_hand => None,
        Choice::Weapon(p) => Some(p.token()),
        Choice::Empty => Some("无".to_string()),
    };
    let mut out = pick.clone();
    if off_hand {
        out.off = token;
    } else {
        out.main = token;
    }
    out
}

fn icon_of(data: &StartingData, skin: &Skin, weapon: Option<u32>) -> Option<Texture> {
    weapon.and_then(|w| data.weapons.icon(w)).and_then(|i| skin.icon(i))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn choosing_own_default_clears_the_hand() {
        let item = |owner, off_hand, weapon| Choice::Weapon(PoolItem { owner, off_hand, weapon });
        let start = WeaponPick { main: Some("执行者".into()), off: None };
        // 追踪者主手选回自己的大剑 -> 原版
        assert_eq!(with_choice(&start, 0, 0, item(0, false, 3750000)), WeaponPick::default());
        // 追踪者副手换成守护者大盾
        assert_eq!(with_choice(&start, 0, 1, item(1, true, 32750000)).off.as_deref(), Some("守护者副手"));
        assert_eq!(with_choice(&start, 0, 1, Choice::Empty).off.as_deref(), Some("无"));
    }
}
