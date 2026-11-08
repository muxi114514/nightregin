//! 用本机的游戏文件做对照测试（文件不存在时跳过）：原版往返逐字节一致、与旧 SoulsFormats 脚本打出的补丁一致、
//! 重复打补丁结果不变、从原版提取的数据正确。

use std::path::Path;

use nr_stat_alloc_shared::extract::{Tables, extract};
use nr_stat_alloc_shared::loadouts::Loadouts;
use nr_stat_alloc_shared::param::layout::{
    Table, antique_stand, attach_effect, attach_effect_table, chara_init, correct_graph, custom_weapon, equip_weapon, hero, hero_status,
    item_lot, item_table, permanent_buff, sp_effect, sword_arts,
};
use nr_stat_alloc_shared::boss_loot::LotEdit;
use nr_stat_alloc_shared::orb_refresh::{OptionSlot, Pools};
use nr_stat_alloc_shared::passive_inherit::Rarities;
use nr_stat_alloc_shared::passive_pool::PassivePool;
use nr_stat_alloc_shared::passive_tiers::PassiveTiers;
use nr_stat_alloc_shared::weapons::WeaponInfo;

use nr_stat_alloc_shared::param::ParamFile;

use crate::regulation::Regulation;
use crate::{bnd4, crypto, dcx, param_write};

const VANILLA: &str = "Game/regulation.bin";
const MORE_MAP: &str = "me3/NightreignPLUS/More Map and Weapons/regulation.bin";
const DEPLOYED: &str = "me3/NightreignPLUS/StatAlloc/pkg/regulation.bin";
const KONG_CHENG: &str = "me3/MMV/me3/modKongCheng/regulation.bin";
/// 空城 0.46 独立包（不叠在 MMV 上）
const KONG_CHENG_046: &str = "me3/kongcheng/mod/regulation.bin";
/// 带三档锻体的 MMV 包
const MMV_TIERED: &str = "me3/MMV/me3/StatAlloc/pkg_mmv/regulation.bin";

fn load(relative: &str) -> Option<Vec<u8>> {
    std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(relative)).ok()
}

fn extract_from(reg: &Regulation) -> nr_stat_alloc_shared::extract::Extracted {
    let files = [
        (sp_effect::TABLE, "SpEffectParam"),
        (hero::TABLE, "HeroParam"),
        (hero_status::TABLE, "HeroStatusParam"),
        (chara_init::TABLE, "CharaInitParam"),
        (antique_stand::TABLE, "AntiqueStandParam"),
        (correct_graph::TABLE, "CalcCorrectGraph"),
    ];
    let tables = Tables::load(|t| {
        files.iter().find(|(f, _)| f.struct_name == t.struct_name).and_then(|(_, name)| reg.param(name))
    })
    .unwrap();
    extract(&tables).unwrap()
}

#[test]
fn vanilla_round_trips_byte_exact() {
    let Some(file) = load(VANILLA) else { return };
    let (_, raw) = dcx::Dcx::decompress(&crypto::decrypt(&file).unwrap()).unwrap();
    let bnd = bnd4::Bnd4::parse(raw.clone()).unwrap();
    let (index, data) = bnd.find("\\SpEffectParam.param").unwrap();
    let table = ParamFile::parse(data, sp_effect::TABLE.row_size).unwrap();
    assert!(param_write::write(&table, &param_write::rows_of(&table)).unwrap() == data);
    assert!(bnd.rebuild(index, data) == raw);
}

#[test]
fn patch_matches_soulsformats_and_is_idempotent() {
    let (Some(base), Some(deployed)) = (load(MORE_MAP), load(DEPLOYED)) else { return };
    let (patched, report) = Regulation::decode(&base).unwrap().patch().unwrap();
    assert_eq!((report.added, report.replaced, report.foreign), (2034, 0, 0));
    let ours = Regulation::decode(&patched).unwrap();
    assert!(ours.sp_effect().unwrap() == Regulation::decode(&deployed).unwrap().sp_effect().unwrap());

    let (again, report) = ours.patch().unwrap();
    assert_eq!((report.replaced, report.foreign, report.rows_after), (2034, 0, report.rows_before));
    assert!(Regulation::decode(&again).unwrap().sp_effect().unwrap() == ours.sp_effect().unwrap());
}

