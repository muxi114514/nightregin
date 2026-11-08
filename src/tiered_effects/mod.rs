//! 三档锻体里靠 DLL 实现的词条（配置 [tiered_buffs]）。regulation 里只有标记效果（shared::tiered_runes / tiered_flasks / tiered_weapons / tiered_items / tiered_downed /
//! tiered_low_hp / tiered_beast / tiered_params / tiered_guardian / tiered_sixth_sense / tiered_refund / tiered_cold_mirage / tiered_quick_forge / tiered_mixcraft），
//! 这里每帧看主角身上有没有标记再动手，都在游戏主线程（每帧任务）里执行；只在出击中生效，回到圆桌就清空记录。

mod beast;
mod cold_mirage;
mod direct_cocktail;
mod downed;
mod flasks;
mod gauge_refund;
mod guardian;
mod items;
mod low_hp;
#[cfg(feature = "tiered-diag")]
mod mem_diff;
mod orb_refresh;
mod param_overrides;
mod quick_forge;
mod runes;
mod sixth_sense;
#[cfg(feature = "tiered-diag")]
mod watch;
mod weapons;

use std::sync::Arc;
use std::time::Instant;

use log::{info, warn};
use nr_stat_alloc_shared::orb_refresh::Pools;
use nr_stat_alloc_shared::tiered_downed::DOWNED_BURST_SPAWN;
use nr_stat_alloc_shared::tiered_flasks::{FLASK_FULL_ATTACK, FLASK_FULL_ATTACK_BUFF, GRACE_RELOAD};

use crate::bridge::UiBridge;
use crate::config::{OffsetOverrides, TieredBuffsConfig};
use crate::data::GameData;
use crate::game::addresses::{Addresses, PrivateItemAddresses};
use crate::game::player_data::PlayerDataLocator;
use crate::game::flasks::{self as flask_data, Flask};
use crate::game::params::ParamRepository;
use crate::game::{menu, runes as held_runes, sixth_sense as sixth_sense_count, sp_effect, world};
use crate::passive_inherit;
use crate::rng::XorShift;

use beast::BeastLock;
use cold_mirage::ColdMirage;
use direct_cocktail::DirectCocktail;
use downed::DownedBurst;
use flasks::{Plus, Refill};
use gauge_refund::GaugeRefund;
use guardian::Guardian;
use items::ItemLimits;
use low_hp::LowHp;
use orb_refresh::OrbRefresh;
use param_overrides::ParamOverrides;
use quick_forge::QuickForge;
use runes::{Gamble, Refund, Shield};
use sixth_sense::SixthSense;
pub use weapons::Weapons;

/// 本帧主角的状态
pub struct Frame<'a> {
    pub ids: &'a [i32],
    /// 血量（当前, 上限）
    pub hp: (u32, u32),
    pub runes: u32,
}

pub struct TieredEffects {
    addrs: Addresses,
    locator: PlayerDataLocator,
    cfg: TieredBuffsConfig,
    rng: XorShift,
    gamble: Gamble,
    refund: Refund,
    shield: Shield,
    refill: Refill,
    plus: Plus,
    downed: DownedBurst,
    beast: BeastLock,
    guardian: Guardian,
    sixth_sense: SixthSense,
    gauge_refund: GaugeRefund,
    cold_mirage: ColdMirage,
    quick_forge: QuickForge,
    #[cfg(feature = "tiered-diag")]
    watch: watch::Watch,
    /// 上次找到的圣杯瓶背包条目编号
    flask_hint: Option<u32>,
    /// 武器类（缺武器表或换编号函数时为 None）
    weapons: Option<Weapons>,
    /// 道具持有上限（缺道具表时为 None）
    items: Option<ItemLimits>,
    /// 血量偏低的判定线与锁血（缺效果表时为 None）
    low_hp: Option<LowHp>,
    /// 改内存参数的角色锻体（参数表都找不到时为 None）
    params: Option<ParamOverrides>,
    /// 隐士1（缺武器表时为 None）
    cocktail: Option<DirectCocktail>,
    /// 三选一选项刷新（配置关闭或地址、参数表缺失时为 None）
    orb_refresh: Option<OrbRefresh>,
}

/// 启动时（参数表还是原值）从内存参数表定位到的部分
#[derive(Default)]
pub struct LocatedParams {
    items: Option<ItemLimits>,
    low_hp: Option<LowHp>,
    params: Option<ParamOverrides>,
    cocktail: Option<DirectCocktail>,
    orb_pools: Option<Pools>,
    /// 诊断版下硬件断点的地址
    #[cfg(feature = "tiered-diag")]
    diag_breakpoints: Vec<usize>,
}

