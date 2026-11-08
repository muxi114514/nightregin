//! 午夜每帧任务（游戏主线程）：读位置、联机身份与遗物 → 处理开关 → 文本 / 拦准备 → 发布自己的状态 → 结算 → 发布快照给叠加层。
//! 联机身份未知（Steam 接口不可用或大厅过渡中）时只在本机生效：允许自己切换。

use std::sync::Arc;
use std::time::{Duration, Instant};

use log::{info, warn};
use nr_stat_alloc_shared::midnight::curse::{self as curse_table, CurseId, NO_CURSE};
use nr_stat_alloc_shared::midnight::wire::MemberState;
use nr_stat_alloc_shared::midnight::{Curve, MAX_LEVEL, Place};

use super::bridge::{MidnightBridge, MidnightSnapshot, Notice, RosterRow};
use super::curse::{self, CurseEngine};
use super::mode::BlockReason;
use super::net::MidnightLink;
use super::result::ResultTracker;
use super::{menu, mode, ready, texts};
use crate::config::{MidnightConfig, OffsetOverrides};
use crate::game::addresses::{Addresses, MidnightMenuAddresses};
use crate::game::menu as game_menu;
use crate::game::module::GameModule;
use crate::game::rtti;
use crate::game::player_data::PlayerDataLocator;
use crate::game::{sp_effect, world};
use crate::hotkey::HotkeyState;
use crate::rng::XorShift;

/// 遗物装备情况的重读间隔
const RELIC_POLL: Duration = Duration::from_millis(250);
/// 玩家类诅咒的检查间隔
const CURSE_WEAR_CHECK: Duration = Duration::from_millis(500);

pub struct MidnightTask {
    addrs: Addresses,
    menu_addrs: MidnightMenuAddresses,
    bridge: Arc<MidnightBridge>,
    link: Box<dyn MidnightLink + Send>,
    tracker: ResultTracker,
    curve: Curve,
    locator: PlayerDataLocator,
    key: HotkeyState,
    local_toggle: bool,
    active: bool,
    /// 上次发给队友的午夜状态（出征中保持）
    published_mode: bool,
    last_place: Option<Place>,
    relics: bool,
    relics_read: Option<Instant>,
    /// 上次的遗物检查结果（变了才写日志）
    last_report: Option<ready::RelicReport>,
    teammates_seen: usize,
    sortie: menu::SortieTracker,
    /// 上次记进日志的出击菜单对话框（诊断用，变了才写）
    logged_dialog: Option<(usize, usize)>,
    engine: CurseEngine,
    rng: XorShift,
    diag_curse: CurseId,
    /// 自己抽好的下一局诅咒（房主的这条对全队生效）
    own_curse: CurseId,
    /// 这局的诅咒：圆桌里跟目标，出征中保持
    run_curse: CurseId,
    /// 已挂到角色上的玩家类效果
    worn_effect: Option<i32>,
    worn_checked: Option<Instant>,
    hero_select_seen: bool,
    /// 这次出征确实是午夜出击（见 mode::armed）
    armed: bool,
}

/// 构造 MidnightTask 所需的游戏侧依赖
pub struct TaskParts {
    pub addrs: Addresses,
    pub menu_addrs: MidnightMenuAddresses,
    pub bridge: Arc<MidnightBridge>,
    pub link: Box<dyn MidnightLink + Send>,
    pub tracker: ResultTracker,
    pub engine: CurseEngine,
    pub diag_curse: CurseId,
}

impl MidnightTask {
    pub fn new(cfg: &MidnightConfig, offsets: &OffsetOverrides, parts: TaskParts) -> Self {
        Self {
            addrs: parts.addrs,
            menu_addrs: parts.menu_addrs,
            bridge: parts.bridge,
            link: parts.link,
            tracker: parts.tracker,
            curve: cfg.curve(),
            locator: PlayerDataLocator::new(offsets),
            key: HotkeyState::new(&[cfg.toggle_key]),
            local_toggle: false,
            active: false,
            published_mode: false,
            last_place: None,
            relics: false,
            relics_read: None,
            last_report: None,
            teammates_seen: 0,
            sortie: menu::SortieTracker::default(),
            logged_dialog: None,
            engine: parts.engine,
            rng: XorShift::from_time(),
            diag_curse: parts.diag_curse,
            own_curse: NO_CURSE,
            run_curse: NO_CURSE,
            worn_effect: None,
            worn_checked: None,
            hero_select_seen: false,
            armed: false,
        }
    }

