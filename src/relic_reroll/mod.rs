//! 遗物重铸（游戏线程）：读当前角色装备的遗物与货币发布给面板；执行面板发来的重铸请求——
//! 重新读取并核对遗物与价格 → 按规则抽取（shared relic_reroll）→ 扣费 → 写入词条与 SpEffect 缓存。
//! 写入失败时退还费用；所有步骤都在游戏主线程完成，面板只拿只读快照。

mod view;

pub use view::{EquippedRelic, Pricing, RelicView, RerollOutcome, RerollRequest, amount_of};

use std::collections::HashSet;
use std::sync::Arc;

use log::{error, info};
use nr_stat_alloc_shared::relic_reroll::RerollError;

use crate::data::{RelicData, relic_names};
use crate::game::currency::{self, Currency};
use crate::game::gaitem::Gaitems;
use crate::game::relic::{self, CACHE_SLOTS, RelicIns};
use crate::rng::XorShift;
use crate::tr;

pub struct RelicReroll {
    data: Arc<RelicData>,
    pricing: Pricing,
    gaitems: Gaitems,
    rng: XorShift,
}

impl RelicReroll {
    pub fn new(data: Arc<RelicData>, pricing: Pricing, gaitems: Gaitems) -> Self {
        Self { data, pricing, gaitems, rng: XorShift::from_time() }
    }

    /// hero 为角色序号（0 起）
    pub fn view(&self, game_data: usize, hero: usize) -> Option<RelicView> {
        let handles = relic::equipped(game_data)?;
        Some(RelicView {
            hero,
            slots: handles.map(|h| h.and_then(|h| self.read(h))),
            murk: currency::balance(game_data, Currency::Murk)?,
            sigils: currency::balance(game_data, Currency::Sigils)?,
        })
    }

    /// 返回 (结果说明, 新出现的词条)
    pub fn execute(&mut self, game_data: usize, hero: usize, req: &RerollRequest) -> Result<(String, Vec<u32>), String> {
        let stale = || tr!("遗物或角色已变化，请重新选择", "The relic or hero changed; please select again");
        let handle = relic::equipped(game_data).and_then(|h| h.get(req.slot).copied().flatten());
        if hero != req.hero || handle != Some(req.handle) {
            return Err(stale());
        }
        let ins = self.gaitems.relic(req.handle).map(RelicIns).ok_or_else(stale)?;
        let current = self.read(req.handle).ok_or_else(stale)?;
        if current.effects != req.effects {
            return Err(stale());
        }
        let catalog = &self.data.catalog;
        let cost = self.pricing.quote(catalog, &current, &req.targets, hero, &req.excluded).ok_or_else(|| tr!("所选分组里没有可抽的词条", "No effect can be drawn from the selected groups"))?;
        let amount = amount_of(cost, req.currency);
        if amount != req.amount {
            return Err(tr!("价格已变化，请重新确认", "The price changed; please confirm again"));
        }
        let have = currency::balance(game_data, req.currency).unwrap_or(0);
        if have < amount {
            return Err(match req.currency {
                Currency::Murk => tr!("暗痕不足（需要 {amount}，持有 {have}）", "Not enough Murk ({amount} needed, {have} held)"),
                Currency::Sigils => tr!("为王之证不足（需要 {amount}，持有 {have}）", "Not enough Sovereign Sigils ({amount} needed, {have} held)"),
            });
        }
        let excluded: HashSet<u32> = req.excluded.iter().copied().collect();
        let result = catalog
            .reroll(current.relic, &current.effects, &req.targets, hero as u32 + 1, &excluded, || self.rng.next())
            .map_err(describe)?;
        let cache = catalog.sp_effect_cache(&result.effects).filter(|c| c.len() <= CACHE_SLOTS).ok_or_else(|| tr!("词条数据异常，未重铸", "Effect data is invalid; nothing changed"))?;
        currency::spend(game_data, req.currency, amount).map_err(|e| {
            error!("relic reroll: spend {amount} {:?} failed: {e}", req.currency);
            tr!("扣费失败，未重铸", "Payment failed; nothing changed")
        })?;
        if !ins.write(&result.effects, &cache) {
            let refunded = currency::refund(game_data, req.currency, amount);
            error!("relic reroll: writing relic {:#x} failed (refunded: {refunded})", req.handle);
            return Err(tr!("写入遗物失败，费用已退还", "Writing the relic failed; the cost was refunded"));
        }
        info!(
            "relic reroll: hero {hero} slot {} relic {} ({:#x}) {:?} -> {:?}, paid {amount} {:?}",
            req.slot, current.relic, req.handle, current.effects, result.effects, req.currency
        );
        let names: Vec<String> = result.changes.iter().map(|&(old, new)| format!("{} → {}", self.effect_name(old), self.effect_name(new))).collect();
        let names = names.join(lang_sep());
        let paid = match req.currency {
            Currency::Murk => tr!("{amount} 暗痕", "{amount} Murk"),
            Currency::Sigils => tr!("{amount} 为王之证", "{amount} Sovereign Sigils"),
        };
        Ok((tr!("重铸完成：{names}（花费 {paid}）", "Rerolled: {names} (paid {paid})"), result.changes.iter().map(|c| c.1).collect()))
    }

    fn read(&self, handle: u32) -> Option<EquippedRelic> {
        let ins = RelicIns(self.gaitems.relic(handle)?);
        Some(EquippedRelic { handle, relic: ins.item_id()?, effects: ins.effects()? })
    }

    fn effect_name(&self, id: u32) -> String {
        self.data.catalog.effect(id).and_then(|e| relic_names::effect(e.text_id)).map_or_else(|| format!("#{id}"), |n| n.replace('\n', ""))
    }
}

fn lang_sep() -> &'static str {
    crate::lang::pick("；", "; ")
}

fn describe(e: RerollError) -> String {
    match e {
        RerollError::NotRerollable => tr!("这件遗物的词条是固定的，不能重铸", "This relic has fixed effects and cannot be rerolled"),
        RerollError::EmptySlot(_) => tr!("选中的词条已不存在，请重新选择", "A selected effect no longer exists; please select again"),
        RerollError::NoCandidates(_) => tr!("所选分组里没有可抽的词条", "No effect can be drawn from the selected groups"),
    }
}
