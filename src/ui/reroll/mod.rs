//! 重铸遗物面板：全屏覆盖遗物装配界面，与属性分配面板用同一套原生素材。
//! 左栏：当前角色装备的 6 件遗物（上排普通、下排深夜）、选中遗物的词条（可多选，负面词条可单独选）、价格与重铸按钮；
//! 右栏：按原版遗物筛选分组勾选抽取范围（只列出选中词条能抽到的分组）。
//! 面板只读游戏线程发布的快照；重铸请求带上面板看到的词条与价格，由游戏线程重新核对后执行。

mod effects;
mod pools;
mod relics;
pub mod text;

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use hudhook::imgui::{Condition, Key, StyleColor, StyleVar, Ui, WindowFlags};
use nr_stat_alloc_shared::relic_reroll::{Cost, RelicCatalog, Target};

use crate::bridge::{UiBridge, UiCommand};
use crate::data::RelicData;
use crate::game::currency::Currency;
use crate::relic_reroll::{EquippedRelic, Pricing, RelicView, RerollOutcome, RerollRequest, amount_of};
use crate::stats::heroes::hero_name;

use super::sfx::{self, Cue};
use super::skin::Skin;
use super::{theme, widgets};
use text::tx;

/// 两栏的横向位置与宽度（占屏幕宽度的比例），纵向与属性分配面板一致
const COLUMNS: [(f32, f32); 2] = [(0.035, 0.43), (0.49, 0.475)];
const MARGIN_X: f32 = 0.035;
const TITLE_Y: f32 = 0.035;
const BODY_Y: f32 = 0.13;
const BODY_H: f32 = 0.75;

pub struct RerollPanel {
    data: Option<(Arc<RelicData>, Pricing)>,
    view: Option<Arc<RelicView>>,
    view_seen: u64,
    outcome: Option<RerollOutcome>,
    outcome_seen: u64,
    /// 选中的遗物栏（0..6）与其中要重铸的词条
    slot: usize,
    targets: Vec<Target>,
    /// 各角色取消勾选的分组（默认全选）
    excluded: HashMap<usize, HashSet<u32>>,
    weapons_open: bool,
    /// 第一次点击重铸后等待确认的货币
    confirm: Option<Currency>,
    /// 已发出请求、等待游戏线程的结果
    pending: bool,
    /// 上次重铸新出现的词条（高亮）
    highlight: Vec<u32>,
}

impl RerollPanel {
    pub fn new(data: Option<(Arc<RelicData>, Pricing)>) -> Self {
        Self {
            data,
            view: None,
            view_seen: 0,
            outcome: None,
            outcome_seen: 0,
            slot: 0,
            targets: Vec::new(),
            excluded: HashMap::new(),
            weapons_open: false,
            confirm: None,
            pending: false,
            highlight: Vec::new(),
        }
    }

    pub fn available(&self) -> bool {
        self.data.is_some()
    }

    /// 面板打开时每帧调用：取最新的遗物快照与重铸结果
    pub fn sync(&mut self, bridge: &UiBridge) {
        if let Some(o) = bridge.reroll_outcome_if_newer(&mut self.outcome_seen) {
            self.pending = false;
            self.confirm = None;
            if o.ok {
                self.highlight = o.new_effects.clone();
                self.targets.clear();
            }
            self.outcome = Some(o);
        }
        let Some(view) = bridge.relics_if_newer(&mut self.view_seen) else { return };
        let before = self.selected().cloned();
        let hero_changed = self.view.as_ref().is_some_and(|v| v.hero != view.hero);
        self.view = Some(view);
        if hero_changed || self.selected().is_none() {
            self.slot = self.view.as_ref().and_then(|v| v.slots.iter().position(Option::is_some)).unwrap_or(0);
        }
        // 选中的遗物被换掉或词条变了（重铸成功以外的原因）时清空选择，避免按旧词条操作
        if self.selected() != before.as_ref() && !self.pending {
            self.targets.clear();
            self.confirm = None;
        }
    }