#[test]
fn extracts_vanilla_data_and_detects_components() {
    let Some(file) = load(VANILLA) else { return };
    let vanilla = Regulation::decode(&file).unwrap();
    let d = extract_from(&vanilla);
    assert_eq!(d.heroes.len(), 10);
    assert_eq!(d.heroes[7].levels, [1, 2, 12, 15]);
    assert_eq!(d.heroes[7].stats[0], [7, 2, 3, 5, 8, 2, 2, 28]);
    assert_eq!(d.vessels.len(), 74);
    assert_eq!((d.vessels[0].id, d.vessels[0].hero_type, d.vessels[0].goods_id), (1000, 1, 9600));
    assert!(d.chara_hero.contains(&(50710, 8)) && d.chara_hero.contains(&(50050, 1)));
    assert_eq!(d.replacement, [46260, 46261, 46262, 46263, 46264]);
    assert_eq!(d.modifier.len(), 20);
    assert_eq!(d.curves.unwrap()[0].stage_max_grow_val, [100.0, 580.0, 1080.0, 1580.0, 2060.0]);
    assert!(!d.components_ready && d.lease_seconds.is_none());

    let (patched, _) = vanilla.patch().unwrap();
    let d = extract_from(&Regulation::decode(&patched).unwrap());
    assert!(d.components_ready);
    assert_eq!(d.lease_seconds, Some(6.0));
}

#[test]
fn passive_tiers_follow_vanilla_potency_chains() {
    for path in [VANILLA, KONG_CHENG] {
        let Some(file) = load(path) else { continue };
        let reg = Regulation::decode(&file).unwrap();
        let table = ParamFile::parse(reg.param("AttachEffectParam").unwrap(), attach_effect::TABLE.row_size).unwrap();
        assert_eq!(table.struct_name(), Some(attach_effect::TABLE.struct_name));
        let t = PassiveTiers::build(&table);
        // 血量全满时提升攻击力：+7% → +10.5% → +14%
        assert_eq!((t.next(8670000), t.next(8670001), t.next(8670002)), (Some(8670001), Some(8670002), None));
        // 步长 +10 的链与同文本的第二条子链
        assert_eq!(t.upgrade(8320500, 3), 8320520);
        assert_eq!((t.next(8110002), t.next(8110010)), (None, Some(8110011)));
        // 特殊变体（显示值 26）不是 8110700 的上位
        assert_eq!((t.next(8110700), t.next(8110703)), (Some(8110701), None));
        // 负面词条：降低血量上限 -4% → -6%（红色武器的档位）
        assert_eq!(t.next(8760000), Some(8760050));
        // 深夜专属：紫色 +8 → 红色 +12
        assert_eq!(t.next(8850000), Some(8850050));
        assert!(t.is_known(8670000) && !t.is_known(1010000));
    }
}

fn table<'a>(reg: &'a Regulation, file: &str, t: Table) -> ParamFile<'a> {
    let p = ParamFile::parse(reg.param(file).unwrap(), t.row_size).unwrap();
    assert_eq!(p.struct_name(), Some(t.struct_name), "{file}");
    p
}

/// 文件中相邻两行数据的最小间距，即实际行大小
fn stride(p: &ParamFile) -> usize {
    let mut offsets: Vec<usize> = p.rows().map(|r| r.offset).collect();
    offsets.sort();
    offsets.windows(2).map(|w| w[1] - w[0]).min().unwrap()
}

