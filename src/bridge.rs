//! 游戏线程与叠加层（渲染线程）之间的共享状态：游戏线程发布，渲染线程读取。
//! 只用原子量与短暂持有的锁；渲染线程从不调用游戏函数。

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use crate::passive_inherit::{InheritOutcome, InheritRequest, InheritView};
use crate::relic_reroll::{RelicView, RerollOutcome, RerollRequest};
use crate::stats::growth::Stats;
use crate::stats::rules::SchemeSpec;

/// 面板编辑所需的方案快照：游戏线程在角色 / 器皿 / 等级变化或方案重载后发布
#[derive(Debug, Clone, PartialEq)]
pub struct SchemeSnapshot {
    pub hero: usize,
    pub vessel: Option<u32>,
    pub level: u32,
    /// 当前器皿的专属方案
    pub vessel_spec: Option<SchemeSpec>,
    /// 角色默认方案
    pub default_spec: Option<SchemeSpec>,
}

/// 角色实时属性与三维上限（变化时发布），面板据此推算遗物等装备对三维预览的影响
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LiveStats {
    pub hero: usize,
    pub level: u32,
    /// 当前实际属性（含遗物与组件）
    pub effective: Stats,
    /// 当前生效方案（无方案时为原版）在当前等级的目标属性
    pub target: Stats,
    /// 血量 / 专注值 / 精力上限
    pub max: [u32; 3],
}

/// 面板发给游戏线程的命令（写文件、重载方案、改遗物都在游戏线程完成）
#[derive(Debug, Clone)]
pub enum UiCommand {
    Scheme(SchemeCommand),
    Reroll(RerollRequest),
}

/// 属性分配方案的保存 / 删除
#[derive(Debug, Clone)]
pub enum SchemeCommand {
    Save { hero: usize, vessel: Option<u32>, spec: SchemeSpec },
    Delete { hero: usize, vessel: Option<u32> },
}

/// 命令执行结果，显示在面板底部
#[derive(Debug, Clone)]
pub struct UiMessage {
    pub text: String,
    pub ok: bool,
}

/// 蓝10 装备界面的入口按钮
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuickForgeAction {
    Reinforce,
    Inherit,
}

/// 三选一界面的刷新按钮：游戏线程发布，渲染线程显示
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrbRefreshView {
    /// 这个球还能刷新几次
    pub remaining: u32,
    /// 下一次的卢恩价格
    pub cost: u32,
    pub affordable: bool,
}

/// 带版本号的值：发布方每次写入加 1，读取方记住看过的版本，只在有新值时复制
struct Versioned<T> {
    value: Mutex<Option<T>>,
    version: AtomicU64,
}

impl<T> Default for Versioned<T> {
    fn default() -> Self {
        Self { value: Mutex::new(None), version: AtomicU64::new(0) }
    }
}

impl<T: Clone> Versioned<T> {
    fn publish(&self, value: T) {
        if let Ok(mut v) = self.value.lock() {
            *v = Some(value);
        }
        self.version.fetch_add(1, Ordering::AcqRel);
    }

    fn if_newer(&self, seen: &mut u64) -> Option<T> {
        let version = self.version.load(Ordering::Acquire);
        if version == *seen {
            return None;
        }
        let value = self.value.lock().ok()?.clone()?;
        *seen = version;
        Some(value)
    }
}

#[derive(Default)]
pub struct UiBridge {
    relic_menu_open: AtomicBool,
    toggle_requested: AtomicBool,
    snapshot: Versioned<Arc<SchemeSnapshot>>,
    commands: Mutex<Vec<UiCommand>>,
    message: Versioned<UiMessage>,
    live: Mutex<Option<LiveStats>>,
    relics: Versioned<Arc<RelicView>>,
    reroll_outcome: Versioned<RerollOutcome>,
    /// 词条继承面板：游戏线程打开（锻造台 / 测试热键），任一方关闭（返回 / 离开出征地图）
    inherit_open: AtomicBool,
    inherit_view: Versioned<Arc<InheritView>>,
    /// 一次只有一个请求（面板等结果期间不再发）
    inherit_request: Mutex<Option<InheritRequest>>,
    inherit_outcome: Versioned<InheritOutcome>,
    /// 蓝10：游戏线程决定入口按钮显不显示，渲染线程发点击
    quick_forge_shown: AtomicBool,
    quick_forge_click: Mutex<Option<QuickForgeAction>>,
    orb_refresh: Mutex<Option<OrbRefreshView>>,
    orb_refresh_click: AtomicBool,
}

impl UiBridge {
    pub fn set_relic_menu_open(&self, open: bool) {
        self.relic_menu_open.store(open, Ordering::Release);
    }

