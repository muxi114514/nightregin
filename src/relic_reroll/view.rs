//! 游戏线程与重铸面板之间交换的数据：当前角色的遗物与货币（游戏线程发布）、重铸请求（面板发出）与结果。

use nr_stat_alloc_shared::relic_reroll::{Cost, Prices, RelicCatalog, RelicEffects, Target};

use crate::game::currency::Currency;
use crate::game::relic::EQUIPPED_SLOTS;

/// 当前角色装备的一件遗物
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EquippedRelic {
    pub handle: u32,
    /// EquipParamAntique 行号
    pub relic: u32,
    pub effects: RelicEffects,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelicView {
    /// 角色序号（0 起）
    pub hero: usize,
    /// 前 3 个普通栏、后 3 个深夜栏
    pub slots: [Option<EquippedRelic>; EQUIPPED_SLOTS],
    pub murk: u32,
    pub sigils: u32,
}

#[derive(Debug, Clone)]
pub struct RerollRequest {
    pub hero: usize,
    pub slot: usize,
    pub handle: u32,
    /// 面板看到的词条；与游戏线程读到的不一致时拒绝，避免按旧状态扣费
    pub effects: RelicEffects,
    pub targets: Vec<Target>,
    /// 取消勾选的分组
    pub excluded: Vec<u32>,
    pub currency: Currency,
    /// 面板显示的价格；游戏线程重算后不一致时拒绝
    pub amount: u32,
}

#[derive(Debug, Clone)]
pub struct RerollOutcome {
    pub ok: bool,
    pub text: String,
    /// 成功时遗物上新出现的词条（面板高亮用）
    pub new_effects: Vec<u32>,
}

/// 计价参数：商店价格（配置可覆盖）与筛选加价指数
#[derive(Debug, Clone, Copy)]
pub struct Pricing {
    pub prices: Prices,
    pub exponent: f32,
}

impl Pricing {
    /// hero 为角色序号（0 起）；excluded 为取消勾选的分组
    pub fn quote(&self, catalog: &RelicCatalog, relic: &EquippedRelic, targets: &[Target], hero: usize, excluded: &[u32]) -> Option<Cost> {
        let excluded = excluded.iter().copied().collect();
        catalog.quote(&self.prices, relic.relic, &relic.effects, targets, hero as u32 + 1, &excluded, self.exponent)
    }
}

pub fn amount_of(cost: Cost, currency: Currency) -> u32 {
    match currency {
        Currency::Murk => cost.murk,
        Currency::Sigils => cost.sigils,
    }
}
