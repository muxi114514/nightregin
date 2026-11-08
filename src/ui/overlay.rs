//! 叠加层渲染循环：遗物装配界面打开时在右上角画"属性分配""重铸遗物"按钮，点开后全屏显示对应面板（此时按钮隐藏）；
//! 出征中游戏线程打开词条继承面板时全屏显示它；持有蓝10 时装备界面右上角画「强化武器」「继承词条」按钮。
//! 面板打开时拦截键鼠（窗口消息 + DirectInput + 鼠标轮询），鼠标停在叠加层上时也拦截，避免操作穿透到下面的游戏界面。

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;
use std::sync::Arc;

use hudhook::imgui::{Condition, Context, Io, Ui};
use hudhook::{ImguiRenderLoop, MessageFilter, RenderContext};
use log::{error, info, warn};

use crate::bridge::{OrbRefreshView, QuickForgeAction, UiBridge};
use crate::config::UiConfig;
use crate::data::{GameData, StartingData};
use crate::midnight::MidnightBridge;
use crate::starting_weapons::pick;
use crate::stats::rules::Limits;

use super::PanelData;
use super::inherit::InheritPanel;
use super::midnight::MidnightUi;
use super::panel::Panel;
use super::reroll::{self, RerollPanel};
use super::sfx::{self, Cue};
use super::skin::Skin;
use super::input_block::{pad, pad_buttons};
use super::{fonts, input_block, text, theme, widgets};

/// 1080p 下的字号
const BASE_FONT_PX: f32 = 23.0;
/// 三选一界面「潜在能力」标题右边的位置（占 16:9 界面区域的比例，游戏界面按 16:9 居中；1080p 下约 (283, 145)，与标题垂直居中）
const ORB_REFRESH_POS: [f32; 2] = [0.1474, 0.1343];
const GAME_UI_ASPECT: f32 = 16.0 / 9.0;

/// 当前全屏显示的面板
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen {
    None,
    Stats,
    Reroll,
    Inherit,
    /// 午夜主界面
    Midnight,
}

pub struct Overlay {
    bridge: Arc<UiBridge>,
    data: Option<Arc<GameData>>,
    /// 初始武器数据（面板中栏的武器选择）
    starting: Option<Arc<StartingData>>,
    cfg: UiConfig,
    glyphs: &'static [u32],
    /// 字体图集已上传后释放 ImGui 保留的字体文件副本（约 20 MB）
    font_input_cleared: bool,
    screen: Screen,
    panel: Panel,
    reroll: RerollPanel,
    inherit: InheritPanel,
    midnight: Option<MidnightUi>,
    /// 遗物图标编号（初始化时随武器图标一起上传）
    relic_icons: Vec<u16>,
    /// 原生界面素材所在目录（DLL 同目录 skin\）
    skin_dir: PathBuf,
    skin: Skin,
    /// 手柄开关面板的按键组合与上一帧的手柄状态
    pad_toggle: u32,
    pad_prev: u32,
    /// 诊断：本次打开继承面板后叠加层是否收到过鼠标移动 / 点击（各记一次日志）
    inherit_mouse_seen: [bool; 2],
    failed: bool,
}

impl Overlay {
    pub fn new(bridge: Arc<UiBridge>, midnight: Option<Arc<MidnightBridge>>, panels: PanelData, cfg: UiConfig, limits: Limits, glyphs: &'static [u32], skin_dir: PathBuf) -> Self {
        let PanelData { game: data, starting, relics, inherit } = panels;
        let pad_toggle = pad::parse_combo(&cfg.pad_toggle).unwrap_or_else(|| {
            warn!("ui pad_toggle \"{}\" not recognized, using L3", cfg.pad_toggle);
            pad::L3
        });
        let mut relic_icons: Vec<u16> = relics.as_ref().map_or_else(Vec::new, |(d, _)| d.catalog.icon_ids().filter_map(|i| u16::try_from(i).ok()).collect());
        relic_icons.sort_unstable();
        relic_icons.dedup();
        Self {
            bridge,
            data,
            starting,
            cfg,
            glyphs,
            font_input_cleared: false,
            screen: Screen::None,
            panel: Panel::new(limits),
            reroll: RerollPanel::new(relics),
            inherit: InheritPanel::new(inherit),
            midnight: midnight.map(|b| MidnightUi::new(b, glyphs)),
            relic_icons,
            skin_dir,
            skin: Skin::default(),
            pad_toggle,
            pad_prev: 0,
            inherit_mouse_seen: [false; 2],
            failed: false,
        }
    }