#[test]
fn starting_loadouts_and_passive_pool_match_regulation() {
    for path in [VANILLA, KONG_CHENG] {
        let Some(file) = load(path) else { continue };
        let reg = Regulation::decode(&file).unwrap();
        let weapons = WeaponInfo::build(&table(&reg, "EquipParamWeapon", equip_weapon::TABLE));
        // 大剑 5、小盾 65、弓 51、圣印记 61、法杖 57
        assert_eq!(
            [3750000, 30750000, 41750000, 34750000, 33750000].map(|id| weapons.wep_type(id)),
            [Some(5), Some(65), Some(51), Some(61), Some(57)]
        );
        // 菜单图标编号（MENU_ItemIcon_xxxxx）：大剑、执行者的刀、送葬者的铁锤
        assert_eq!([3750000, 9750000, 11750000].map(|id| weapons.icon(id)), [Some(10102), Some(10329), Some(10415)]);
        let l = Loadouts::build(&table(&reg, "CharaInitParam", chara_init::TABLE));
        assert_eq!(l.defaults[&1], (3750000, 30750000));
        assert_eq!(l.defaults[&2], (18750000, 32750000));
        assert_eq!(l.defaults[&6], (21750000, 34750000));
        assert_eq!(l.defaults[&8], (9750000, -1));
        // 换武器后每个皮肤保留原强化等级（空城包 50721 为 +2 皮肤 9750002）
        let edits = l.plan(8, 3750000, None, &weapons).unwrap();
        for s in l.skins.iter().filter(|s| s.hero_id == 8) {
            let main = edits.iter().find(|e| e.offset == s.offset + chara_init::WEAPON_RIGHT_1).unwrap();
            assert_eq!(main.value, 3750000 + s.main % 100, "skin {}", s.id);
        }
        // 女爵（单武器）只能换成一把：追踪者大剑可以，再加副手不行
        assert!(l.plan(4, 3750000, None, &weapons).is_ok());
        assert!(l.plan(4, 3750000, Some(30750000), &weapons).is_err());
        // 守护者（两把）：执行者太刀 + 追踪者大剑不行，铁之眼弓 + 追踪者大剑可以
        assert!(l.plan(2, 9750000, Some(3750000), &weapons).is_err());
        assert!(l.plan(2, 41750000, Some(3750000), &weapons).is_ok());

        let pool = PassivePool::build(
            &table(&reg, "AttachEffectParam", attach_effect::TABLE),
            &table(&reg, "AttachEffectTableParam", attach_effect_table::TABLE),
            &table(&reg, "EquipParamCustomWeapon", custom_weapon::TABLE),
            &weapons,
        );
        let families = [3750000, 30750000, 33750000, 34750000, 41750000].map(|id| pool.table_for(id, &weapons));
        assert_eq!(families, [Some(501000000), Some(501300000), Some(501400000), Some(501500000), Some(501600000)]);
        for hero in 1..=10 {
            for seed in [0u64, 12345, 987654321] {
                let id = pool.roll(3750000, &weapons, hero, seed).unwrap();
                assert!((8000000..9000000).contains(&id), "hero {hero} rolled {id}");
            }
        }
    }
}

/// 宝箱怪两个选项按真实参数表抽：第一选项只有本角色专属，第二选项紫 / 专属各半
#[test]
fn orb_refresh_draws_mimic_options_from_their_pools() {
    const EXECUTOR: u8 = 8;
    const WYLDER: u8 = 1;
    const EXECUTOR_BUFF: u32 = 9_971_760;
    const PURPLE_BUFF: u32 = 8_650_100;
    const ROLE_LOT: u32 = 6_900_000;
    const MIXED_LOT: u32 = 6_900_001;
    const DRAWS: u32 = 4000;
    let Some(file) = load(MMV_TIERED) else { return };
    let reg = Regulation::decode(&file).unwrap();
    let lots = [table(&reg, "ItemLotParam_enemy", item_lot::TABLE), table(&reg, "ItemLotParam_map", item_lot::TABLE)];
    let pools = Pools::build(
        &lots,
        &table(&reg, "ItemTableParam", item_table::TABLE),
        &table(&reg, "PermanentBuffParam", permanent_buff::TABLE),
        &table(&reg, "SpEffectParam", sp_effect::TABLE),
    );
    let slot = |lot, buff| OptionSlot { lot, buff };
    assert!(pools.is_buff(EXECUTOR_BUFF) && pools.is_buff(PURPLE_BUFF));
    assert!(pools.drops(slot(ROLE_LOT, EXECUTOR_BUFF), EXECUTOR));
    assert!(!pools.drops(slot(ROLE_LOT, EXECUTOR_BUFF), WYLDER), "other heroes must not draw it");
    assert!(!pools.drops(slot(ROLE_LOT, PURPLE_BUFF), EXECUTOR));
    assert!(pools.drops(slot(MIXED_LOT, PURPLE_BUFF), EXECUTOR));
    assert!(pools.drops(slot(MIXED_LOT, EXECUTOR_BUFF), EXECUTOR));
    let mut state = 0x9E37_79B9_7F4A_7C15u64;
    let mut random = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let mut role = 0;
    for _ in 0..DRAWS {
        let plan = pools.plan(&[slot(ROLE_LOT, EXECUTOR_BUFF), slot(MIXED_LOT, PURPLE_BUFF)], &[], EXECUTOR, &[], &mut random);
        let (first, second) = (plan[0].unwrap(), plan[1].unwrap());
        assert!(pools.drops(slot(ROLE_LOT, first), EXECUTOR), "first option must stay role-only, got {first}");
        assert!(first != EXECUTOR_BUFF && second != first && second != PURPLE_BUFF);
        role += pools.drops(slot(ROLE_LOT, second), EXECUTOR) as u32;
    }
    assert!((DRAWS * 45 / 100..DRAWS * 55 / 100).contains(&role), "second option drew {role} / {DRAWS} role buffs");
}

