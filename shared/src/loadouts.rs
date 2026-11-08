//! 角色初始装备（CharaInitParam 皮肤行的主手 / 副手武器）：各角色默认组合与改写计划。
//! 皮肤行在 50000..=50999（52xxx / 53xxx 等是调试行，不动）；+2 皮肤用带强化等级的 ID（如 3750002），改写时保留等级。

use std::collections::{BTreeMap, HashMap};
use std::ops::RangeInclusive;

use crate::param::layout::chara_init;
use crate::param::{ParamFile, field};
use crate::weapons::WeaponInfo;

pub const SKIN_ROWS: RangeInclusive<u32> = 50_000..=50_999;

#[derive(Debug, Clone, Copy)]
pub struct SkinRow {
    pub id: u32,
    /// 行数据在表中的偏移
    pub offset: usize,
    pub hero_id: u32,
    pub main: i32,
    pub off: i32,
}

/// 一处改写：表内偏移 + 新值
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldEdit {
    pub offset: usize,
    pub value: i32,
}

#[derive(Debug, Default, Clone)]
pub struct Loadouts {
    pub skins: Vec<SkinRow>,
    /// heroId -> (默认主手, 默认副手)，均为不带强化等级的本体 ID，副手 -1 为空
    pub defaults: BTreeMap<u32, (i32, i32)>,
}

impl Loadouts {
    pub fn build(table: &ParamFile) -> Self {
        Self::from_skins(
            table
                .rows()
                .filter(|r| SKIN_ROWS.contains(&r.id))
                .filter_map(|r| {
                    Some(SkinRow {
                        id: r.id,
                        offset: r.offset,
                        hero_id: field::u8(r.data, chara_init::HERO_ID)? as u32,
                        main: field::i32(r.data, chara_init::WEAPON_RIGHT_1)?,
                        off: field::i32(r.data, chara_init::WEAPON_LEFT_1)?,
                    })
                })
                .filter(|s| (1..=10).contains(&s.hero_id) && s.main > 0)
                .collect(),
        )
    }

    pub fn from_skins(skins: Vec<SkinRow>) -> Self {
        let mut counts: HashMap<u32, HashMap<(i32, i32), u32>> = HashMap::new();
        for s in &skins {
            *counts.entry(s.hero_id).or_default().entry((base(s.main), base(s.off))).or_default() += 1;
        }
        let defaults = counts
            .into_iter()
            .filter_map(|(hero, c)| c.into_iter().max_by_key(|&(pair, n)| (n, std::cmp::Reverse(pair))).map(|(pair, _)| (hero, pair)))
            .collect();
        Self { skins, defaults }
    }

    /// 把该角色所有皮肤的主手 / 副手换成 main / off（本体 ID，off 为 None 时去掉副手）。
    /// 武器数量不能超过原版：原本只有一把的角色不能加副手；有两把的角色主副手不能都是近战。
    /// 保留各皮肤原来的强化等级（EquipParamWeapon 只有本体行，+1~+3 由 ID 末位表示，所以只校验本体）
    pub fn plan(&self, hero_id: u32, main: u32, off: Option<u32>, weapons: &WeaponInfo) -> Result<Vec<FieldEdit>, String> {
        let &(_, own_off) = self.defaults.get(&hero_id).ok_or(format!("hero {hero_id} has no skin rows"))?;
        for id in std::iter::once(main).chain(off) {
            if !weapons.exists(id) {
                return Err(format!("weapon {id} not in EquipParamWeapon"));
            }
        }
        if let Some(off) = off {
            if own_off < 0 {
                return Err(format!("hero {hero_id} starts with one weapon only, off-hand {off} not allowed"));
            }
            if weapons.is_melee(main) && weapons.is_melee(off) {
                return Err(format!("main {main} and off-hand {off} are both melee weapons"));
            }
        }
        let mut edits = Vec::new();
        for s in self.skins.iter().filter(|s| s.hero_id == hero_id) {
            let level = s.main.rem_euclid(100) as u32;
            edits.push(FieldEdit { offset: s.offset + chara_init::WEAPON_RIGHT_1, value: (main + level) as i32 });
            let off = off.map_or(-1, |id| (id + level) as i32);
            edits.push(FieldEdit { offset: s.offset + chara_init::WEAPON_LEFT_1, value: off });
        }
        Ok(edits)
    }

