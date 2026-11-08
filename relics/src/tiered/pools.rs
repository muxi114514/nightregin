//! 锻体池（ItemTableParam）：同一表 ID 占连续多行，每行一个候选，只放锻体或道具（类别 1），不嵌套别的表。
//! - 蓝 / 紫池 = 原版对应池里的锻体（去掉不再掉落与移进金池的）+ 新增候选；金池全由 buffs::GOLD 给出
//! - 角色池 = 各角色的专属行（onlyForClass 过滤，游戏只在符合条件的行里抽）；还没有专属行的角色补一份只给他的紫池
//! - 指定角色池（hero_role）= 同一批专属行按角色各一张、不设 onlyForClass，罪人入侵掉落按罪人对应的角色引用
//! - 外层表里嵌套引用旧池的行（法师塔 / 神殿 / 古老法师塔奖励）改指新池，紫色那行按 tiered::split 拆成紫 / 金 / 角色三行
//!   重复运行先删掉旧的新表再生成，结果不变。

use nr_stat_alloc_patcher::param_write::OwnedRow;
use nr_stat_alloc_shared::param::{ParamFile, field};

use crate::layout::equip_goods::BUFF_ID;
use crate::layout::item_table::{CATEGORY, EVENT_FLAG, ITEM_ID, ONLY_FOR_CLASS, QUANTITY, WEIGHT};
use crate::pool::{self, BUFF_CATEGORY};
use crate::spec::NAME_PREFIX;

use super::{
    BLUE, BLUE_SOURCES, GOLD, HEROES, PURPLE, PURPLE_SOURCES, RETIRED, ROLE, TABLES, Tier, aura, buffs, dll, event, guardian, hero_role, is_hero_role,
    kept, low_hp, params, role_actions, role_scripts, scripted, split, tae, tier_of,
};

const TABLE_CATEGORY: i32 = 7;
const CANDIDATE_WEIGHT: u16 = 100;

#[derive(Clone, Copy)]
pub enum Candidate {
    /// 锻体（PermanentBuffParam，同号道具行类型 11）
    Buff(u32),
    /// 道具与数量
    Goods(i32, u8),
}

/// 只给某个角色抽的锻体
pub struct RoleBuff {
    pub hero: u8,
    pub buff: u32,
}

#[derive(Default)]
pub struct Report {
    pub blue: usize,
    pub purple: usize,
    pub gold: usize,
    pub role: usize,
    pub nested: usize,
}

#[derive(Clone, Copy)]
struct Entry {
    item: i32,
    quantity: u8,
    hero: u8,
}

impl Candidate {
    fn entry(self) -> Entry {
        match self {
            Candidate::Buff(id) => Entry { item: id as i32, quantity: 1, hero: 0 },
            Candidate::Goods(id, n) => Entry { item: id, quantity: n, hero: 0 },
        }
    }
}

