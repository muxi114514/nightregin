//! 叠加层：hudhook 钩住 DX12 Present，在遗物装配界面上画 ImGui 界面；出征中锻造台打开词条继承面板。
//! 运行在游戏的渲染线程，与游戏线程只通过 UiBridge 交换数据。

mod fonts;
mod inherit;
mod input_block;
mod midnight;
mod overlay;
mod panel;
mod reroll;
mod sfx;
mod skin;
mod text;
mod theme;
mod widgets;

use std::path::Path;
use std::sync::Arc;

use hudhook::Hudhook;
use hudhook::hooks::dx12::ImguiDx12Hooks;
use log::{error, info};

use crate::bridge::UiBridge;
use crate::config::UiConfig;
use crate::data::{GameData, InheritData, RelicData, StartingData};
use crate::relic_reroll::Pricing;
use crate::stats::rules::Limits;

use overlay::Overlay;

/// 会出现在面板上的文字所在的源码：界面文字、编辑 / 规则校验提示、保存与重铸结果、名称表。
/// 字体只为其中出现过的字生成字形，新增提示无需另行登记（注释里的字也会带上，图集仍然很小）
const GLYPH_SOURCES: [&str; 18] = [
    include_str!("text.rs"),
    include_str!("midnight/text.rs"),
    include_str!("midnight/curse.rs"),
    include_str!("reroll/text.rs"),
    include_str!("inherit/text.rs"),
    include_str!("../editor.rs"),
    include_str!("../stats/rules.rs"),
    include_str!("../stats/mod.rs"),
    include_str!("../stats/heroes.rs"),
    include_str!("../app/ui_sync.rs"),
    include_str!("../app/relic_sync.rs"),
    include_str!("../relic_reroll/mod.rs"),
    include_str!("../passive_inherit/mod.rs"),
    include_str!("../scheme_file/write.rs"),
    include_str!("../data/vessel_names.rs"),
    include_str!("../data/weapon_names.rs"),
    include_str!("../data/weapon_text.rs"),
    include_str!("../data/relic_text.rs"),
];

/// imgui-rs 同一进程同时只允许一个上下文，并行运行的绘制冒烟测试先取这把锁（锁要比上下文活得久）
#[cfg(test)]
pub(crate) fn imgui_test_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// 各面板的只读数据；不可用的为 None（对应面板显示不可用，或入口不出现）
pub struct PanelData {
    pub game: Option<Arc<GameData>>,
    /// 初始武器（属性分配面板中栏）
    pub starting: Option<Arc<StartingData>>,
    /// 遗物重铸的数据与计价（None 时不显示重铸按钮）
    pub relics: Option<(Arc<RelicData>, Pricing)>,
    /// 词条继承（None 时继承面板不会打开）
    pub inherit: Option<Arc<InheritData>>,
}

/// dir：DLL 所在目录（原生素材在其下的 skin\）
pub fn install(bridge: Arc<UiBridge>, midnight: Option<Arc<crate::midnight::MidnightBridge>>, panels: PanelData, cfg: UiConfig, limits: Limits, dir: &Path) {
    let mut texts: Vec<&str> = GLYPH_SOURCES.to_vec();
    if let Some(d) = panels.game.as_deref() {
        texts.extend(d.vessels.iter().map(|v| v.name.as_str()));
    }
    let glyphs = fonts::glyph_ranges(&texts);
    let overlay = Overlay::new(bridge, midnight, panels, cfg, limits, glyphs, dir.join("skin"));
    match Hudhook::builder().with::<ImguiDx12Hooks>(overlay).build().apply() {
        Ok(()) => info!("overlay hooks installed (DX12)"),
        Err(e) => {
            error!("overlay hooks failed: {e:?}; panel unavailable");
            return;
        }
    }
    // MinHook 由 hudhook 初始化，输入屏蔽钩子必须在它之后安装
    input_block::install();
}