    /// 午夜出错停用时调用：还原诅咒改过的内存数据
    pub fn release(&mut self) {
        self.engine.restore();
    }

    pub fn tick(&mut self) {
        let now = Instant::now();
        let player = world::main_player(&self.addrs);
        let place = player.and_then(world::current_map).map(|m| if world::is_expedition(m) { Place::Expedition } else { Place::Hub });
        if place.is_some() {
            if place != self.last_place {
                ready::reset_local();
            }
            self.last_place = place;
        }
        let in_hub = self.last_place == Some(Place::Hub);
        self.link.poll(now);
        let known_host = self.link.is_host();
        let is_host = known_host.unwrap_or(true);
        let host_state = self.link.host_state();
        if in_hub && self.own_curse == NO_CURSE {
            self.own_curse = curse_table::pick(self.diag_curse, self.engine.available(), self.rng.next());
            if self.own_curse != NO_CURSE {
                info!("midnight: next curse {}", self.own_curse);
            }
        }

        let deep_panel = self.sortie_menu_open() && menu::deep_panel_loaded();
        let host_ready = ready::local_ready();
        let can_toggle = is_host && !host_ready && in_hub;
        let pressed = !self.key.poll().is_empty() && deep_panel;
        if (self.bridge.take_toggle() || pressed) && can_toggle {
            self.local_toggle = !self.local_toggle;
        }
        let prev = self.active;
        // 出征中保持出击时的状态
        if in_hub {
            self.active = mode::next_active(self.local_toggle, is_host, host_ready, prev, host_state.map(|s| s.in_midnight));
        }
        if self.active != prev {
            info!("midnight: {} (host {is_host}, session known {})", if self.active { "on" } else { "off" }, known_host.is_some());
        }
        // 房主选着深夜栏才真正生效；选择保留到出击那一刻
        let effective = mode::effective(self.active, is_host, menu::deep_panel_loaded());
        texts::set_state(self.active, effective);

        if in_hub {
            self.refresh_relics(player, now);
            // 圆桌里提前改好数据，不管游戏什么时候读表都已生效；出征中不动
            self.run_curse = mode::target_curse(effective, is_host, self.own_curse, host_state);
            self.engine.sync(self.run_curse);
        }
        let states: Vec<_> = self.link.teammates().iter().map(|t| t.state).collect();
        let block = if in_hub { mode::block_reason(effective, is_host, self.relics, &states) } else { None };
        ready::set_block(block.is_some());
        // 队友已准备后才收到午夜：带遗物就取消准备，取消不了按「本局不计午夜」
        if self.active && !prev && self.relics && ready::local_ready() && !ready::cancel_ready() {
            info!("midnight: could not cancel ready, this sortie will not count");
        }
        if ready::take_blocked() {
            self.bridge.notify(if block == Some(BlockReason::TeammateUnmodded) { Notice::TeammateUnmodded } else { Notice::RelicsBlocked });
        }

        self.armed = mode::armed(in_hub, self.armed, effective, ready::departing());
        let counted = mode::counted(effective && self.armed, self.relics);
        let level = self.curve.level_of(self.tracker.progress().points).min(MAX_LEVEL);
        self.published_mode = mode::published_mode(in_hub, effective, self.published_mode);
        self.link.publish(&MemberState { in_midnight: self.published_mode, has_relics: self.relics, ready: ready::local_ready(), level: level as u8, curse: self.own_curse }, now);
        // 结算完一局才换诅咒，下一帧重抽
        if self.tracker.tick(place, counted).is_some() {
            self.own_curse = NO_CURSE;
        }
        if place == Some(Place::Expedition) {
            menu::reset_panel();
        }
        self.wear_curse(place, player, now);
        let reveal = mode::reveal(self.active, self.run_curse, self.hero_select_showing());
        self.publish(is_host, can_toggle, deep_panel, known_host.is_some(), reveal);
    }