    pub fn relic_menu_open(&self) -> bool {
        self.relic_menu_open.load(Ordering::Acquire)
    }

    pub fn set_quick_forge_shown(&self, shown: bool) {
        self.quick_forge_shown.store(shown, Ordering::Release);
    }

    pub fn quick_forge_shown(&self) -> bool {
        self.quick_forge_shown.load(Ordering::Acquire)
    }

    pub fn click_quick_forge(&self, action: QuickForgeAction) {
        if let Ok(mut c) = self.quick_forge_click.lock() {
            *c = Some(action);
        }
    }

    pub fn take_quick_forge_click(&self) -> Option<QuickForgeAction> {
        self.quick_forge_click.lock().ok()?.take()
    }

    pub fn set_orb_refresh(&self, view: Option<OrbRefreshView>) {
        if let Ok(mut v) = self.orb_refresh.lock() {
            *v = view;
        }
    }

    pub fn orb_refresh(&self) -> Option<OrbRefreshView> {
        *self.orb_refresh.lock().ok()?
    }

    pub fn click_orb_refresh(&self) {
        self.orb_refresh_click.store(true, Ordering::Release);
    }

    pub fn take_orb_refresh_click(&self) -> bool {
        self.orb_refresh_click.swap(false, Ordering::AcqRel)
    }

    /// 面板热键：游戏线程检测按键，渲染线程消费
    pub fn request_toggle(&self) {
        self.toggle_requested.store(true, Ordering::Release);
    }

    pub fn take_toggle(&self) -> bool {
        self.toggle_requested.swap(false, Ordering::AcqRel)
    }

    pub fn publish_snapshot(&self, snapshot: SchemeSnapshot) {
        self.snapshot.publish(Arc::new(snapshot));
    }

    /// 版本号比 seen 新时才返回快照（避免每帧复制）
    pub fn snapshot_if_newer(&self, seen: &mut u64) -> Option<Arc<SchemeSnapshot>> {
        self.snapshot.if_newer(seen)
    }

    pub fn publish_live(&self, live: LiveStats) {
        if let Ok(mut l) = self.live.lock() {
            *l = Some(live);
        }
    }

    pub fn live(&self) -> Option<LiveStats> {
        self.live.lock().ok().and_then(|l| *l)
    }

    pub fn send(&self, cmd: UiCommand) {
        if let Ok(mut q) = self.commands.lock() {
            q.push(cmd);
        }
    }

    pub fn take_commands(&self) -> Vec<UiCommand> {
        self.commands.lock().map(|mut q| std::mem::take(&mut *q)).unwrap_or_default()
    }

    pub fn post_message(&self, text: String, ok: bool) {
        self.message.publish(UiMessage { text, ok });
    }

    pub fn message_if_newer(&self, seen: &mut u64) -> Option<UiMessage> {
        self.message.if_newer(seen)
    }

    /// 当前角色装备的遗物与货币（遗物装配界面打开时由游戏线程定期发布）
    pub fn publish_relics(&self, view: RelicView) {
        self.relics.publish(Arc::new(view));
    }

    pub fn relics_if_newer(&self, seen: &mut u64) -> Option<Arc<RelicView>> {
        self.relics.if_newer(seen)
    }

    pub fn post_reroll_outcome(&self, outcome: RerollOutcome) {
        self.reroll_outcome.publish(outcome);
    }

    pub fn reroll_outcome_if_newer(&self, seen: &mut u64) -> Option<RerollOutcome> {
        self.reroll_outcome.if_newer(seen)
    }

    pub fn open_inherit(&self) {
        self.inherit_open.store(true, Ordering::Release);
    }

    pub fn close_inherit(&self) {
        self.inherit_open.store(false, Ordering::Release);
    }

    pub fn inherit_open(&self) -> bool {
        self.inherit_open.load(Ordering::Acquire)
    }

    pub fn publish_inherit_view(&self, view: InheritView) {
        self.inherit_view.publish(Arc::new(view));
    }

    pub fn inherit_view_if_newer(&self, seen: &mut u64) -> Option<Arc<InheritView>> {
        self.inherit_view.if_newer(seen)
    }

    pub fn send_inherit(&self, req: InheritRequest) {
        if let Ok(mut r) = self.inherit_request.lock() {
            *r = Some(req);
        }
    }

    pub fn take_inherit_request(&self) -> Option<InheritRequest> {
        self.inherit_request.lock().ok()?.take()
    }

    pub fn post_inherit_outcome(&self, outcome: InheritOutcome) {
        self.inherit_outcome.publish(outcome);
    }

    pub fn inherit_outcome_if_newer(&self, seen: &mut u64) -> Option<InheritOutcome> {
        self.inherit_outcome.if_newer(seen)
    }
}