impl LocatedParams {
    pub fn locate(repository: &ParamRepository) -> Self {
        Self {
            items: ItemLimits::locate(repository),
            low_hp: LowHp::locate(repository),
            params: ParamOverrides::locate(repository),
            cocktail: DirectCocktail::locate(repository),
            orb_pools: orb_refresh::locate_pools(repository),
            #[cfg(feature = "tiered-diag")]
            diag_breakpoints: watch::sixth_sense_fields(repository),
        }
    }
}

impl TieredEffects {
    /// orb_refresh：三选一选项刷新要用的地址与角色表，缺了就没有这个功能
    pub fn new(
        addrs: Addresses,
        offsets: &OffsetOverrides,
        cfg: TieredBuffsConfig,
        weapons: Option<Weapons>,
        located: LocatedParams,
        bridge: Arc<UiBridge>,
        orb_refresh: Option<(PrivateItemAddresses, Arc<GameData>)>,
    ) -> Self {
        let orb_refresh = orb_refresh.zip(located.orb_pools).filter(|_| !cfg.orb_refresh_costs.is_empty()).map(|((private, data), pools)| {
            OrbRefresh::new(private, pools, data, cfg.orb_refresh_costs.clone(), Arc::clone(&bridge))
        });
        let quick_forge = QuickForge::new(cfg.quick_forge_key, cfg.quick_inherit_key, bridge);
        Self {
            addrs,
            locator: PlayerDataLocator::new(offsets),
            cfg,
            rng: XorShift::from_time(),
            gamble: Gamble::default(),
            refund: Refund::default(),
            shield: Shield::default(),
            refill: Refill::default(),
            plus: Plus::default(),
            downed: DownedBurst::default(),
            beast: BeastLock::default(),
            guardian: Guardian::default(),
            sixth_sense: SixthSense::default(),
            gauge_refund: GaugeRefund::default(),
            cold_mirage: ColdMirage::default(),
            quick_forge,
            #[cfg(feature = "tiered-diag")]
            watch: watch::Watch::new(located.diag_breakpoints),
            flask_hint: None,
            weapons,
            items: located.items,
            low_hp: located.low_hp,
            params: located.params,
            cocktail: located.cocktail,
            orb_refresh,
        }
    }

    pub fn tick(&mut self) {
        let Some(player) = world::main_player(&self.addrs) else { return self.reset() };
        if !world::current_map(player).is_some_and(world::is_expedition) {
            return self.reset();
        }
        let Some(ids) = sp_effect::list_ids(player) else { return };
        let Some(gd) = self.locator.game_data(&self.addrs, player) else { return };
        #[cfg(feature = "tiered-diag")]
        self.watch.observe(&ids, player, gd);
        let (Some(snapshot), Some(runes)) = (self.locator.snapshot(gd), held_runes::held(gd)) else { return };
        let frame = Frame { ids: &ids, hp: snapshot.hp(), runes };
        // 先和上一帧比有没有花掉卢恩，再做会改动卢恩的倍数与免死（它们改的不算花费）
        let mut held = self.refund.check(&frame, &self.cfg, &mut self.rng).unwrap_or(runes);
        if let Some(r) = self.gamble.check(&frame, held, &self.cfg, &mut self.rng) {
            held = r;
        }
        let addrs = &self.addrs;
        if let Some(r) = self.shield.check(&frame, held, &self.cfg, |id| sp_effect::apply(addrs, player, id, false)) {
            held = r;
        }
        if held != runes && !held_runes::set(gd, held) {
            warn!("tiered buffs: cannot write runes");
        }
        self.refund.remember(held);
        if self.downed.check(&frame) {
            sp_effect::apply(&self.addrs, player, DOWNED_BURST_SPAWN, false);
            info!("downed burst");
        }
        self.beast.check(&frame, |id| sp_effect::apply(addrs, player, id, false), |id| sp_effect::remove(addrs, player, id));
        self.guardian.check(&frame, |id| sp_effect::apply(addrs, player, id, false), |id| sp_effect::remove(addrs, player, id));
        self.sixth_sense.check(&frame, Instant::now(), || sixth_sense_count::set_count(player, gd, 1));
        self.gauge_refund.check(&frame, |id| sp_effect::apply(addrs, player, id, false), |id| sp_effect::remove(addrs, player, id));
        self.cold_mirage.check(&frame, Instant::now(), |id| sp_effect::apply(addrs, player, id, false), |id| sp_effect::remove(addrs, player, id));
        self.quick_forge.check(&frame, || menu::equip_open(addrs), || menu::open_reinforce(addrs), passive_inherit::request_panel);
        if let Some(orb) = self.orb_refresh.as_mut() {
            orb.tick(&frame, addrs, snapshot.chara_init, gd, &mut self.rng);
        }
        if let Some(cocktail) = self.cocktail.as_mut() {
            cocktail.tick(&frame, addrs, player, gd, Instant::now(), |id| sp_effect::apply(addrs, player, id, false));
        }
        if let Some(params) = self.params.as_mut() {
            params.tick(&frame);
        }
        self.flasks(&frame, gd, player);
        if let Some(w) = self.weapons.as_mut() {
            let addrs = &self.addrs;
            w.tick(&frame, gd, |id| sp_effect::apply(addrs, player, id, false), |id| sp_effect::remove(addrs, player, id));
        }
        if let Some(items) = self.items.as_mut() {
            items.tick(&frame, |id| sp_effect::apply(&self.addrs, player, id, false));
        }
        if let Some(low_hp) = self.low_hp.as_mut() {
            low_hp.tick(&frame, |id| sp_effect::apply(&self.addrs, player, id, false));
        }
    }