    fn draw(&mut self, ui: &Ui) {
        if self.screen == Screen::Inherit {
            self.note_inherit_mouse(ui);
            self.inherit.sync(&self.bridge);
            if !self.inherit.draw(ui, &self.bridge, &self.skin) {
                self.bridge.close_inherit();
                self.screen = Screen::None;
                sfx::request(Cue::Cancel);
            }
            return;
        }
        if self.screen == Screen::None && self.bridge.quick_forge_shown() {
            self.quick_forge_buttons(ui);
        }
        if self.screen == Screen::None
            && let Some(view) = self.bridge.orb_refresh()
        {
            self.orb_refresh_button(ui, view);
        }
        if matches!(self.screen, Screen::None | Screen::Midnight)
            && let Some(m) = self.midnight.as_mut()
        {
            m.draw(ui, &self.skin);
            self.screen = if m.home_open() { Screen::Midnight } else { Screen::None };
        }
        if self.screen == Screen::Midnight || !self.bridge.relic_menu_open() {
            return;
        }
        let keep = match self.screen {
            Screen::None => {
                self.buttons(ui);
                return;
            }
            Screen::Stats => {
                if let Some(data) = self.data.as_deref() {
                    self.panel.sync(&self.bridge, data);
                }
                self.panel.draw(ui, &self.bridge, self.data.as_deref(), self.starting.as_deref(), &self.skin)
            }
            Screen::Reroll => {
                self.reroll.sync(&self.bridge);
                self.reroll.draw(ui, &self.bridge, &self.skin)
            }
            Screen::Inherit | Screen::Midnight => true,
        };
        if !keep {
            self.screen = Screen::None;
            sfx::request(Cue::Cancel);
        }
    }

    /// 右上角的入口按钮（重铸数据不可用时只有属性分配）
    fn buttons(&mut self, ui: &Ui) {
        let [w, h] = ui.io().display_size;
        let reroll = self.reroll.available();
        ui.window(text::tx().open_button)
            .position([w * 0.98, h * 0.03], Condition::Always)
            .position_pivot([1.0, 0.0])
            .no_decoration()
            .always_auto_resize(true)
            .movable(false)
            .focus_on_appearing(false)
            .build(|| {
                let fs = ui.current_font_size();
                let width = ui.calc_text_size(text::tx().title)[0].max(ui.calc_text_size(reroll::text::tx().title)[0]) + fs * 3.0;
                if widgets::button(ui, &self.skin, text::tx().open_button, [width, 0.0]) {
                    self.screen = Screen::Stats;
                }
                if reroll {
                    ui.same_line();
                    if widgets::button(ui, &self.skin, reroll::text::tx().open_button, [width, 0.0]) {
                        self.screen = Screen::Reroll;
                    }
                }
            });
    }

    /// 蓝10：装备界面右上角、卢恩数下方的入口（继承面板不可用时只有强化）；点击交给游戏线程执行
    fn quick_forge_buttons(&self, ui: &Ui) {
        let [w, h] = ui.io().display_size;
        let tx = text::tx();
        let inherit = self.inherit.available();
        ui.window("##nr_quick_forge")
            .position([w * 0.98, h * 0.09], Condition::Always)
            .position_pivot([1.0, 0.0])
            .no_decoration()
            .always_auto_resize(true)
            .movable(false)
            .focus_on_appearing(false)
            .build(|| {
                let fs = ui.current_font_size();
                let width = ui.calc_text_size(tx.title)[0] + fs * 3.0;
                if widgets::button(ui, &self.skin, tx.quick_reinforce, [width, 0.0]) {
                    self.bridge.click_quick_forge(QuickForgeAction::Reinforce);
                }
                if inherit {
                    ui.same_line();
                    if widgets::button(ui, &self.skin, tx.quick_inherit, [width, 0.0]) {
                        self.bridge.click_quick_forge(QuickForgeAction::Inherit);
                    }
                }
            });
    }

    /// 三选一界面的刷新按钮：次数用完或卢恩不够时变灰；点击交给游戏线程执行
    fn orb_refresh_button(&self, ui: &Ui, view: OrbRefreshView) {
        let [w, h] = ui.io().display_size;
        let ui_w = w.min(h * GAME_UI_ASPECT);
        let pos = [(w - ui_w) * 0.5 + ui_w * ORB_REFRESH_POS[0], h * ORB_REFRESH_POS[1]];
        ui.window("##nr_orb_refresh")
            .position(pos, Condition::Always)
            .no_decoration()
            .draw_background(false)
            .always_auto_resize(true)
            .movable(false)
            .focus_on_appearing(false)
            .build(|| {
                let caption = text::orb_refresh_label(view.remaining, view.cost);
                let width = ui.calc_text_size(&caption)[0] + ui.current_font_size() * 3.0;
                let clicked = {
                    let _d = ui.begin_disabled(view.remaining == 0 || !view.affordable);
                    widgets::button(ui, &self.skin, &format!("{caption}##nr_orb_refresh_go"), [width, 0.0])
                };
                if clicked {
                    self.bridge.click_orb_refresh();
                }
            });
    }

