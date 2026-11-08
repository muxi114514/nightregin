//! 从参数表提取 DLL 需要的数据：各角色成长关键帧、器皿、三维曲线、皮肤 -> 角色、整表替换 / 属性改变类效果、组件是否就绪。
//! 数据来源可以是游戏内存中已加载的表（DLL），也可以是 regulation 文件（补丁工具的测试），提取逻辑相同。

use crate::components::{self, RESTORE_FP, RESTORE_HP};
use crate::param::layout::{Table, antique_stand, chara_init, correct_graph, hero, hero_status, sp_effect};
use crate::param::{ParamFile, field};

pub const HERO_COUNT: u32 = 10;
/// AntiqueStandParam.heroType：1..=10 专属器皿，11 为所有角色通用
pub const UNIVERSAL_VESSEL: u8 = 11;
/// 每个角色的成长关键帧在 HeroStatusParam 中占用的 ID 跨度
const STATUS_SPAN: u32 = 100;

pub struct Tables<'a> {
    pub sp_effect: ParamFile<'a>,
    pub hero: ParamFile<'a>,
    pub hero_status: ParamFile<'a>,
    pub chara_init: ParamFile<'a>,
    pub antique_stand: ParamFile<'a>,
    pub correct_graph: ParamFile<'a>,
}

impl<'a> Tables<'a> {
    /// source：按表取原始数据；逐张校验类型名与行大小
    pub fn load(mut source: impl FnMut(&Table) -> Option<&'a [u8]>) -> Result<Self, String> {
        let mut open = |t: Table| -> Result<ParamFile<'a>, String> {
            let bytes = source(&t).ok_or(format!("{} not found", t.struct_name))?;
            let file = ParamFile::parse(bytes, t.row_size).map_err(|e| format!("{}: {e}", t.struct_name))?;
            match file.struct_name() {
                Some(name) if name == t.struct_name => Ok(file),
                other => Err(format!("expected {}, found {other:?}", t.struct_name)),
            }
        };
        Ok(Self {
            sp_effect: open(sp_effect::TABLE)?,
            hero: open(hero::TABLE)?,
            hero_status: open(hero_status::TABLE)?,
            chara_init: open(chara_init::TABLE)?,
            antique_stand: open(antique_stand::TABLE)?,
            correct_graph: open(correct_graph::TABLE)?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct HeroRecord {
    /// 1..=10
    pub hero_id: u32,
    pub levels: Vec<u32>,
    pub stats: Vec<[u32; components::STAT_COUNT]>,
}

#[derive(Debug, Clone, Copy)]
pub struct VesselRecord {
    pub id: u32,
    pub hero_type: u8,
    pub goods_id: i32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CurveRecord {
    pub stage_max_val: [f32; 5],
    pub stage_max_grow_val: [f32; 5],
    pub adj_pt: [f32; 5],
}

pub struct Extracted {
    pub heroes: Vec<HeroRecord>,
    pub vessels: Vec<VesselRecord>,
    /// 血量 / 专注值 / 精力
    pub curves: Option<[CurveRecord; 3]>,
    /// (CharaInitParam ID, heroId)
    pub chara_hero: Vec<(u32, u32)>,
    pub replacement: Vec<i32>,
    pub modifier: Vec<i32>,
    /// 组件与回满效果全部存在（regulation 打过补丁）
    pub components_ready: bool,
    /// 组件时长（秒）；组件不存在时为 None
    pub lease_seconds: Option<f32>,
}

pub fn extract(t: &Tables) -> Result<Extracted, String> {
    Ok(Extracted {
        heroes: heroes(t)?,
        vessels: vessels(t),
        curves: curves(t),
        chara_hero: t
            .chara_init
            .rows()
            .filter_map(|r| field::u8(r.data, chara_init::HERO_ID).map(|h| (r.id, h as u32)))
            .filter(|&(_, h)| (1..=HERO_COUNT).contains(&h))
            .collect(),
        replacement: ids_where(&t.sp_effect, sp_effect::HERO_STATUS_ID),
        modifier: ids_where(&t.sp_effect, sp_effect::HERO_STATUS_MODIFIER),
        components_ready: components::all()
            .map(|c| c.0)
            .chain([RESTORE_HP, RESTORE_FP])
            .all(|id| t.sp_effect.row(id as u32).is_some()),
        lease_seconds: components::component_id(0, 1)
            .and_then(|id| t.sp_effect.row(id as u32))
            .and_then(|r| field::f32(r, sp_effect::EFFECT_ENDURANCE)),
    })
}

fn heroes(t: &Tables) -> Result<Vec<HeroRecord>, String> {
    (1..=HERO_COUNT)
        .map(|hero_id| {
            let row = t.hero.row(hero_id).ok_or(format!("HeroParam {hero_id} missing"))?;
            let base = field::i32(row, hero::STATUS_PARAM_ID).unwrap_or(0).max(0) as u32;
            let (mut levels, mut stats) = (Vec::new(), Vec::new());
            for r in t.hero_status.rows().filter(|r| (base..base + STATUS_SPAN).contains(&r.id)) {
                levels.push(field::u16(r.data, hero_status::TOTAL_LEVEL).unwrap_or(0) as u32);
                stats.push(std::array::from_fn(|i| {
                    field::i8(r.data, hero_status::STATS + i).unwrap_or(0).max(0) as u32
                }));
            }
            if levels.len() < 2 {
                return Err(format!("hero {hero_id}: no growth keyframes at HeroStatusParam {base}"));
            }
            Ok(HeroRecord { hero_id, levels, stats })
        })
        .collect()
}

fn vessels(t: &Tables) -> Vec<VesselRecord> {
    t.antique_stand
        .rows()
        .filter_map(|r| {
            let hero_type = field::i8(r.data, antique_stand::HERO_TYPE)?;
            let goods_id = field::i32(r.data, antique_stand::GOODS_ID)?;
            (1..=UNIVERSAL_VESSEL as i8).contains(&hero_type).then_some(VesselRecord { id: r.id, hero_type: hero_type as u8, goods_id })
        })
        .collect()
}

fn curves(t: &Tables) -> Option<[CurveRecord; 3]> {
    let curve = |id: u32| {
        let row = t.correct_graph.row(id)?;
        Some(CurveRecord {
            stage_max_val: field::f32s(row, correct_graph::STAGE_MAX_VAL)?,
            stage_max_grow_val: field::f32s(row, correct_graph::STAGE_MAX_GROW_VAL)?,
            adj_pt: field::f32s(row, correct_graph::ADJ_PT_MAX_GROW_VAL)?,
        })
    };
    Some([curve(correct_graph::HP)?, curve(correct_graph::FP)?, curve(correct_graph::STAMINA)?])
}

fn ids_where(table: &ParamFile, offset: usize) -> Vec<i32> {
    table.rows().filter(|r| field::i32(r.data, offset).is_some_and(|v| v > 0)).map(|r| r.id as i32).collect()
}