#[test]
fn boss_loot_third_choice_gets_smithing_stone() {
    use nr_stat_alloc_shared::boss_loot::{SMITHING_STONE_3, WENDING_GRACE, entries, plan};
    for path in [VANILLA, KONG_CHENG] {
        let Some(file) = load(path) else { continue };
        let reg = Regulation::decode(&file).unwrap();
        let enemy = table(&reg, "ItemLotParam_enemy", item_lot::TABLE);
        let edits = plan(&enemy, 10);
        if path == KONG_CHENG {
            assert_eq!(edits.len(), 55);
        }
        assert!(!edits.is_empty(), "{path}");
        // 按改写结果重建每行，核对：锻造石 10%、徘徊赐福不变、合计仍 1000
        let mut bytes = enemy.bytes().to_vec();
        for (_, row_edits) in &edits {
            for e in row_edits {
                match *e {
                    LotEdit::I32(o, v) => bytes[o..o + 4].copy_from_slice(&v.to_le_bytes()),
                    LotEdit::U16(o, v) => bytes[o..o + 2].copy_from_slice(&v.to_le_bytes()),
                    LotEdit::U8(o, v) => bytes[o] = v,
                }
            }
        }
        let after = ParamFile::parse(&bytes, item_lot::TABLE.row_size).unwrap();
        for (id, _) in &edits {
            let e = entries(after.row(*id).unwrap()).unwrap();
            let weight = |item: i32| e.iter().filter(|x| x.category == 1 && x.id == item).map(|x| x.weight).sum::<u16>();
            assert_eq!((weight(SMITHING_STONE_3), weight(WENDING_GRACE)), (100, 50), "row {id}");
            assert_eq!(e.iter().map(|x| x.weight as u32).sum::<u32>(), 1000, "row {id}");
        }
        // 地图掉落表没有这种行；改过的表再跑一次不再改
        assert!(plan(&table(&reg, "ItemLotParam_map", item_lot::TABLE), 10).is_empty());
        assert!(plan(&after, 10).is_empty());
    }
}