    /// 圣杯瓶类：只在要动手时才读背包
    fn flasks(&mut self, frame: &Frame, gd: usize, player: usize) {
        if self.refill.check(frame) {
            // 已装的瓶子直接补满；空瓶要换编号，交给游戏自己的「赐福 - 补充」
            sp_effect::apply(&self.addrs, player, GRACE_RELOAD, false);
            if let (Some(max), Some(f)) = (flask_data::max(gd), self.flask(gd))
                && f.filled()
                && f.quantity < max as u32
            {
                f.set_quantity(max as u32);
            }
            info!("flask refill on revive");
        }
        let Some(max) = flask_data::max(gd) else { return };
        let raised = self.plus.check(frame, max);
        let wants_full = frame.ids.contains(&FLASK_FULL_ATTACK);
        let has_buff = frame.ids.contains(&FLASK_FULL_ATTACK_BUFF);
        if raised.is_none() && !wants_full && !has_buff {
            return;
        }
        let flask = self.flask(gd);
        let mut cap = max;
        if let Some(new_max) = raised
            && flask_data::set_max(gd, new_max)
        {
            // 已装的瓶子也立刻多出新增的次数
            if let Some(f) = &flask
                && f.filled()
                && new_max > max
            {
                f.set_quantity(f.quantity + (new_max - max) as u32);
            }
            info!("flask cap {max} -> {new_max}");
            cap = new_max;
        }
        // 紫5：增益不限时，瓶满时挂上、不满（或没有这个锻体）时移除
        let full = wants_full && flask.as_ref().is_some_and(|f| flasks::full(f.filled(), f.quantity, cap));
        if full && !has_buff {
            sp_effect::apply(&self.addrs, player, FLASK_FULL_ATTACK_BUFF, false);
        } else if !full && has_buff {
            sp_effect::remove(&self.addrs, player, FLASK_FULL_ATTACK_BUFF);
        }
    }

    fn flask(&mut self, gd: usize) -> Option<Flask> {
        let found = flask_data::find(gd, self.flask_hint);
        self.flask_hint = found.as_ref().map(|f| f.entry.index);
        found
    }

    fn reset(&mut self) {
        self.gamble.reset();
        self.refund.reset();
        self.shield.reset();
        self.refill.reset();
        self.plus.reset();
        self.downed.reset();
        self.beast.reset();
        self.guardian.reset();
        self.sixth_sense.reset();
        self.gauge_refund.reset();
        self.cold_mirage.reset();
        self.quick_forge.reset();
        if let Some(orb) = self.orb_refresh.as_mut() {
            orb.reset();
        }
        if let Some(cocktail) = self.cocktail.as_mut() {
            cocktail.reset();
        }
        if let Some(params) = self.params.as_mut() {
            params.reset();
        }
        #[cfg(feature = "tiered-diag")]
        self.watch.reset();
        self.flask_hint = None;
        if let Some(w) = self.weapons.as_mut() {
            w.reset();
        }
        if let Some(items) = self.items.as_mut() {
            items.reset();
        }
        if let Some(low_hp) = self.low_hp.as_mut() {
            low_hp.reset();
        }
    }
}