pub fn apply(rows: &mut Vec<OwnedRow>, report: &mut Report) -> Result<(), String> {
    let template = rows.iter().find(|r| r.id == BLUE_SOURCES[0]).ok_or(format!("pool {} not found", BLUE_SOURCES[0]))?.data.clone();
    rows.retain(|r| !TABLES.contains(&r.id) && !RETIRED.contains(&r.id) && !is_hero_role(r.id));
    let gold_list = concat(&[buffs::GOLD, scripted::GOLD, kept::GOLD, dll::GOLD]);
    let gold: Vec<u32> = gold_list.iter().filter_map(|c| if let Candidate::Buff(b) = c { Some(*b) } else { None }).collect();
    let keep = |b: &u32| !buffs::EXCLUDED.contains(b) && !gold.contains(b);
    let blue: Vec<u32> = source_buffs(rows, BLUE_SOURCES).into_iter().filter(keep).collect();
    let purple: Vec<u32> = source_buffs(rows, PURPLE_SOURCES).into_iter().filter(|b| keep(b) && !blue.contains(b)).collect();

    let with_extra = |buffs: &[u32], extra: &[Candidate]| -> Vec<Entry> {
        buffs.iter().map(|&b| Candidate::Buff(b)).chain(extra.iter().copied()).map(Candidate::entry).collect()
    };
    let blue_rows = with_extra(&blue, &concat(&[buffs::BLUE_EXTRA, dll::BLUE, params::BLUE]));
    let purple_rows = with_extra(&purple, &concat(&[buffs::PURPLE_EXTRA, scripted::PURPLE, kept::PURPLE, dll::PURPLE, event::PURPLE, low_hp::PURPLE]));
    let gold_rows = with_extra(&[], &gold_list);
    let role_rows = role_entries(&purple_rows);
    (report.blue, report.purple, report.gold, report.role) = (blue_rows.len(), purple_rows.len(), gold_rows.len(), role_buffs().count());

    report.nested = rewrite_nested(rows);
    let any_class: Vec<(u32, String, Vec<Entry>)> = HEROES
        .map(|hero| {
            let entries = role_rows.iter().filter(|e| e.hero == hero).map(|e| Entry { hero: 0, ..*e }).collect();
            (hero_role(hero), format!("Hero {hero} Any"), entries)
        })
        .collect();
    let groups = [(BLUE, "Blue", blue_rows), (PURPLE, "Purple", purple_rows), (GOLD, "Gold", gold_rows), (ROLE, "Hero", role_rows)]
        .into_iter()
        .map(|(id, label, entries)| (id, label.to_string(), entries))
        .chain(any_class);
    for (id, label, entries) in groups {
        insert_group(rows, id, &label, &template, &entries);
        pool::check_contiguous(rows, id)?;
    }
    Ok(())
}

/// 输出自检：新池都在、只有类别 1 的行且都有道具行（缺了选项显示 ?GoodsName?），锻体的道具行指回锻体自身（指错就变成别的锻体），
/// 每个角色都有专属行，指定角色池不限角色，外层表不再嵌套引用旧池
pub fn verify(file: &ParamFile, goods: &ParamFile, permanent: &ParamFile) -> Result<(), String> {
    for id in TABLES.into_iter().chain(HEROES.map(hero_role)) {
        if file.row(id).is_none() {
            return Err(format!("self-check: tiered pool {id} missing"));
        }
    }
    for r in file.rows().filter(|r| TABLES.contains(&r.id) || is_hero_role(r.id)) {
        let item = field::i32(r.data, ITEM_ID).unwrap_or(0) as u32;
        if field::i32(r.data, CATEGORY) != Some(BUFF_CATEGORY) {
            return Err(format!("self-check: tiered pool {} nests a table ({item})", r.id));
        }
        if is_hero_role(r.id) && r.data[ONLY_FOR_CLASS] != 0 {
            return Err(format!("self-check: hero pool {} is limited to class {}", r.id, r.data[ONLY_FOR_CLASS]));
        }
        let Some(goods_row) = goods.row(item) else {
            return Err(format!("self-check: tiered candidate {item} has no EquipParamGoods row"));
        };
        if permanent.row(item).is_some() && field::i32(goods_row, BUFF_ID) != Some(item as i32) {
            return Err(format!("self-check: goods row of permanent buff {item} points to {:?}", field::i32(goods_row, BUFF_ID)));
        }
    }
    if let Some(hero) = HEROES.clone().find(|&h| !file.rows().any(|r| r.id == ROLE && r.data[ONLY_FOR_CLASS] == h)) {
        return Err(format!("self-check: hero {hero} has no row in the hero pool"));
    }
    let stale = file.rows().find(|r| {
        !BLUE_SOURCES.contains(&r.id)
            && !PURPLE_SOURCES.contains(&r.id)
            && field::i32(r.data, CATEGORY) == Some(TABLE_CATEGORY)
            && field::i32(r.data, ITEM_ID).is_some_and(|t| tier_of(t as u32).is_some() || RETIRED.contains(&(t as u32)))
    });
    match stale {
        Some(r) => Err(format!("self-check: item table {} still nests an old pool", r.id)),
        None => Ok(()),
    }
}

/// 若干原版池里出现过的锻体，按首次出现的顺序去重
fn source_buffs(rows: &[OwnedRow], pools: &[u32]) -> Vec<u32> {
    let mut out = Vec::new();
    for r in rows.iter().filter(|r| pools.contains(&r.id)) {
        if field::i32(&r.data, CATEGORY) != Some(BUFF_CATEGORY) {
            continue;
        }
        if let Some(b) = field::i32(&r.data, ITEM_ID).map(|b| b as u32).filter(|b| !out.contains(b)) {
            out.push(b);
        }
    }
    out
}