#[test]
fn start_weapon_relics_follow_swapped_weapons() {
    use std::collections::BTreeMap;

    use nr_stat_alloc_shared::start_relics::{GateTable, Kind, StartRelics, hero_mask};
    const UNDERTAKER: u16 = 1 << 9;
    for path in [VANILLA, KONG_CHENG, KONG_CHENG_046] {
        let Some(file) = load(path) else { continue };
        let reg = Regulation::decode(&file).unwrap();
        let relics = StartRelics::build(
            &table(&reg, "AttachEffectParam", attach_effect::TABLE),
            &table(&reg, "SpEffectParam", sp_effect::TABLE),
            &table(&reg, "SwordArtsParam", sword_arts::TABLE),
        );
        let weapons = WeaponInfo::build(&table(&reg, "EquipParamWeapon", equip_weapon::TABLE));
        let defaults = Loadouts::build(&table(&reg, "CharaInitParam", chara_init::TABLE)).defaults;
        // 改祷告雷电枪只开复仇者、箭雨只开铁之眼、改魔法辉剑圆阵只开隐士
        assert_eq!(relics.gate(GateTable::AttachEffect, 7370400), Some((Kind::Magic, 1 << 5)));
        assert_eq!(relics.gate(GateTable::AttachEffect, 7124700), Some((Kind::Skill, 1 << 2)));
        assert_eq!(relics.gate(GateTable::AttachEffect, 7360600), Some((Kind::Magic, 1 << 6)));
        assert_eq!(relics.gate(GateTable::SwordArts, 406), Some((Kind::Skill, 1 << 2)));
        // 血刃：女爵、执行者、学者，两张表一致（核对 isStartSkill 的位序）
        let blood = (1 << 3) | (1 << 7) | (1 << 8);
        assert_eq!(relics.gate(GateTable::AttachEffect, 7123700), Some((Kind::Skill, blood)));
        assert_eq!(relics.gate(GateTable::SwordArts, 224), Some((Kind::Skill, blood)));
        // 出击时的武器附加（7 种）：隐士以外都开
        let no_recluse = 0x3FF & !(1 << 6);
        for id in (7120000..=7120600).step_by(100) {
            assert_eq!(relics.gate(GateTable::AttachEffect, id), Some((Kind::Skill, no_recluse)), "{path} {id}");
        }

        // 各角色带自己的原版武器时与原版逐字节相同
        let own: BTreeMap<u32, Vec<u32>> =
            defaults.iter().map(|(&h, &(m, o))| (h, [m, o].into_iter().filter(|&w| w > 0).map(|w| w as u32).collect())).collect();
        assert!(relics.plan(&own, &defaults, &weapons).iter().all(|e| e.value == e.original), "{path}");

        let apply = |carried: BTreeMap<u32, Vec<u32>>| {
            let mut attach = reg.param("AttachEffectParam").unwrap().to_vec();
            let mut arts = reg.param("SwordArtsParam").unwrap().to_vec();
            for e in relics.plan(&carried, &defaults, &weapons) {
                match e.table {
                    GateTable::AttachEffect => attach[e.offset] = e.value,
                    GateTable::SwordArts => arts[e.offset] = e.value,
                }
            }
            (attach, arts)
        };
        let mask = |bytes: &[u8], t: Table, gt: GateTable, id: u32| hero_mask(gt, ParamFile::parse(bytes, t.row_size).unwrap().row(id).unwrap()).unwrap();
        // 送葬者拿复仇者的圣印记：雷电枪生效，原本可用的近战战技（寒气冻雾）关掉
        let (attach, _) = apply(BTreeMap::from([(10, vec![34750000])]));
        assert_eq!(mask(&attach, attach_effect::TABLE, GateTable::AttachEffect, 7370400), (1 << 5) | UNDERTAKER);
        assert_eq!(mask(&attach, attach_effect::TABLE, GateTable::AttachEffect, 7123900) & UNDERTAKER, 0);
        // 送葬者拿铁之眼的弓：箭雨两张表都开
        let (attach, arts) = apply(BTreeMap::from([(10, vec![41750000])]));
        assert_eq!(mask(&attach, attach_effect::TABLE, GateTable::AttachEffect, 7124700), (1 << 2) | UNDERTAKER);
        assert_eq!(mask(&arts, sword_arts::TABLE, GateTable::SwordArts, 406), (1 << 2) | UNDERTAKER);
        // 送葬者拿隐士的法杖：改魔法生效，附加关掉
        let (attach, _) = apply(BTreeMap::from([(10, vec![33750000])]));
        assert_eq!(mask(&attach, attach_effect::TABLE, GateTable::AttachEffect, 7360600), (1 << 6) | UNDERTAKER);
        assert_eq!(mask(&attach, attach_effect::TABLE, GateTable::AttachEffect, 7120100), no_recluse & !UNDERTAKER);
        // 隐士拿追踪者的大剑：附加生效
        let (attach, _) = apply(BTreeMap::from([(7, vec![3750000])]));
        assert_eq!(mask(&attach, attach_effect::TABLE, GateTable::AttachEffect, 7120100), 0x3FF);
        // 守护者拿复仇者的圣印记 + 自己的盾：雷电枪生效（0.46 送葬者也默认带这把，不能顶掉复仇者）
        let (attach, _) = apply(BTreeMap::from([(2, vec![34750000, 32750000])]));
        assert_eq!(mask(&attach, attach_effect::TABLE, GateTable::AttachEffect, 7370400), (1 << 5) | (1 << 1), "{path}");
        if path == KONG_CHENG_046 {
            assert_eq!(defaults[&10], (11750000, 34750000));
            // 送葬者圣印记主手 + 追踪者小盾（用户实测组合）
            let (attach, _) = apply(BTreeMap::from([(10, vec![34750000, 30750000])]));
            assert_eq!(mask(&attach, attach_effect::TABLE, GateTable::AttachEffect, 7370400), (1 << 5) | UNDERTAKER);
        }
    }
}

