//! 初始化线程：读配置、开日志、校验地址、等待游戏任务系统就绪并注册每帧任务。

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use log::{error, info, warn};
use windows_sys::Win32::System::LibraryLoader::GetModuleFileNameW;

use crate::app::App;
use crate::boss_loot;
use crate::bridge::UiBridge;
use crate::config::{self, Config};
use crate::data::{self, InheritData, RelicData};
use crate::diag;
use crate::game::addresses::{self, Addresses, GaitemAddresses};
use crate::game::event_flags::{self, FlagHandler, QueueFilter};
use crate::game::equip_ops::EquipOps;
use crate::game::gaitem::Gaitems;
use crate::game::module::GameModule;
use crate::game::task;
use crate::crucible_sword;
use crate::havok_guard;
use crate::lang;
use crate::legendary_upgrade;
use crate::logger;
use crate::midnight::{self, curse::CurseEngine};
use crate::passive_inherit::{self, InheritTask, PassiveInherit};
use crate::passive_upgrade;
use crate::relic_reroll::{Pricing, RelicReroll};
use crate::rune_attack;
use crate::sound_banks;
use crate::starting_weapons::{LoadoutApplier, StartingPassives};
use crate::stats::rules::Limits;
use crate::super_orb;
use crate::tiered_effects::{LocatedParams, TieredEffects, Weapons};
use crate::ui::{self, PanelData};

/// 每帧任务一旦 panic 就在本次游戏会话内停用，防止反复崩溃或刷屏
static DISABLED: AtomicBool = AtomicBool::new(false);
/// 出征随机词条单独停用，不连带属性分配
static PASSIVES_DISABLED: AtomicBool = AtomicBool::new(false);
static INHERIT_DISABLED: AtomicBool = AtomicBool::new(false);
static TIERED_DISABLED: AtomicBool = AtomicBool::new(false);
static MIDNIGHT_DISABLED: AtomicBool = AtomicBool::new(false);
const TASK_WAIT_TIMEOUT: Duration = Duration::from_secs(600);

pub fn run(dll_module: usize) {
    let dir = dll_dir(dll_module).unwrap_or_else(|| PathBuf::from("."));
    let (cfg, cfg_message) = config::load(&dir);
    if logger::init(&dir, cfg.log_filter()).is_err() {
        return;
    }
    // 默认钩子写 stderr（游戏没有控制台），改为写日志，保留 panic 的位置与原因
    std::panic::set_hook(Box::new(|info| error!("panic: {info}")));
    info!("NRStatAlloc {} starting from {}", env!("CARGO_PKG_VERSION"), dir.display());
    info!("{cfg_message}");
    let game_dir = std::env::current_exe().ok().and_then(|p| p.parent().map(PathBuf::from)).unwrap_or_default();
    let (language, note) = lang::resolve(&cfg.ui.language, &game_dir);
    lang::set(language);
    info!("{note}");
    if catch_unwind(AssertUnwindSafe(|| start(cfg, dir))).is_err() {
        error!("bootstrap panicked, mod disabled");
    }
}