    /// 把该角色所有皮肤的主手 / 副手写回读表时的原值
    pub fn restore(&self, hero_id: u32) -> Vec<FieldEdit> {
        self.skins
            .iter()
            .filter(|s| s.hero_id == hero_id)
            .flat_map(|s| {
                [
                    FieldEdit { offset: s.offset + chara_init::WEAPON_RIGHT_1, value: s.main },
                    FieldEdit { offset: s.offset + chara_init::WEAPON_LEFT_1, value: s.off },
                ]
            })
            .collect()
    }
}

/// 去掉强化等级的本体 ID；-1 保持 -1
fn base(id: i32) -> i32 {
    if id < 0 { -1 } else { id - id % 100 }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn skin(id: u32, offset: usize, hero_id: u32, main: i32, off: i32) -> SkinRow {
        SkinRow { id, offset, hero_id, main, off }
    }

    fn sample() -> (Loadouts, WeaponInfo) {
        let l = Loadouts::from_skins(vec![
            skin(50000, 0x100, 1, 3750000, 30750000),
            skin(50010, 0x200, 1, 3750000, 30750000),
            skin(50021, 0x300, 1, 3750002, 30750002),
            skin(50022, 0x400, 1, 3750000, -1),
            skin(50700, 0x500, 8, 9750000, -1),
        ]);
        // 与游戏一致：只有本体行
        let w = WeaponInfo::from_rows([(3750000, 5, 10102), (30750000, 65, 12021), (9750000, 13, 10329)]);
        (l, w)
    }

    #[test]
    fn finds_default_pairs() {
        let (l, _) = sample();
        assert_eq!(l.defaults[&1], (3750000, 30750000));
        assert_eq!(l.defaults[&8], (9750000, -1));
    }

    #[test]
    fn plans_edits_keeping_level() {
        let (l, w) = sample();
        let e = l.plan(1, 9750000, None, &w).unwrap();
        assert_eq!(e.len(), 8);
        assert!(e.contains(&FieldEdit { offset: 0x300 + chara_init::WEAPON_RIGHT_1, value: 9750002 }));
        assert!(e.contains(&FieldEdit { offset: 0x100 + chara_init::WEAPON_LEFT_1, value: -1 }));
        // 执行者只换主手（原本单武器）
        let e = l.plan(8, 3750000, None, &w).unwrap();
        assert_eq!(e, vec![
            FieldEdit { offset: 0x500 + chara_init::WEAPON_RIGHT_1, value: 3750000 },
            FieldEdit { offset: 0x500 + chara_init::WEAPON_LEFT_1, value: -1 },
        ]);
        // 追踪者：执行者刀 + 自己的小盾
        assert!(l.plan(1, 9750000, Some(30750000), &w).is_ok());
    }

    #[test]
    fn restore_writes_original_values() {
        let (l, _) = sample();
        let e = l.restore(1);
        assert_eq!(e.len(), 8);
        assert!(e.contains(&FieldEdit { offset: 0x300 + chara_init::WEAPON_RIGHT_1, value: 3750002 }));
        assert!(e.contains(&FieldEdit { offset: 0x400 + chara_init::WEAPON_LEFT_1, value: -1 }));
    }

    #[test]
    fn rejects_extra_weapon_two_melee_and_unknown() {
        let (l, w) = sample();
        // 单武器角色不能加副手
        assert!(l.plan(8, 9750000, Some(30750000), &w).is_err());
        // 两把近战
        assert!(l.plan(1, 3750000, Some(9750000), &w).is_err());
        assert!(l.plan(1, 1234500, None, &w).is_err());
        assert!(l.plan(3, 9750000, None, &w).is_err());
    }
}
