//! 遗物重铸用的数据（规则见 shared relic_reroll），与 GameData 同一次读取参数表时构建；缺表时为 None，只停用重铸。

use std::collections::HashMap;

use log::{info, warn};
use nr_stat_alloc_shared::param::ParamFile;
use nr_stat_alloc_shared::param::layout::{Table, attach_effect, attach_effect_filter, attach_effect_filter_sub, attach_effect_table, equip_antique, shop_lineup};
use nr_stat_alloc_shared::relic_reroll::{Prices, RelicCatalog};

/// 除武器功能已读的 AttachEffect / 抽奖表之外还要读的表
pub const RELIC_TABLES: [Table; 4] = [equip_antique::TABLE, attach_effect_filter::TABLE, attach_effect_filter_sub::TABLE, shop_lineup::TABLE];

pub struct RelicData {
    pub catalog: RelicCatalog,
    /// 商店里情景原石的价格（配置可覆盖）
    pub shop_prices: Prices,
}

/// custom：本模组新增的追踪者词条随模板词条出现
pub fn build(tables: &HashMap<String, Vec<u8>>, custom: bool) -> Option<RelicData> {
    let parse = |t: Table| {
        let bytes = tables.get(t.struct_name)?;
        ParamFile::parse(bytes, t.row_size).map_err(|e| warn!("{}: {e}", t.struct_name)).ok()
    };
    let data = (|| {
        let catalog = RelicCatalog::build(
            &parse(equip_antique::TABLE)?,
            &parse(attach_effect::TABLE)?,
            &parse(attach_effect_table::TABLE)?,
            &parse(attach_effect_filter::TABLE)?,
            &parse(attach_effect_filter_sub::TABLE)?,
            custom,
        );
        Some(RelicData { catalog, shop_prices: Prices::from_shop(&parse(shop_lineup::TABLE)?) })
    })();
    match &data {
        Some(d) => info!("relic reroll data ready, shop prices {:?}", d.shop_prices),
        None => warn!("relic reroll unavailable (tables missing or unexpected layout)"),
    }
    data
}
