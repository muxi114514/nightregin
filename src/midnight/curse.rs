//! 静默之域的诅咒生效：在圆桌里按目标诅咒改内存参数并记原值，取消就还原；玩家类效果由每帧任务挂到角色上。
//! 全部在游戏主线程调用；只改本机内存，不碰 regulation。

use log::{info, warn};
use nr_stat_alloc_shared::midnight::curse::{Curse, CurseEffect, CurseId, NO_CURSE, POOL, enemy_effect_rows, find};
use nr_stat_alloc_shared::param::ParamFile;
use nr_stat_alloc_shared::param::layout::{chaos_correct, chaos_rank, sp_effect};
use nr_stat_alloc_shared::tiered_params::Change;

use crate::game::param_patch::{PatchField, locate_rows};
use crate::game::params::ParamRepository;

/// 世界类诅咒里被选中的权重
const FAVORED_WEIGHT: u8 = 100;

pub struct CurseEngine {
    groups: Vec<(CurseId, Vec<PatchField>)>,
    available: Vec<CurseId>,
    applied: CurseId,
    warned_unknown: bool,
}

impl CurseEngine {
    /// 参数表读不到时用：没有可用诅咒，午夜照常
    pub fn empty() -> Self {
        Self { groups: Vec::new(), available: Vec::new(), applied: NO_CURSE, warned_unknown: false }
    }

    /// 启动时（参数表还是原值）定位每条诅咒的字段；有表或行找不到的诅咒从池里去掉
    pub fn locate(repository: &ParamRepository) -> Self {
        let tables = repository.read_tables(&[chaos_correct::TABLE.struct_name, sp_effect::TABLE.struct_name]);
        let correct = tables.get(chaos_correct::TABLE.struct_name).map_or_else(Vec::new, |b| correct_rows(b));
        let effects = tables.get(sp_effect::TABLE.struct_name).and_then(|b| ParamFile::parse(b, sp_effect::TABLE.row_size).ok());
        let mut engine = Self::empty();
        let mut removed = Vec::new();
        for curse in &POOL {
            match locate_curse(repository, curse, &correct, effects.as_ref()) {
                Some(fields) => {
                    engine.available.push(curse.id);
                    if !fields.is_empty() {
                        engine.groups.push((curse.id, fields));
                    }
                }
                None => removed.push(curse.id),
            }
        }
        info!("midnight curses: {} available, removed {removed:?}", engine.available.len());
        engine
    }

    #[cfg(test)]
    fn from_groups(groups: Vec<(CurseId, Vec<PatchField>)>, available: Vec<CurseId>) -> Self {
        Self { groups, available, ..Self::empty() }
    }

    pub fn available(&self) -> &[CurseId] {
        &self.available
    }

    /// 让内存数据对应 target：变了才写，先还原旧的再写新的；不认识的编号按没有诅咒处理
    pub fn sync(&mut self, target: CurseId) {
        let target = if target == NO_CURSE || find(target).is_some() {
            target
        } else {
            if !std::mem::replace(&mut self.warned_unknown, true) {
                warn!("midnight curses: unknown curse {target}, ignored");
            }
            NO_CURSE
        };
        if target == self.applied {
            return;
        }
        self.write_group(self.applied, false);
        self.write_group(target, true);
        info!("midnight curses: data {} -> {target}", self.applied);
        self.applied = target;
    }

    pub fn restore(&mut self) {
        self.sync(NO_CURSE);
    }

    fn write_group(&self, id: CurseId, on: bool) {
        let Some((_, fields)) = self.groups.iter().find(|(g, _)| *g == id) else { return };
        let failed = fields.iter().filter(|f| !if on { f.write(1) } else { f.restore() }).count();
        if failed > 0 {
            warn!("midnight curses: curse {id}: {failed} writes failed");
        }
    }
}

/// 强度表每行：(行 ID, 5 个效果编号)
fn correct_rows(bytes: &[u8]) -> Vec<(u32, [i32; 5])> {
    let Ok(file) = ParamFile::parse(bytes, chaos_correct::TABLE.row_size) else { return Vec::new() };
    file.rows()
        .map(|r| {
            let cell = |offset: usize| r.data.get(offset..offset + 4).and_then(|b| b.try_into().ok()).map_or(-1, i32::from_le_bytes);
            (r.id, chaos_correct::EFFECTS.map(cell))
        })
        .collect()
}