/// 各批候选合在一起
fn concat(parts: &[&[Candidate]]) -> Vec<Candidate> {
    parts.iter().flat_map(|p| p.iter().copied()).collect()
}

fn role_buffs() -> impl Iterator<Item = &'static RoleBuff> + Clone {
    buffs::ROLE
        .iter()
        .chain(scripted::ROLE)
        .chain(kept::ROLE)
        .chain(aura::ROLE)
        .chain(dll::ROLE)
        .chain(tae::ROLE)
        .chain(role_scripts::ROLE)
        .chain(role_actions::ROLE)
        .chain(params::ROLE)
        .chain(guardian::ROLE)
}

/// 专属行，加上还没有专属锻体的角色各一份紫池
fn role_entries(purple: &[Entry]) -> Vec<Entry> {
    let mut out: Vec<Entry> = role_buffs().map(|r| Entry { hero: r.hero, ..Candidate::Buff(r.buff).entry() }).collect();
    for hero in HEROES.filter(|h| !role_buffs().any(|r| r.hero == *h)) {
        out.extend(purple.iter().map(|e| Entry { hero, ..*e }));
    }
    out
}

/// 外层表里嵌套旧池的行改指新池：蓝 / 角色直接换表，紫色那行拆成紫 / 金 / 角色三行（权重按 split 分）
fn rewrite_nested(rows: &mut Vec<OwnedRow>) -> usize {
    let mut n = 0;
    let mut i = 0;
    while i < rows.len() {
        let row = &rows[i];
        let tier = (!BLUE_SOURCES.contains(&row.id) && !PURPLE_SOURCES.contains(&row.id) && field::i32(&row.data, CATEGORY) == Some(TABLE_CATEGORY))
            .then(|| field::i32(&row.data, ITEM_ID).and_then(|t| tier_of(t as u32)))
            .flatten();
        let Some(tier) = tier else {
            i += 1;
            continue;
        };
        let weight = field::u16(&row.data, WEIGHT).unwrap_or(0) as u32;
        let parts = if tier == Tier::Blue { vec![(BLUE, weight)] } else { split(tier, weight) };
        let replaced: Vec<OwnedRow> = parts
            .iter()
            .map(|&(table, w)| {
                let mut data = row.data.clone();
                data[ITEM_ID..ITEM_ID + 4].copy_from_slice(&(table as i32).to_le_bytes());
                data[WEIGHT..WEIGHT + 2].copy_from_slice(&(w as u16).to_le_bytes());
                OwnedRow { id: row.id, data, name: row.name.clone() }
            })
            .collect();
        let len = replaced.len();
        rows.splice(i..=i, replaced);
        i += len;
        n += 1;
    }
    n
}

/// 有序表插在比它大的第一行之前，乱序表追加在末尾（游戏加载时按 ID 重建索引，同 ID 的行要连续）
fn insert_group(rows: &mut Vec<OwnedRow>, id: u32, label: &str, template: &[u8], entries: &[Entry]) {
    let sorted = rows.windows(2).all(|w| w[0].id <= w[1].id);
    let at = if sorted { rows.iter().position(|r| r.id > id).unwrap_or(rows.len()) } else { rows.len() };
    let group = entries.iter().map(|e| {
        let mut data = template.to_vec();
        data[CATEGORY..CATEGORY + 4].copy_from_slice(&BUFF_CATEGORY.to_le_bytes());
        data[ITEM_ID..ITEM_ID + 4].copy_from_slice(&e.item.to_le_bytes());
        data[WEIGHT..WEIGHT + 2].copy_from_slice(&CANDIDATE_WEIGHT.to_le_bytes());
        data[QUANTITY] = e.quantity;
        data[ONLY_FOR_CLASS] = e.hero;
        data[EVENT_FLAG..EVENT_FLAG + 4].copy_from_slice(&0i32.to_le_bytes());
        OwnedRow { id, data, name: Some(format!("{NAME_PREFIX}Tiered {label}")) }
    });
    rows.splice(at..at, group.collect::<Vec<_>>());
}