    /// 选角界面是否正在显示（第一次识别到时写日志，便于实机确认）
    fn hero_select_showing(&mut self) -> bool {
        let showing = self.menu_addrs.hero_select.is_some_and(|vt| game_menu::showing_any(&self.addrs, &[vt]));
        if showing && !self.hero_select_seen {
            info!("midnight: hero select screen");
        }
        self.hero_select_seen = showing;
        showing
    }

    /// 玩家类诅咒：出征中定期检查，缺了补回（倒地、复活后效果可能没了）；回圆桌时摘掉
    fn wear_curse(&mut self, place: Option<Place>, player: Option<usize>, now: Instant) {
        let Some(chr) = player else { return };
        match place {
            Some(Place::Expedition) if self.active && self.armed => {
                let Some(effect) = curse::player_effect(self.run_curse) else { return };
                if self.worn_checked.is_some_and(|t| now.saturating_duration_since(t) < CURSE_WEAR_CHECK) {
                    return;
                }
                self.worn_checked = Some(now);
                if curse::effect_missing(sp_effect::list_ids(chr).as_deref(), effect) {
                    sp_effect::apply(&self.addrs, chr, effect, false);
                    if self.worn_effect != Some(effect) {
                        info!("midnight: curse effect {effect} applied");
                    }
                    self.worn_effect = Some(effect);
                }
            }
            Some(Place::Hub) => {
                if let Some(effect) = self.worn_effect.take() {
                    sp_effect::remove(&self.addrs, chr, effect);
                }
            }
            _ => {}
        }
    }

    /// 出击菜单是否开着：面板载入时记下的对话框还在显示，或当前对话框是已知的 Matching 类
    fn sortie_menu_open(&mut self) -> bool {
        let showing = game_menu::showing_any_dialog(&self.addrs);
        let event = menu::take_panel_event();
        if event && showing != self.logged_dialog {
            self.logged_dialog = showing;
            let name = showing.and_then(|(d, _)| GameModule::current().and_then(|m| rtti::object_type_name(&m, d)));
            info!("midnight: sortie dialog {showing:x?} type {name:?}");
        }
        self.sortie.update(event, showing) || menu::sortie_menu_open(&self.addrs, &self.menu_addrs)
    }

    fn refresh_relics(&mut self, player: Option<usize>, now: Instant) {
        if self.relics_read.is_some_and(|t| now.saturating_duration_since(t) < RELIC_POLL) {
            return;
        }
        self.relics_read = Some(now);
        let Some(game_data) = player.and_then(|p| self.locator.game_data(&self.addrs, p)) else { return };
        let report = ready::relic_report(game_data);
        let changed = self.last_report.as_ref() != Some(&report);
        if changed {
            info!("midnight: relics — current {:?}, heroes with relics {:?}, unreadable (hero, vessel) {:?}", report.current, report.with_relics, report.unknown);
        }
        match report.verdict() {
            Some(r) => self.relics = r,
            None if changed => warn!("midnight: could not read every hero's relics; keeping the last result ({})", self.relics),
            None => {}
        }
        self.last_report = Some(report);
    }

    fn publish(&mut self, is_host: bool, can_toggle: bool, deep_panel: bool, session_known: bool, reveal: bool) {
        let blocking = ready::blocking_supported();
        let roster: Vec<RosterRow> = self
            .link
            .teammates()
            .iter()
            .map(|t| RosterRow { slot: t.slot, name: t.name.clone(), level: t.state.map_or(0, |s| s.level as u32), status: mode::roster_status(t.state, blocking) })
            .collect();
        if roster.len() != self.teammates_seen {
            info!("midnight: {} teammate(s) seen", roster.len());
            self.teammates_seen = roster.len();
        }
        let p = self.tracker.progress();
        self.bridge.publish(MidnightSnapshot {
            active: self.active,
            is_host,
            can_toggle,
            deep_panel,
            in_expedition: self.last_place == Some(Place::Expedition),
            relics: self.relics,
            session_known,
            level: self.curve.progress(p.points),
            points: p.points,
            wins: p.wins,
            losses: p.losses,
            roster,
            curse: if self.last_place == Some(Place::Expedition) && !self.armed { NO_CURSE } else { self.run_curse },
            reveal,
        });
    }
}