    fn note_inherit_mouse(&mut self, ui: &Ui) {
        let io = ui.io();
        let events = [io.mouse_delta != [0.0, 0.0], io.mouse_down.iter().any(|&d| d)];
        for (i, (&event, seen)) in events.iter().zip(self.inherit_mouse_seen.iter_mut()).enumerate() {
            if event && !*seen {
                *seen = true;
                info!("inherit panel: overlay receives mouse {}", if i == 0 { "movement" } else { "click" });
            }
        }
    }

    /// 继承面板的开关跟随游戏线程（锻造台 / 测试热键打开，离开出征地图关闭）；显示期间按需加载武器图标。
    /// 返回继承面板是否正在显示（此时不处理遗物装配界面的面板热键）
    fn sync_inherit(&mut self, render_context: &mut dyn RenderContext) -> bool {
        let open = self.bridge.inherit_open() && self.inherit.available();
        if open && self.screen != Screen::Inherit {
            self.screen = Screen::Inherit;
            self.inherit.reset();
            self.inherit_mouse_seen = [false; 2];
            sfx::request(Cue::Confirm);
        } else if !open && self.screen == Screen::Inherit {
            self.screen = Screen::None;
        }
        if self.screen != Screen::Inherit {
            return false;
        }
        self.skin.ensure_icons(render_context, &self.inherit.icon_ids());
        true
    }
}

impl ImguiRenderLoop for Overlay {
    /// 渲染管线（重新）创建时调用，每次都是新的 ImGui 上下文
    fn initialize<'a>(&'a mut self, ctx: &mut Context, render_context: &'a mut dyn RenderContext) {
        let [w, h] = ctx.io().display_size;
        let scale = (h.max(720.0) / 1080.0) * self.cfg.scale;
        fonts::add_ui_font(ctx, &self.cfg.font, BASE_FONT_PX * scale, self.glyphs);
        theme::apply(ctx.style_mut(), scale);
        let mut icons: Vec<u16> = self.starting.as_deref().map_or_else(Vec::new, |s| pick::pool(s).iter().filter_map(|p| s.weapons.icon(p.weapon)).collect());
        icons.extend(&self.relic_icons);
        self.skin = Skin::load(&self.skin_dir, render_context, &icons);
        self.font_input_cleared = false;
        info!("overlay initialized at {w:.0}x{h:.0}, scale {scale:.2}");
    }

    fn before_render<'a>(&'a mut self, ctx: &mut Context, render_context: &'a mut dyn RenderContext) {
        if !self.font_input_cleared {
            ctx.fonts().clear_input_data();
            self.font_input_cleared = true;
        }
        // 键盘热键由游戏线程检测；手柄组合键在这里检测（按下的那一帧）
        let bits = pad_buttons();
        let combo = self.pad_toggle;
        let pad_pressed = bits & combo == combo && self.pad_prev & combo != combo;
        self.pad_prev = bits;
        if let Some(m) = self.midnight.as_mut() {
            m.poll_pad(bits);
        }
        if self.sync_inherit(render_context) {
            return;
        }
        let relic_open = self.bridge.relic_menu_open();
        // 热键 / 手柄组合键开关属性分配面板；任一面板开着时则关闭它
        if (self.bridge.take_toggle() || pad_pressed) && relic_open {
            self.screen = if self.screen == Screen::None { Screen::Stats } else { Screen::None };
            sfx::request(if self.screen == Screen::None { Cue::Cancel } else { Cue::Confirm });
        }
        // 遗物装配界面关闭时属性 / 重铸面板一并关闭（午夜主界面不在遗物界面里）
        if !relic_open && matches!(self.screen, Screen::Stats | Screen::Reroll) {
            self.screen = Screen::None;
        }
    }

    fn render(&mut self, ui: &mut Ui) {
        if self.failed {
            input_block::set_blocked(false);
            return;
        }
        // panic 不能穿过钩子回到游戏（会直接崩溃），出错后本次会话停用叠加层
        if catch_unwind(AssertUnwindSafe(|| self.draw(ui))).is_err() {
            self.failed = true;
            self.screen = Screen::None;
            error!("overlay panicked; overlay disabled for this session");
        }
        sfx::end_frame(ui, self.cfg.sound && !self.failed);
        input_block::set_blocked(!self.failed && (self.screen != Screen::None || ui.io().want_capture_mouse));
    }

    fn message_filter(&self, io: &Io) -> MessageFilter {
        if self.failed {
            MessageFilter::empty()
        } else if self.screen != Screen::None {
            MessageFilter::InputAll
        } else if io.want_capture_mouse {
            MessageFilter::InputMouse
        } else {
            MessageFilter::empty()
        }
    }
}
