//! 每帧任务的业务入口：热键、方案 / 测试差值、组件对账与续期、满血恢复、按方案同步初始武器，以及遗物重铸。
//! 只在游戏主线程运行，状态全部归本结构独占，无需加锁。

mod book;
mod desired;
mod perf;
mod relic_sync;
mod restore;
mod ui_sync;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use log::{error, info, warn};

use crate::bridge::{LiveStats, UiBridge};
use crate::config::Config;
use crate::data::GameData;
use crate::diag::sound_trace::SoundTrace;
use crate::diag::{self, CalibrationState, DiagContext};
use crate::game::addresses::Addresses;
use crate::game::module::GameModule;
use crate::game::player_data::{PlayerDataLocator, PlayerSnapshot};
use crate::game::{equip_data, menu, sp_effect, world};
use crate::hotkey::HotkeyState;
use crate::relic_reroll::{RelicReroll, RelicView};
use crate::starting_weapons::LoadoutApplier;
use crate::stats::components::{compose, decompose};
use crate::stats::reconcile::Reconciler;
use crate::stats::rules::Limits;

use book::SchemeBook;
use desired::{Inputs, Status};
use perf::FrameMonitor;
use restore::{RestoreTracker, SpawnGate};
use ui_sync::SnapshotKey;

/// 单次对账最多增删的组件数，避免一帧内调用过多原生函数
const MAX_OPS_PER_TICK: usize = 32;
/// 同一组件连续补挂失败的上限，超过后停止重试并报错（多半是 regulation 没生效）
const MAX_ADD_FAILURES: u32 = 3;

pub struct App {
    cfg: Config,
    dir: PathBuf,
    module: GameModule,
    addrs: Addresses,
    data: Option<Arc<GameData>>,
    book: SchemeBook,
    hotkeys: HotkeyState,
    locator: PlayerDataLocator,
    reconciler: Reconciler,
    restore: RestoreTracker,
    spawn_gate: SpawnGate,
    enabled: bool,
    status: Status,
    interval: Duration,
    last_reconcile: Instant,
    lease_refresh: Duration,
    last_refresh: Instant,
    last_player: usize,
    pending_adds: Vec<i32>,
    add_failures: HashMap<i32, u32>,
    calibration: CalibrationState,
    /// 遗物装配界面是否正在显示（叠加层按钮据此显示 / 隐藏）
    relic_menu_open: bool,
    bridge: Arc<UiBridge>,
    perf: FrameMonitor,
    /// 方案簿每次重载加 1，用于判断是否需要重新发布快照
    book_generation: u64,
    last_snapshot: Option<SnapshotKey>,
    last_live: Option<LiveStats>,
    sound_trace: Option<SoundTrace>,
    /// 方案里的初始武器（数据不可用时为 None）
    loadouts: Option<LoadoutApplier>,
    /// 遗物重铸（数据或地址不可用、配置关闭时为 None）
    reroll: Option<RelicReroll>,
    last_relic_publish: Instant,
    last_relic_view: Option<RelicView>,
}

impl App {
    pub fn new(
        cfg: Config,
        dir: PathBuf,
        module: GameModule,
        addrs: Addresses,
        bridge: Arc<UiBridge>,
        data: Option<Arc<GameData>>,
        loadouts: Option<LoadoutApplier>,
        reroll: Option<RelicReroll>,
    ) -> Self {
        if let Some(d) = &data {
            if !d.components_ready {
                error!("regulation has no NRStatAlloc components (SpEffect 9960001+): install pkg/regulation.bin or patch your regulation with nr_stat_alloc_patcher; schemes will not take effect");
            } else if cfg.lease_refresh_ms as f32 / 1000.0 > d.lease_seconds / 2.0 {
                warn!("lease_refresh_ms {} is too close to component lifetime {}s", cfg.lease_refresh_ms, d.lease_seconds);
            }
        }
        let limits = Limits { min: cfg.rules.min, cap: cfg.rules.cap };
        let book = data.as_ref().map(|d| SchemeBook::load(&dir, d, limits)).unwrap_or_else(SchemeBook::empty);
        let keys = [cfg.hotkeys.panel, cfg.hotkeys.reload, cfg.hotkeys.calibrate, cfg.hotkeys.dump, cfg.hotkeys.toggle];
        info!("mode {:?}, cap {} / min {}", cfg.mode, cfg.rules.cap, cfg.rules.min);
        Self {
            hotkeys: HotkeyState::new(&keys),
            locator: PlayerDataLocator::new(&cfg.offsets),
            reconciler: Reconciler::default(),
            restore: RestoreTracker::default(),
            spawn_gate: SpawnGate::default(),
            enabled: true,
            status: Status::Disabled,
            interval: Duration::from_millis(cfg.reconcile_interval_ms),
            last_reconcile: Instant::now(),
            lease_refresh: Duration::from_millis(cfg.lease_refresh_ms),
            last_refresh: Instant::now(),
            last_player: 0,
            pending_adds: Vec::new(),
            add_failures: HashMap::new(),
            calibration: CalibrationState::default(),
            relic_menu_open: false,
            bridge,
            perf: FrameMonitor::default(),
            book_generation: 0,
            last_snapshot: None,
            last_live: None,
            sound_trace: cfg.diag.sound_trace.then(|| SoundTrace::new(&module)),
            loadouts,
            reroll,
            last_relic_publish: Instant::now(),
            last_relic_view: None,
            data,
            book,
            cfg,
            dir,
            module,
            addrs,
        }
    }