/// 这条诅咒要改的字段（玩家类为空）；缺表或缺行时为 None
fn locate_curse(repository: &ParamRepository, curse: &Curse, correct: &[(u32, [i32; 5])], effects: Option<&ParamFile>) -> Option<Vec<PatchField>> {
    match curse.effect {
        CurseEffect::Player { sp_effect } => effects?.row(sp_effect as u32).map(|_| Vec::new()),
        CurseEffect::Enemy { rows, fields, scale } => {
            let ids = enemy_effect_rows(correct, rows);
            let mut out = Vec::new();
            for &offset in fields {
                out.extend(all_rows(locate_rows(repository, sp_effect::TABLE, &ids, offset, Change::ScaleF32(scale)), ids.len())?);
            }
            (!out.is_empty()).then_some(out)
        }
        CurseEffect::World { fields, favored } => {
            let mut out = Vec::new();
            for offset in fields {
                let weight = if offset == favored { FAVORED_WEIGHT } else { 0 };
                out.extend(all_rows(locate_rows(repository, chaos_rank::TABLE, &chaos_rank::ROWS, offset, Change::SetU8(weight)), chaos_rank::ROWS.len())?);
            }
            Some(out)
        }
    }
}

fn all_rows(found: Vec<PatchField>, want: usize) -> Option<Vec<PatchField>> {
    (found.len() == want).then_some(found)
}

/// 玩家类诅咒要挂的效果
pub fn player_effect(id: CurseId) -> Option<i32> {
    match find(id)?.effect {
        CurseEffect::Player { sp_effect } => Some(sp_effect),
        _ => None,
    }
}

/// 角色身上缺这个效果（效果列表读不到时不算缺，避免重复施加）
pub fn effect_missing(ids: Option<&[i32]>, sp_effect: i32) -> bool {
    ids.is_some_and(|ids| !ids.contains(&sp_effect))
}

#[cfg(test)]
mod tests {
    use nr_stat_alloc_shared::tiered_params::Change;

    use super::*;

    /// 敌人类诅咒 11 的分组：一个 f32 字段
    fn scaled(buf: &mut [u8; 4], k: f32) -> PatchField {
        PatchField::new(buf.as_mut_ptr() as usize, buf.to_vec(), Change::ScaleF32(k))
    }

    fn value(buf: &[u8; 4]) -> f32 {
        f32::from_le_bytes(*buf)
    }

    #[test]
    fn sync_applies_target_fields() {
        let mut a = 2.0f32.to_le_bytes();
        let mut e = CurseEngine::from_groups(vec![(11, vec![scaled(&mut a, 1.5)])], vec![11]);
        e.sync(11);
        assert_eq!(value(&a), 3.0);
        assert_eq!(e.available(), &[11]);
    }

    #[test]
    fn switching_curses_restores_the_previous_one() {
        let (mut a, mut b) = (2.0f32.to_le_bytes(), 4.0f32.to_le_bytes());
        let mut e = CurseEngine::from_groups(vec![(11, vec![scaled(&mut a, 1.5)]), (12, vec![scaled(&mut b, 2.0)])], vec![11, 12]);
        e.sync(11);
        e.sync(12);
        assert_eq!((value(&a), value(&b)), (2.0, 8.0));
    }

    #[test]
    fn repeated_switching_never_compounds() {
        let mut a = 2.0f32.to_le_bytes();
        let mut e = CurseEngine::from_groups(vec![(11, vec![scaled(&mut a, 1.5)])], vec![11]);
        for _ in 0..10 {
            e.sync(11);
            e.sync(NO_CURSE);
        }
        e.sync(11);
        e.sync(11);
        assert_eq!(value(&a), 3.0);
    }

    #[test]
    fn restore_returns_exact_bytes() {
        let mut a = 1.718f32.to_le_bytes();
        let before = a;
        let mut e = CurseEngine::from_groups(vec![(11, vec![scaled(&mut a, 1.25)])], vec![11]);
        e.sync(11);
        assert_ne!(a, before);
        e.restore();
        assert_eq!(a, before);
    }

    #[test]
    fn unknown_curse_does_nothing() {
        let mut a = 2.0f32.to_le_bytes();
        let before = a;
        let mut e = CurseEngine::from_groups(vec![(11, vec![scaled(&mut a, 1.5)])], vec![11]);
        e.sync(200);
        assert_eq!(a, before);
        e.sync(11);
        assert_eq!(value(&a), 3.0);
    }

    #[test]
    fn unknown_curse_restores_the_previous_one() {
        let mut a = 2.0f32.to_le_bytes();
        let before = a;
        let mut e = CurseEngine::from_groups(vec![(11, vec![scaled(&mut a, 1.5)])], vec![11]);
        e.sync(11);
        e.sync(200);
        assert_eq!(a, before);
    }

    #[test]
    fn player_curse_has_no_fields_to_write() {
        let mut a = 2.0f32.to_le_bytes();
        let before = a;
        let mut e = CurseEngine::from_groups(vec![(11, vec![scaled(&mut a, 1.5)])], vec![1, 11]);
        e.sync(1);
        assert_eq!(a, before);
    }

    #[test]
    fn empty_engine_has_no_curses() {
        let mut e = CurseEngine::empty();
        assert!(e.available().is_empty());
        e.sync(11);
        e.restore();
    }

    #[test]
    fn effect_missing_only_when_readable() {
        assert!(effect_missing(Some(&[1, 2]), 3));
        assert!(!effect_missing(Some(&[1, 3]), 3));
        assert!(!effect_missing(None, 3));
    }
}
