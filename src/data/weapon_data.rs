//! 武器相关功能（强化词条升档、初始武器互换、出击遗物开关与随机词条、词条继承）用到的数据，与 GameData 同一次读取参数表时构建。
//! 某张表缺失或结构不符时对应部分为 None，只停用该功能。

use std::collections::HashMap;

use log::{info, warn};
use nr_stat_alloc_shared::loadouts::Loadouts;
use nr_stat_alloc_shared::passive_inherit::Rarities;
use nr_stat_alloc_shared::param::ParamFile;
use nr_stat_alloc_shared::param::layout::{Table, attach_effect, attach_effect_table, chara_init, custom_weapon, equip_weapon, sp_effect, sword_arts};
use nr_stat_alloc_shared::passive_pool::PassivePool;
use nr_stat_alloc_shared::passive_tiers::PassiveTiers;
use nr_stat_alloc_shared::start_relics::StartRelics;
use nr_stat_alloc_shared::weapons::WeaponInfo;

use crate::game::params::ParamRepository;

use super::InheritData;

/// 除 GameData 必需的表之外还要读的表
pub const EXTRA_TABLES: [Table; 5] = [attach_effect::TABLE, equip_weapon::TABLE, custom_weapon::TABLE, attach_effect_table::TABLE, sword_arts::TABLE];

pub struct WeaponData {
    pub passive_tiers: Option<PassiveTiers>,
    pub starting: Option<StartingData>,
    /// 词条继承的规则与面板数据
    pub inherit: Option<InheritData>,
    /// 武器当前稀有度（三档锻体的金色武器计数、升紫用）
    pub rarities: Option<Rarities>,
}

pub struct StartingData {
    pub weapons: WeaponInfo,
    pub loadouts: Loadouts,
    pub pool: PassivePool,
    /// CharaInitParam 在游戏内存中的地址（初始武器互换直接改它）
    pub chara_init_address: usize,
    /// 出击时改战技 / 改魔法 / 附加类遗物的角色开关；缺表时为 None，只是这类遗物不跟着武器走
    pub relics: Option<RelicData>,
}

pub struct RelicData {
    pub gates: StartRelics,
    /// AttachEffectParam / SwordArtsParam 在游戏内存中的地址
    pub attach_address: usize,
    pub sword_arts_address: usize,
}

pub fn build(tables: &HashMap<String, Vec<u8>>, repository: &ParamRepository) -> WeaponData {
    let attach = parse(tables, attach_effect::TABLE);
    let starting = (|| {
        let weapons = WeaponInfo::build(&parse(tables, equip_weapon::TABLE)?);
        let pool = PassivePool::build(attach.as_ref()?, &parse(tables, attach_effect_table::TABLE)?, &parse(tables, custom_weapon::TABLE)?, &weapons);
        let loadouts = Loadouts::build(&parse(tables, chara_init::TABLE)?);
        let chara_init_address = repository.table_address(chara_init::TABLE.struct_name)?;
        let relics = relic_data(tables, attach.as_ref()?, repository);
        Some(StartingData { weapons, loadouts, pool, chara_init_address, relics })
    })();
    if starting.is_none() {
        warn!("starting weapon features unavailable (tables missing or unexpected layout)");
    }
    let inherit = parse(tables, equip_weapon::TABLE).and_then(|w| InheritData::build(&w, attach.as_ref()?));
    let rarities = parse(tables, equip_weapon::TABLE).map(|w| Rarities::build(&w)).filter(|r| !r.is_empty());
    WeaponData { passive_tiers: attach.as_ref().map(PassiveTiers::build), starting, inherit, rarities }
}

fn relic_data(tables: &HashMap<String, Vec<u8>>, attach: &ParamFile, repository: &ParamRepository) -> Option<RelicData> {
    let data = (|| {
        let gates = StartRelics::build(attach, &parse(tables, sp_effect::TABLE)?, &parse(tables, sword_arts::TABLE)?);
        let attach_address = repository.table_address(attach_effect::TABLE.struct_name)?;
        let sword_arts_address = repository.table_address(sword_arts::TABLE.struct_name)?;
        Some(RelicData { gates, attach_address, sword_arts_address })
    })();
    match &data {
        Some(d) => info!("start-weapon relics: {} hero gates", d.gates.len()),
        None => warn!("start-weapon relic gates unavailable; skill / sorcery / imbue relics stay tied to vanilla heroes"),
    }
    data
}

fn parse(tables: &HashMap<String, Vec<u8>>, t: Table) -> Option<ParamFile<'_>> {
    let bytes = tables.get(t.struct_name)?;
    ParamFile::parse(bytes, t.row_size).map_err(|e| warn!("{}: {e}", t.struct_name)).ok()
}