    pub fn tick(&mut self) {
        let started = Instant::now();
        self.perf.frame_start(started);
        for vk in self.hotkeys.poll() {
            let k = &self.cfg.hotkeys;
            if vk == k.dump {
                let ctx = DiagContext {
                    module: &self.module,
                    addrs: &self.addrs,
                    data: self.data.as_deref(),
                    desired: self.reconciler.desired(),
                    status: &self.status.describe(self.data.as_deref()),
                };
                diag::dump(&ctx, &mut self.locator);
            } else if vk == k.calibrate {
                diag::calibrate(&self.addrs, &mut self.locator, &mut self.calibration);
            } else if vk == k.toggle {
                self.enabled = !self.enabled;
                info!("F10: {}", if self.enabled { "enabled" } else { "disabled" });
            } else if vk == k.reload {
                info!("F7: reloading schemes");
                self.reload_book();
            } else if vk == k.panel {
                self.bridge.request_toggle();
            }
        }
        self.process_ui_commands();
        if let Some(trace) = self.sound_trace.as_mut() {
            trace.flush();
        }
        let open = menu::relic_equip_open(&self.addrs);
        if open != self.relic_menu_open {
            self.relic_menu_open = open;
            self.bridge.set_relic_menu_open(open);
            info!("relic equip menu {}", if open { "opened" } else { "closed" });
        }
        self.sync_relics();
        if self.last_reconcile.elapsed() >= self.interval {
            self.last_reconcile = Instant::now();
            self.reconcile();
        }
        self.perf.frame_cost(started.elapsed());
    }

    fn limits(&self) -> Limits {
        Limits { min: self.cfg.rules.min, cap: self.cfg.rules.cap }
    }

    /// 重新读取 schemes.toml（F7 或面板保存后）
    fn reload_book(&mut self) {
        let Some(data) = self.data.as_ref() else {
            warn!("game data unavailable, nothing to reload");
            return;
        };
        self.book = SchemeBook::load(&self.dir, data, self.limits());
        self.book_generation += 1;
    }