fn start(cfg: Config, dir: PathBuf) {
    let Some(module) = GameModule::current() else {
        error!("cannot read game module");
        return;
    };
    info!("game base {:#x} size {:#x} pe timestamp {:#x}", module.base, module.size, module.timestamp);

    let addrs = match addresses::resolve(&module) {
        Ok(a) => a,
        Err(e) => {
            error!("address check failed: {e}; mod disabled (wrong game build?)");
            return;
        }
    };
    let Some(task_groups) = task::wait_for_task_groups(&addrs, TASK_WAIT_TIMEOUT) else {
        error!("CSTaskGroupImp not available after {TASK_WAIT_TIMEOUT:?}");
        return;
    };
    let group = task::frame_begin_index(task_groups);
    // 参数表在游戏启动早期加载，这里等它就绪后一次性读出；读不到时面板与方案仍可编辑，只是不生效
    let (data, weapon_data, relic_data, rune_curve, tiered_params, orb_fxr, curse_engine) = match data::wait_for_game(&module, &addrs, TASK_WAIT_TIMEOUT, cfg.relic_reroll.include_custom) {
        Ok(loaded) => {
            boss_loot::apply(&cfg.boss_loot, &loaded.repository);
            let rune_curve = rune_attack::curve_row(&loaded.repository);
            let tiered_params = cfg.tiered_buffs.enabled.then(|| LocatedParams::locate(&loaded.repository)).unwrap_or_default();
            let orb_fxr = super_orb::group_orb_fxr(&loaded.repository);
            let curse_engine = cfg.midnight.enabled.then(|| CurseEngine::locate(&loaded.repository)).unwrap_or_else(CurseEngine::empty);
            (Some(Arc::new(loaded.data)), Some(loaded.weapons), loaded.relics.map(Arc::new), rune_curve, tiered_params, orb_fxr, curse_engine)
        }
        Err(e) => {
            error!("game data unavailable: {e}");
            (None, None, None, None, LocatedParams::default(), None, CurseEngine::empty())
        }
    };
    legendary_upgrade::apply(&cfg.legendary_upgrade, &module);
    sound_banks::load_in_background(&dir);
    let rune_targets = rune_attack::locate(&module, rune_curve);
    let havok_target = havok_guard::locate(&module);
    let crucible_targets = cfg.tiered_buffs.enabled.then(|| crucible_sword::locate(&module)).flatten();
    let super_orb_targets = super_orb::locate(&module, orb_fxr);
    let (passive_tiers, starting, inherit_data, rarities) =
        weapon_data.map_or((None, None, None, None), |w| (w.passive_tiers, w.starting.map(Arc::new), w.inherit.map(Arc::new), w.rarities));
    let gaitem_addrs = addresses::resolve_gaitem(&module).map_err(|e| warn!("gaitem features disabled: {e}")).ok();
    let reroll = reroll_setup(&cfg, relic_data, gaitem_addrs.as_ref());
    let bridge = Arc::new(UiBridge::default());
    let inherit = inherit_setup(&cfg, &module, &addrs, inherit_data.clone(), gaitem_addrs.as_ref(), &bridge);
    let mut starting_passives = match (&starting, &gaitem_addrs) {
        (Some(s), Some(g)) if cfg.starting_weapons.random_passive => Some(StartingPassives::new(addrs.clone(), &cfg.offsets, Arc::clone(s), g)),
        _ => None,
    };
    info!("registering frame task in group {group} (CSTaskGroupImp {task_groups:#x})");

    let flag_addrs = addresses::resolve_event_flags(&module);
    let inherit_entry = inherit.is_some();
    let midnight_on = cfg.midnight.enabled && cfg.ui.enabled;
    let (mut midnight_task, midnight_bridge, midnight_native) = match midnight_on.then(|| midnight::setup(&cfg.midnight, &dir, &module, &addrs, &cfg.offsets, curse_engine, cfg.diag.midnight_curse)) {
        Some((t, b, n)) => (Some(t), Some(b), Some(n)),
        None => (None, None, None),
    };
    let inherit_ui = inherit_data.filter(|_| inherit.is_some());
    let mut inherit_task = inherit;
    let task_addrs = addrs.clone();
    let ui_cfg = cfg.ui.clone();
    let sound_trace = cfg.diag.sound_trace;
    let weapon_passive = cfg.weapon_passive.clone();
    let limits = Limits { min: cfg.rules.min, cap: cfg.rules.cap };
    let loadouts = starting.clone().map(LoadoutApplier::new);
    let reroll_ui = reroll.as_ref().map(|(d, p, _)| (Arc::clone(d), *p));
    let reroll_service = reroll.map(|(d, p, g)| RelicReroll::new(d, p, g));
    let tiered_weapons = rarities.zip(gaitem_addrs.as_ref()).map(|(r, g)| Weapons::new(r, g.change_item_id));
    let orb_refresh = cfg.tiered_buffs.enabled.then(|| addresses::resolve_private_item(&module).map_err(|e| warn!("orb refresh disabled: {e}")).ok()).flatten().zip(data.clone());
    let mut tiered = cfg.tiered_buffs.enabled.then(|| TieredEffects::new(addrs.clone(), &cfg.offsets, cfg.tiered_buffs.clone(), tiered_weapons, tiered_params, Arc::clone(&bridge), orb_refresh));
    let mut app = App::new(cfg, dir.clone(), module, addrs, Arc::clone(&bridge), data.clone(), loadouts, reroll_service);
    let closure = Box::new(move || {
        if !DISABLED.load(Ordering::Relaxed) && catch_unwind(AssertUnwindSafe(|| app.tick())).is_err() {
            DISABLED.store(true, Ordering::Relaxed);
            error!("frame task panicked; mod disabled for this session");
        }
        if let Some(p) = starting_passives.as_mut()
            && !PASSIVES_DISABLED.load(Ordering::Relaxed)
            && catch_unwind(AssertUnwindSafe(|| p.tick())).is_err()
        {
            PASSIVES_DISABLED.store(true, Ordering::Relaxed);
            error!("starting passive panicked; disabled for this session");
        }
        if let Some(t) = inherit_task.as_mut()
            && !INHERIT_DISABLED.load(Ordering::Relaxed)
            && catch_unwind(AssertUnwindSafe(|| t.tick())).is_err()
        {
            INHERIT_DISABLED.store(true, Ordering::Relaxed);
            error!("passive inherit panicked; disabled for this session");
        }
        if let Some(t) = tiered.as_mut()
            && !TIERED_DISABLED.load(Ordering::Relaxed)
            && catch_unwind(AssertUnwindSafe(|| t.tick())).is_err()
        {
            TIERED_DISABLED.store(true, Ordering::Relaxed);
            error!("tiered buffs panicked; disabled for this session");
        }
        if let Some(t) = midnight_task.as_mut()
            && !MIDNIGHT_DISABLED.load(Ordering::Relaxed)
            && catch_unwind(AssertUnwindSafe(|| t.tick())).is_err()
        {
            MIDNIGHT_DISABLED.store(true, Ordering::Relaxed);
            if catch_unwind(AssertUnwindSafe(|| t.release())).is_err() {
                error!("midnight: restoring curse data panicked");
            }
            midnight::shutdown();
            error!("midnight panicked; disabled for this session");
        }
    });
    task::register_recurring(&task_addrs, task_groups, group, closure);
    if ui_cfg.enabled {
        let panels = PanelData { game: data, starting, relics: reroll_ui, inherit: inherit_ui };
        ui::install(bridge, midnight_bridge, panels, ui_cfg, limits, &dir);
        if sound_trace {
            diag::sound_trace::install();
        }
    }
    // MinHook 由叠加层初始化，所以放在它之后
    if let Some(g) = &gaitem_addrs {
        passive_upgrade::install(&weapon_passive, passive_tiers, g);
    }
    install_flag_hooks(&flag_addrs, inherit_entry, midnight_on);
    if let Some(n) = &midnight_native {
        midnight::install_hooks(n);
    }
    if let Some(t) = rune_targets {
        rune_attack::install(t);
    }
    if let Some(t) = havok_target {
        havok_guard::install(t);
    }
    if let Some(t) = crucible_targets {
        crucible_sword::install(t, task_addrs);
    }
    if let Some(t) = super_orb_targets {
        super_orb::install(t);
    }
    info!("ready: F6 panel / F7 reload schemes / F8 calibrate / F9 dump / F10 enable-disable");
}