#[test]
fn relic_reroll_rules_match_regulation() {
    use nr_stat_alloc_shared::param::layout::{attach_effect_filter, attach_effect_filter_sub, equip_antique, shop_lineup};
    use nr_stat_alloc_shared::relic_reroll::{DEFAULT_PRICES, GroupKind, Prices, RelicCatalog, RelicEffects, Target};
    /// 整合包实际加载的 regulation（含本模组的追踪者自定义词条）
    const MMV_PKG: &str = "me3/MMV/me3/StatAlloc/pkg/regulation.bin";
    const WYLDER: u32 = 1;
    for path in [VANILLA, KONG_CHENG, MMV_PKG] {
        let Some(file) = load(path) else { continue };
        let reg = Regulation::decode(&file).unwrap();
        let antique = table(&reg, "EquipParamAntique", equip_antique::TABLE);
        let filter = table(&reg, "AttachEffectFilterParam", attach_effect_filter::TABLE);
        let sub = table(&reg, "AttachEffectFilterSubCategoryParam", attach_effect_filter_sub::TABLE);
        let shop = table(&reg, "ShopLineupParam", shop_lineup::TABLE);
        for p in [&antique, &filter, &sub, &shop] {
            assert_eq!(stride(p), p.row_size(), "{path} {:?}", p.struct_name());
        }
        let c = RelicCatalog::build(
            &antique,
            &table(&reg, "AttachEffectParam", attach_effect::TABLE),
            &table(&reg, "AttachEffectTableParam", attach_effect_table::TABLE),
            &filter,
            &sub,
            true,
        );
        assert_eq!(Prices::from_shop(&shop), DEFAULT_PRICES, "{path}");

        // 分组：15 个通用、10 个角色、3 个减益；武器原版 32 种，整合包加了新武器类别
        let count = |f: fn(&GroupKind) -> bool| c.groups().filter(|g| f(&g.kind)).count();
        let counts = [count(|k| *k == GroupKind::General), count(|k| matches!(k, GroupKind::Hero(_))), count(|k| *k == GroupKind::Debuff)];
        assert_eq!(counts, [15, 10, 3], "{path}");
        let weapons = count(|k| *k == GroupKind::Weapon);
        assert!(if path == VANILLA { weapons == 32 } else { weapons >= 32 }, "{path}: {weapons}");

        // 遗物：辽阔情景（三栏随机）可重铸，深夜标志正确，固定词条遗物不能重铸
        assert!(c.rerollable(202) && !c.relic(202).unwrap().deep && c.relic(2000000).unwrap().deep);
        assert!(!c.rerollable(6001400), "{path}");

        // 追踪者重铸普通遗物的生命力 +1：通用组不按角色过滤（改魔法、箭雨、附加都在），角色组只有追踪者的
        let e = RelicEffects { positive: [Some(7000000), None, None], ..Default::default() };
        let cands = c.candidates(202, &e, Target::Positive(0), WYLDER, &[]);
        let has = |id: u32| cands.iter().any(|x| x.effect == id);
        assert!(has(7360600) && has(7124700) && has(7120100), "{path}");
        let heroes: Vec<GroupKind> = cands.iter().filter_map(|x| c.group(x.group)).map(|g| g.kind).filter(|k| matches!(k, GroupKind::Hero(_))).collect();
        assert!(!heroes.is_empty() && heroes.iter().all(|k| *k == GroupKind::Hero(WYLDER as u8)), "{path}");
        // 原词条不再出；它被换掉了，同类的生命力 +2 可以出
        assert!(!has(7000000) && has(7000001), "{path}");
        // 自定义追斩词条只在加了本模组词条的 regulation 里随模板出现
        assert_eq!(has(7020050), path == MMV_PKG, "{path}");

        // 深夜遗物：强力栏只有通用组，负面栏只有减益组
        let e = RelicEffects { positive: [Some(6001400), None, None], curse: [Some(6820000), None, None] };
        let kinds = |t| {
            let mut k: Vec<GroupKind> = c.candidates(2000000, &e, t, WYLDER, &[]).iter().filter_map(|x| c.group(x.group)).map(|g| g.kind).collect();
            k.dedup();
            k
        };
        assert_eq!(kinds(Target::Positive(0)), [GroupKind::General], "{path}");
        assert_eq!(kinds(Target::Curse(0)), [GroupKind::Debuff], "{path}");

        // SpEffect 缓存：与实机内存里同词条遗物（MMV，2026-10-27 读取）的 +0x5C 内容一致
        if path == MMV_PKG {
            let e = RelicEffects { positive: [Some(7005600), Some(7240000), Some(6001401)], curse: [None, None, Some(6850900)] };
            assert_eq!(c.sp_effect_cache(&e), Some(vec![7005600, 7240000, 7240003, 7240006, 7001404, 6850900]));
            let e = RelicEffects { positive: [Some(6500100), Some(6005601), Some(6610400)], curse: [None, Some(6850800), Some(6850900)] };
            assert_eq!(c.sp_effect_cache(&e), Some(vec![7500100, 7005602, 7610400, 6850800, 8800201, 8800230, 6850900]));
        }
    }
}