    /// 全屏绘制；返回 false 表示关闭面板
    pub fn draw(&mut self, ui: &Ui, bridge: &UiBridge, skin: &Skin) -> bool {
        let [w, h] = ui.io().display_size;
        let mut keep = true;
        let flags = WindowFlags::NO_DECORATION | WindowFlags::NO_MOVE | WindowFlags::NO_SAVED_SETTINGS;
        ui.window(tx().window).position([0.0, 0.0], Condition::Always).size([w, h], Condition::Always).flags(flags).build(|| {
            widgets::background(ui, skin, [w, h]);
            keep = self.title_bar(ui, skin, w, h);
            let (Some((data, pricing)), Some(view)) = (self.data.clone(), self.view.clone()) else {
                ui.set_cursor_pos([w * MARGIN_X, h * BODY_Y]);
                ui.text_wrapped(if self.data.is_none() { tx().unavailable } else { tx().waiting });
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
                let id = if col == 0 { "##reroll_left" } else { "##reroll_right" };
                ui.child_window(id).size([w * cw, h * BODY_H]).border(!framed).always_use_window_padding(framed).build(|| {
                    if col == 0 {
                        self.left(ui, skin, bridge, &view, &data.catalog, pricing);
                    } else {
                        self.pools(ui, skin, &view, &data.catalog);
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
        let clicked = widgets::button(ui, skin, back, [back_w, 0.0]);
        if let Some(view) = &self.view {
            ui.set_cursor_pos([w * MARGIN_X, h * TITLE_Y + ui.frame_height_with_spacing()]);
            ui.text(hero_name(view.hero));
            ui.same_line_with_spacing(0.0, fs * 2.0);
            ui.text_disabled(format!("{} {}　·　{} {}", tx().murk, view.murk, tx().sigils, view.sigils));
        }
        !clicked
    }

    /// 左栏：遗物、词条、价格与重铸按钮
    fn left(&mut self, ui: &Ui, skin: &Skin, bridge: &UiBridge, view: &RelicView, catalog: &RelicCatalog, pricing: Pricing) {
        self.relic_slots(ui, skin, view, catalog);
        ui.spacing();
        let Some(relic) = self.selected().cloned() else {
            widgets::section(ui, skin, tx().section_effects);
            ui.text_disabled(tx().no_relic);
            return;
        };
        let rerollable = catalog.rerollable(relic.relic);
        self.effect_list(ui, skin, &relic, catalog, rerollable);
        if rerollable {
            ui.spacing();
            self.cost(ui, skin, bridge, view, &relic, catalog, pricing);
        }
    }

    /// 价格与两种货币的重铸按钮：第一次点击进入确认，再点一次才发出请求
    #[allow(clippy::too_many_arguments)]
    fn cost(&mut self, ui: &Ui, skin: &Skin, bridge: &UiBridge, view: &RelicView, relic: &EquippedRelic, catalog: &RelicCatalog, pricing: Pricing) {
        widgets::section(ui, skin, tx().section_cost);
        if self.targets.is_empty() {
            ui.text_disabled(tx().no_target);
            return;
        }
        let excluded: Vec<u32> = self.excluded_of(view.hero).iter().copied().collect();
        let Some(cost) = pricing.quote(catalog, relic, &self.targets, view.hero, &excluded) else {
            ui.text_colored(theme::DECREASE, tx().no_candidates);
            return;
        };
        let width = ui.content_region_avail()[0];
        for (currency, held) in [(Currency::Murk, view.murk), (Currency::Sigils, view.sigils)] {
            let amount = amount_of(cost, currency);
            let (pay, unit) = match currency {
                Currency::Murk => (tx().pay_murk, tx().murk),
                Currency::Sigils => (tx().pay_sigils, tx().sigils),
            };
            let label = if self.pending {
                format!("{}##{unit}", tx().pending)
            } else if self.confirm == Some(currency) {
                format!("{}（{amount} {unit}）##{unit}", tx().confirm_again)
            } else {
                format!("{pay}（{amount}）##{unit}")
            };
            let affordable = held >= amount;
            let clicked = {
                let _d = ui.begin_disabled(self.pending || !affordable);
                widgets::button(ui, skin, &label, [width * 0.62, 0.0])
            };
            ui.same_line();
            ui.text_colored(if affordable { theme::DIM } else { theme::DECREASE }, format!("{} {held}", tx().held));
            if clicked {
                self.pay(bridge, view, relic, currency, cost, excluded.clone());
            }
        }
        ui.text_disabled(tx().price_rule);
    }

    fn pay(&mut self, bridge: &UiBridge, view: &RelicView, relic: &EquippedRelic, currency: Currency, cost: Cost, excluded: Vec<u32>) {
        if self.confirm != Some(currency) {
            self.confirm = Some(currency);
            return;
        }
        self.confirm = None;
        self.pending = true;
        self.highlight.clear();
        bridge.send(UiCommand::Reroll(RerollRequest {
            hero: view.hero,
            slot: self.slot,
            handle: relic.handle,
            effects: relic.effects,
            targets: self.targets.clone(),
            excluded,
            currency,
            amount: amount_of(cost, currency),
        }));
    }

    fn footer(&self, ui: &Ui) {
        match &self.outcome {
            Some(o) => ui.text_colored(if o.ok { theme::OK } else { theme::DECREASE }, &o.text),
            None => ui.text(tx().prompt),
        }
    }

    fn selected(&self) -> Option<&EquippedRelic> {
        self.view.as_ref()?.slots.get(self.slot)?.as_ref()
    }

    fn excluded_of(&mut self, hero: usize) -> &mut HashSet<u32> {
        self.excluded.entry(hero).or_default()
    }

    /// 换遗物或改选择时清掉待确认状态与上次结果的高亮
    fn touch(&mut self) {
        self.confirm = None;
        self.highlight.clear();
        sfx::request(Cue::Cursor);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use hudhook::imgui::Context;
    use nr_stat_alloc_shared::relic_reroll::{DEFAULT_PRICES, Effect, Group, GroupKind, RelicEffects, RelicRow};

    use super::*;

    fn data() -> Arc<RelicData> {
        let relic = |deep, tables, curse_tables| RelicRow { deep, color: 1, icon_id: 19100, tables, curse_tables };
        let relics = HashMap::from([(100, relic(false, [100, 100, -1], [-1; 3])), (2000000, relic(true, [2000000, -1, -1], [3000000, -1, -1]))]);
        let entries = [
            (100, 7000000, 50),
            (100, 7000001, 50),
            (100, 7080000, 20),
            (100, 6500000, 10),
            (2000000, 6001400, 100),
            (2000000, 6002600, 100),
            (3000000, 6820000, 100),
            (3000000, 6830000, 100),
        ];
        let effect = |group, compat| Effect { group, compat, sort_key: compat, text_id: compat, passive: [-1; 3] };
        let effects = HashMap::from([
            (7000000, effect(30000, 7000000)),
            (7000001, effect(30000, 7000000)),
            (7080000, effect(20000, 7080000)),
            (6500000, effect(10000, 900)),
            (6001400, effect(30020, 6001400)),
            (6002600, effect(30040, 6002600)),
            (6820000, effect(40010, 6820000)),
            (6830000, effect(40000, 6830000)),
        ]);
        let group = |id, kind, text_id| (id, Group { id, kind, text_id, order: 0 });
        let groups = BTreeMap::from([
            group(30000, GroupKind::General, 136110),
            group(30020, GroupKind::General, 136111),
            group(30040, GroupKind::General, 136113),
            group(20000, GroupKind::Weapon, 60010),
            group(10000, GroupKind::Hero(1), 288050),
            group(40000, GroupKind::Debuff, 136124),
            group(40010, GroupKind::Debuff, 136125),
        ]);
        Arc::new(RelicData { catalog: RelicCatalog::from_parts(relics, entries, effects, groups, false), shop_prices: DEFAULT_PRICES })
    }

    /// 无渲染器的 imgui 帧里实际画几帧：选中普通 / 深夜遗物的词条、展开武器大组、进入确认状态都不能 panic
    #[test]
    fn panel_draws_without_panicking() {
        let bridge = UiBridge::default();
        let standard = EquippedRelic { handle: 0xC000_0001, relic: 100, effects: RelicEffects { positive: [Some(7000000), Some(7080000), None], curse: [None; 3] } };
        let deep = EquippedRelic { handle: 0xC000_0002, relic: 2000000, effects: RelicEffects { positive: [Some(6001400), None, None], curse: [Some(6820000), None, None] } };
        bridge.publish_relics(RelicView { hero: 0, slots: [Some(standard), None, None, Some(deep), None, None], murk: 5000, sigils: 3 });
        let mut panel = RerollPanel::new(Some((data(), Pricing { prices: DEFAULT_PRICES, exponent: 0.5 })));
        let _imgui = crate::ui::imgui_test_lock();
        let mut ctx = Context::create();
        ctx.set_ini_filename(None);
        ctx.io_mut().display_size = [1920.0, 1080.0];
        ctx.fonts().build_rgba32_texture();
        let skin = Skin::default();
        for frame in 0..5 {
            panel.sync(&bridge);
            match frame {
                1 => panel.targets = vec![Target::Positive(0), Target::Positive(1)],
                2 => {
                    panel.weapons_open = true;
                    panel.confirm = Some(Currency::Murk);
                }
                3 => {
                    panel.slot = 3;
                    panel.targets = vec![Target::Positive(0), Target::Curse(0)];
                    panel.excluded_of(0).insert(40000);
                }
                _ => {}
            }
            let ui = ctx.new_frame();
            assert!(panel.draw(ui, &bridge, &skin));
            ctx.render();
        }
        assert_eq!(panel.selected().map(|r| r.relic), Some(2000000));
    }
}