/// 重铸所需的数据、计价与物品实例访问；配置关闭或任一缺失时为 None（按钮不显示）
fn reroll_setup(cfg: &Config, data: Option<Arc<RelicData>>, gaitem: Option<&GaitemAddresses>) -> Option<(Arc<RelicData>, Pricing, Gaitems)> {
    if !cfg.relic_reroll.enabled {
        info!("relic reroll disabled by config");
        return None;
    }
    let (Some(data), Some(gaitem)) = (data, gaitem) else { return None };
    if gaitem.vt_atq_gaitem_ins.is_none() {
        warn!("relic reroll disabled: relic instance vtable not verified");
        return None;
    }
    let pricing = Pricing { prices: cfg.relic_reroll.prices(data.shop_prices), exponent: cfg.relic_reroll.price_exponent };
    info!("relic reroll ready: {:?}, exponent {}", pricing.prices, pricing.exponent);
    Some((data, pricing, Gaitems::new(gaitem)))
}

/// 词条继承的每帧任务与对话脚本入口用到的地址（旗标函数找不到时只停用入口）；配置关闭或其余任一缺失时为 None
fn inherit_setup(
    cfg: &Config,
    module: &GameModule,
    addrs: &Addresses,
    data: Option<Arc<InheritData>>,
    gaitem: Option<&GaitemAddresses>,
    bridge: &Arc<UiBridge>,
) -> Option<InheritTask> {
    if !cfg.passive_inherit.enabled {
        info!("passive inherit disabled by config");
        return None;
    }
    let (Some(data), Some(gaitem)) = (data, gaitem) else {
        warn!("passive inherit disabled: weapon rarity table or gaitem addresses unavailable");
        return None;
    };
    if !cfg.ui.enabled {
        warn!("passive inherit disabled: the overlay ([ui] enabled) is off, the panel cannot be shown");
        return None;
    }
    let native = addresses::resolve_inherit(module).map_err(|e| warn!("passive inherit disabled: {e}")).ok()?;
    info!("passive inherit ready: {} weapon rarities", data.rarities.len());
    let service = PassiveInherit::new(data, Gaitems::new(gaitem), EquipOps::new(gaitem.change_item_id, native.remove_item));
    let task = InheritTask::new(service, Arc::clone(bridge), addrs.clone(), &cfg.offsets, cfg.diag.inherit_panel_key);
    Some(task)
}

/// 写旗标钩子：继承入口（对话脚本请求）与各功能的处理函数一起注册，同一函数只能钩一次
fn install_flag_hooks(addrs: &addresses::EventFlagAddresses, inherit: bool, midnight: bool) {
    let mut handlers: Vec<FlagHandler> = Vec::new();
    let mut filters: Vec<QueueFilter> = Vec::new();
    if inherit {
        handlers.push(passive_inherit::on_flag);
        filters.push(passive_inherit::block_queue);
    }
    if midnight {
        handlers.extend(midnight::flag_handlers());
    }
    if handlers.is_empty() {
        return;
    }
    let report = event_flags::install(addrs, handlers, filters);
    if inherit {
        info!("passive inherit entry installed (flag {}, hooked {}, net sync blocked: {})", passive_inherit::REQUEST_FLAG, report.local, report.queue);
    }
}

fn dll_dir(dll_module: usize) -> Option<PathBuf> {
    let mut buf = [0u16; 1024];
    let len = unsafe { GetModuleFileNameW(dll_module as _, buf.as_mut_ptr(), buf.len() as u32) } as usize;
    if len == 0 {
        return None;
    }
    PathBuf::from(String::from_utf16_lossy(&buf[..len])).parent().map(PathBuf::from)
}