#[test]
fn inherit_tiers_match_regulation() {
    for path in [VANILLA, KONG_CHENG] {
        let Some(file) = load(path) else { continue };
        let reg = Regulation::decode(&file).unwrap();
        let weapons = table(&reg, "EquipParamWeapon", equip_weapon::TABLE);
        let rarities = Rarities::build(&weapons);
        // 普通长剑可强化到传说；稀有度只有 0~3（另有一行占位值 9）
        assert_eq!([2000000, 2000001, 2000002, 2000003].map(|id| rarities.current(id)), [Some(0), Some(1), Some(2), Some(3)], "{path}");
        // 空武器槽里的空手条目不能参与继承
        assert_eq!(rarities.current(110000), None, "{path}");
        let mut seen = [0usize; 4];
        for r in weapons.rows() {
            if let Some(n) = r.data.get(equip_weapon::RARITY).and_then(|&v| seen.get_mut(v as usize)) {
                *n += 1;
            }
        }
        assert!(seen.iter().all(|&n| n > 0), "{path}: {seen:?}");
        // 深夜红色武器与紫、金同档：本体都是稀有；原版另有 10 行风暴统治者事件武器，本体是白色初始武器但带强化等级 3（传说）
        let custom = table(&reg, "EquipParamCustomWeapon", custom_weapon::TABLE);
        let cursed: Vec<u32> = custom
            .rows()
            .filter(|r| r.data[custom_weapon::IS_CURSED] != 0)
            .filter_map(|r| {
                let target = u32::try_from(i32::from_le_bytes(r.data[custom_weapon::TARGET_WEAPON_ID..][..4].try_into().unwrap())).ok()?;
                Some(target + u32::try_from(r.data[custom_weapon::WEAPON_LEVEL] as i8).ok()?)
            })
            .collect();
        assert!(!cursed.is_empty(), "{path}");
        let odd: Vec<(u32, Option<u8>)> = cursed.iter().map(|&id| (id, rarities.current(id))).filter(|(_, r)| !matches!(r, Some(2 | 3))).collect();
        assert!(odd.is_empty(), "{path}: {} of {} {:?}", odd.len(), cursed.len(), &odd[..odd.len().min(20)]);
    }
}