    fn reconcile(&mut self) {
        let Some(player) = world::main_player(&self.addrs) else {
            self.last_player = 0;
            return;
        };
        if player != self.last_player {
            info!("main player now {player:#x}");
            self.last_player = player;
            self.pending_adds.clear();
            self.restore.reset();
        }
        let Some(current) = sp_effect::list_ids(player) else { return };
        let game_data = self.locator.game_data(&self.addrs, player);
        let snapshot = game_data.and_then(|gd| self.locator.snapshot(gd));
        let vessel = self.equipped_vessel(game_data, snapshot.as_ref());
        self.sync_loadouts(game_data);

        self.update_desired(&current, snapshot.as_ref(), vessel);
        if let Some(s) = snapshot.as_ref() {
            if let Some(hero) = self.data.as_ref().and_then(|d| d.hero_of_chara(s.chara_init)) {
                self.publish_snapshot(hero, vessel, s.level);
                self.publish_live(hero, s);
            }
        }
        // 新角色血量字段稳定之前不挂组件，保证满血判断可靠
        if let Some(s) = snapshot.as_ref() {
            if !self.spawn_gate.ready(player, s.hp(), s.fp()) {
                return;
            }
        }
        self.track_failures(&current);

        // 先处理上一轮的满血恢复，再为本轮变化重新布置观察
        if let (Some(s), Some(d)) = (snapshot.as_ref(), self.data.as_ref()) {
            let (hp, fp) = self.restore.poll(player, s.hp(), s.fp());
            if hp {
                sp_effect::apply(&self.addrs, player, d.restore_hp, false);
            }
            if fp {
                sp_effect::apply(&self.addrs, player, d.restore_fp, false);
            }
            if hp || fp {
                self.perf.native_call();
                info!("restored to full: hp={hp} fp={fp}");
            }
        }

        let mut plan = self.reconciler.plan(&current, MAX_OPS_PER_TICK);
        plan.add.retain(|id| self.add_failures.get(id).copied().unwrap_or(0) < MAX_ADD_FAILURES);
        if !plan.is_empty() {
            if let Some(s) = snapshot.as_ref() {
                self.restore.arm(player, s.hp(), s.fp());
            }
            // 移除只在本机生效（游戏不同步移除），队友端的旧组件靠停止续期后自然过期
            self.perf.native_call();
            for &id in &plan.remove {
                sp_effect::remove(&self.addrs, player, id);
            }
            for &id in &plan.add {
                sp_effect::apply(&self.addrs, player, id, self.cfg.sync_network);
            }
            info!("reconcile on {player:#x}: add {:?} remove {:?}", plan.add, plan.remove);
        }

        // 续期：重新施加仍需要的组件，重置本机与队友端的时长（同 ID 只重置不叠加）
        if self.last_refresh.elapsed() >= self.lease_refresh {
            self.last_refresh = Instant::now();
            let mut refreshed = false;
            for &id in self.reconciler.desired() {
                if current.contains(&id) && !plan.add.contains(&id) {
                    sp_effect::apply(&self.addrs, player, id, self.cfg.sync_network);
                    refreshed = true;
                }
            }
            if refreshed {
                self.perf.native_call();
            }
        }
        self.pending_adds = plan.add;
    }

    /// 各角色按自己当前装备的器皿取方案里的初始武器，有变化时改写参数表（出征开局按它发放）
    fn sync_loadouts(&mut self, game_data: Option<usize>) {
        let (Some(gd), Some(applier), Some(data)) = (game_data, self.loadouts.as_mut(), self.data.as_ref()) else { return };
        let book = &self.book;
        applier.sync(|hero| {
            let vessel = equip_data::equipped_vessel(gd, hero as u32 + 1).filter(|&id| data.vessel(id).is_some());
            book.weapons(hero, vessel).cloned()
        });
    }

    /// 当前角色装备的器皿（读不到或不是已知器皿时为 None，回退到角色默认方案）
    fn equipped_vessel(&self, game_data: Option<usize>, snapshot: Option<&PlayerSnapshot>) -> Option<u32> {
        let data = self.data.as_ref()?;
        let hero = data.hero_of_chara(snapshot?.chara_init)?;
        let id = equip_data::equipped_vessel(game_data?, hero as u32 + 1)?;
        data.vessel(id).map(|_| id)
    }

    /// 计算本轮期望的组件；状态或差值变化时写一行日志
    fn update_desired(&mut self, current: &[i32], snapshot: Option<&PlayerSnapshot>, vessel: Option<u32>) {
        let status = desired::decide(&Inputs {
            mode: self.cfg.mode,
            enabled: self.enabled,
            test: self.cfg.test.as_array(),
            data: self.data.as_deref(),
            book: &self.book,
            snapshot,
            vessel,
            speffects: current,
        });
        if status != self.status {
            info!("status: {}", status.describe(self.data.as_deref()));
            self.status = status;
        }
        let (ids, clamped) = decompose(&self.status.delta());
        if ids != self.reconciler.desired() {
            if clamped {
                warn!("delta exceeds ±127 and was clamped");
            }
            info!("desired delta {:?}", compose(&ids));
            self.reconciler.set_desired(ids);
            self.add_failures.clear();
        }
    }

    /// 上一轮补挂的组件这一轮仍然不在，记一次失败
    fn track_failures(&mut self, current: &[i32]) {
        for id in self.pending_adds.drain(..) {
            if !current.contains(&id) {
                let n = self.add_failures.entry(id).or_insert(0);
                *n += 1;
                if *n == MAX_ADD_FAILURES {
                    error!("component {id} did not stick after {n} tries; is the StatAlloc regulation active?");
                }
            }
        }
    }
}
