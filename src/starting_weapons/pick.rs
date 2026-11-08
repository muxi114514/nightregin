//! 方案里的初始武器选择：解析成武器 ID、按初始装备规则校验，以及面板可选的武器列表。
//! 可选武器 = 各角色的默认主手与副手（写法："角色名" / "角色名副手"），与配置写法一致。

use crate::data::StartingData;
use crate::stats::heroes::{HERO_COUNT, HERO_NAMES_ZH};
use crate::stats::rules::WeaponPick;

use super::choice;

/// (主手, 副手) 本体 ID
pub type Resolved = (u32, Option<u32>);

/// 可选的一把初始武器
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PoolItem {
    /// 原主人（角色序号）
    pub owner: usize,
    pub off_hand: bool,
    pub weapon: u32,
}

impl PoolItem {
    /// 写进方案的写法
    pub fn token(&self) -> String {
        let name = HERO_NAMES_ZH[self.owner];
        if self.off_hand { format!("{name}副手") } else { name.to_string() }
    }
}

/// 全部可选武器：按角色顺序，主手在前
pub fn pool(data: &StartingData) -> Vec<PoolItem> {
    (0..HERO_COUNT)
        .filter_map(|owner| Some((owner, *data.loadouts.defaults.get(&(owner as u32 + 1))?)))
        .flat_map(|(owner, (main, off))| {
            let main = (main > 0).then_some(PoolItem { owner, off_hand: false, weapon: main as u32 });
            let off = (off > 0).then_some(PoolItem { owner, off_hand: true, weapon: off as u32 });
            main.into_iter().chain(off)
        })
        .collect()
}

/// 该角色原本是否有副手（决定能不能选两把）
pub fn has_off_hand(data: &StartingData, hero: usize) -> bool {
    data.loadouts.defaults.get(&(hero as u32 + 1)).is_some_and(|&(_, off)| off > 0)
}

/// 选择 -> 实际的 (主手, 副手)；没写的一手取原版
pub fn resolve(data: &StartingData, hero: usize, pick: &WeaponPick) -> Result<Resolved, String> {
    let defaults = &data.loadouts.defaults;
    let &(own_main, own_off) = defaults.get(&(hero as u32 + 1)).ok_or("no skin rows")?;
    let main = match &pick.main {
        Some(token) => choice::resolve(token, defaults)?.ok_or("main hand cannot be empty")?,
        None => own_main as u32,
    };
    let off = match &pick.off {
        Some(token) => choice::resolve(token, defaults)?,
        None => (own_off > 0).then_some(own_off as u32),
    };
    Ok((main, off))
}

/// 按初始装备规则校验（武器存在、数量不超过原版、不能两把近战）
pub fn validate(data: &StartingData, hero: usize, pick: &WeaponPick) -> Result<Resolved, String> {
    let (main, off) = resolve(data, hero, pick)?;
    data.loadouts.plan(hero as u32 + 1, main, off, &data.weapons)?;
    Ok((main, off))
}

#[cfg(test)]
mod tests {
    use nr_stat_alloc_shared::loadouts::{Loadouts, SkinRow};
    use nr_stat_alloc_shared::passive_pool::PassivePool;
    use nr_stat_alloc_shared::weapons::WeaponInfo;

    use super::*;

    fn data() -> StartingData {
        let skin = |id, hero_id, main, off| SkinRow { id, offset: id as usize, hero_id, main, off };
        StartingData {
            weapons: WeaponInfo::from_rows([(3750000, 5, 10102), (30750000, 65, 12021), (1750000, 1, 10017), (32750000, 69, 12109)]),
            loadouts: Loadouts::from_skins(vec![
                skin(50000, 1, 3750000, 30750000),
                skin(50300, 4, 1750000, -1),
                skin(50100, 2, 18750000, 32750000),
            ]),
            pool: PassivePool::default(),
            chara_init_address: 0,
            relics: None,
        }
    }

    #[test]
    fn pool_lists_mains_then_off_hands() {
        let p = pool(&data());
        let tokens: Vec<String> = p.iter().map(PoolItem::token).collect();
        assert_eq!(tokens, ["追踪者", "追踪者副手", "守护者", "守护者副手", "女爵"]);
        assert!(has_off_hand(&data(), 0) && !has_off_hand(&data(), 3));
    }

    #[test]
    fn validates_against_starting_rules() {
        let d = data();
        let pick = |main: Option<&str>, off: Option<&str>| WeaponPick { main: main.map(Into::into), off: off.map(Into::into) };
        // 女爵换成追踪者大剑
        assert_eq!(validate(&d, 3, &pick(Some("追踪者"), None)), Ok((3750000, None)));
        // 女爵不能加副手
        assert!(validate(&d, 3, &pick(None, Some("守护者副手"))).is_err());
        // 追踪者：女爵短剑 + 自己的小盾可以，女爵短剑 + 大剑不行
        assert_eq!(validate(&d, 0, &pick(Some("女爵"), None)), Ok((1750000, Some(30750000))));
        assert!(validate(&d, 0, &pick(Some("女爵"), Some("追踪者"))).is_err());
        // 没写的一手取原版
        assert_eq!(resolve(&d, 0, &WeaponPick::default()), Ok((3750000, Some(30750000))));
    }
}
