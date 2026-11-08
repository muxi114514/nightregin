//! 出击时改战技 / 改魔法 / 附加类遗物（SpEffect 的 startSwordArtsId / startMagicId / 状态值 2101）的角色开关：
//! AttachEffectParam 的允许角色位（不允许的角色带着遗物也不生效），以及战技在 SwordArtsParam 里的 isStartSkill 位。
//! 原版按各角色自带的初始武器设定，如改祷告只开复仇者、改魔法只开隐士、箭雨只开铁之眼、附加不开隐士（法杖不能附加）。
//! 互换初始武器后按武器的原主人重算：带着谁的武器，就获得谁对这类武器的开关（法杖 / 圣印记对应改魔法，其余对应改战技与附加）；
//! 原主人有两把同类武器时（如大剑 + 小盾）只算主手那把。
//! 一把武器可能是多个角色的默认武器（空城 0.46 送葬者副手就是复仇者的圣印记），任一原主人开着即获得；
//! 换回自己原版组合的角色视同没换。

use std::collections::{BTreeMap, HashMap};

use crate::param::layout::{attach_effect, sp_effect, sword_arts};
use crate::param::{ParamFile, field};
use crate::weapons::WeaponInfo;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// 改战技 / 附加：作用于法杖、圣印记以外的武器
    Skill,
    /// 改魔法 / 祷告：作用于法杖、圣印记
    Magic,
}

/// 出击时的武器附加属性 / 异常状态类词条（原版 7120000~7120600）常驻效果的状态值
pub const IMBUE_STATE: u16 = 2101;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateTable {
    AttachEffect,
    SwordArts,
}

impl GateTable {
    fn bits(self) -> &'static [(usize, u8); 10] {
        match self {
            Self::AttachEffect => &attach_effect::ALLOW_BITS,
            Self::SwordArts => &sword_arts::START_SKILL_BITS,
        }
    }
}

/// 一处单字节改写，偏移相对表开头
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ByteEdit {
    pub table: GateTable,
    pub offset: usize,
    pub value: u8,
    pub original: u8,
}

/// 行内角色位 -> 掩码（第 heroId-1 位）
pub fn hero_mask(table: GateTable, row: &[u8]) -> Option<u16> {
    let mut mask = 0;
    for (h, &(byte, bit)) in table.bits().iter().enumerate() {
        if field::u8(row, byte)? >> bit & 1 != 0 {
            mask |= 1 << h;
        }
    }
    Some(mask)
}

#[derive(Debug, Clone)]
struct Gate {
    table: GateTable,
    id: u32,
    /// 行数据在表中的偏移
    offset: usize,
    kind: Kind,
    original: u16,
    /// 角色位所在字节：(行内偏移, 原值)
    bytes: Vec<(usize, u8)>,
}

impl Gate {
    fn read(table: GateTable, id: u32, offset: usize, row: &[u8], kind: Kind) -> Option<Self> {
        let mut bytes: Vec<(usize, u8)> = Vec::new();
        for &(byte, _) in table.bits() {
            if !bytes.iter().any(|&(b, _)| b == byte) {
                bytes.push((byte, field::u8(row, byte)?));
            }
        }
        Some(Self { table, id, offset, kind, original: hero_mask(table, row)?, bytes })
    }

    /// 没换武器的角色保持原值；换过的角色按所带武器的原主人决定
    fn mask(&self, carried: &BTreeMap<u32, Vec<u32>>, carriers: &HashMap<u32, Vec<(u32, Kind)>>) -> u16 {
        let mut mask = self.original;
        for (&hero, weapons) in carried {
            let Some(hero_bit) = bit(hero) else { continue };
            let allowed = weapons.iter().filter_map(|&w| carriers.get(&base(w))).flatten().any(|&(owner, kind)| {
                kind == self.kind && bit(owner).is_some_and(|b| self.original & b != 0)
            });
            if allowed { mask |= hero_bit } else { mask &= !hero_bit }
        }
        mask
    }

    fn edits(&self, mask: u16) -> impl Iterator<Item = ByteEdit> + '_ {
        self.bytes.iter().map(move |&(byte, original)| {
            let mut value = original;
            for (h, &(_, bit)) in self.table.bits().iter().enumerate().filter(|(_, (b, _))| *b == byte) {
                if mask >> h & 1 != 0 { value |= 1 << bit } else { value &= !(1 << bit) }
            }
            ByteEdit { table: self.table, offset: self.offset + byte, value, original }
        })
    }
}

#[derive(Debug, Default, Clone)]
pub struct StartRelics {
    gates: Vec<Gate>,
}

impl StartRelics {
    pub fn build(attach: &ParamFile, sp_effects: &ParamFile, arts: &ParamFile) -> Self {
        let mut gates = Vec::new();
        let mut art_ids = Vec::new();
        for r in attach.rows() {
            let sp = field::i32(r.data, attach_effect::PASSIVE_SP_EFFECT).and_then(|id| sp_effects.row(u32::try_from(id).ok()?));
            let Some(sp) = sp else { continue };
            let start = |offset| field::i32(sp, offset).filter(|&v| v > 0);
            let kind = match (start(sp_effect::START_SWORD_ARTS_ID), start(sp_effect::START_MAGIC_ID)) {
                (Some(art), _) => {
                    art_ids.push(art as u32);
                    Kind::Skill
                }
                (None, Some(_)) => Kind::Magic,
                _ if field::u16(sp, sp_effect::STATE_INFO) == Some(IMBUE_STATE) => Kind::Skill,
                _ => continue,
            };
            gates.extend(Gate::read(GateTable::AttachEffect, r.id, r.offset, r.data, kind));
        }
        for r in arts.rows().filter(|r| art_ids.contains(&r.id)) {
            gates.extend(Gate::read(GateTable::SwordArts, r.id, r.offset, r.data, Kind::Skill));
        }
        Self { gates }
    }

    pub fn len(&self) -> usize {
        self.gates.len()
    }

    pub fn is_empty(&self) -> bool {
        self.gates.is_empty()
    }

    /// 某行的类别与原版角色掩码
    pub fn gate(&self, table: GateTable, id: u32) -> Option<(Kind, u16)> {
        self.gates.iter().find(|g| g.table == table && g.id == id).map(|g| (g.kind, g.original))
    }

    /// carried：换过初始武器的角色（heroId）-> 实际带的武器；defaults：各角色原版 (主手, 副手)。
    /// 返回全部开关字节的目标值（含与原版相同的），整体写入即可，切回原版也走这里
    pub fn plan(&self, carried: &BTreeMap<u32, Vec<u32>>, defaults: &BTreeMap<u32, (i32, i32)>, weapons: &WeaponInfo) -> Vec<ByteEdit> {
        let carriers = carriers(defaults, weapons);
        let swapped: BTreeMap<u32, Vec<u32>> =
            carried.iter().filter(|&(hero, carried)| !is_own_loadout(defaults, *hero, carried)).map(|(&h, w)| (h, w.clone())).collect();
        self.gates.iter().flat_map(|g| g.edits(g.mask(&swapped, &carriers))).collect()
    }
}

/// 带的正是自己原版的那几把（不计顺序与强化等级）
fn is_own_loadout(defaults: &BTreeMap<u32, (i32, i32)>, hero: u32, carried: &[u32]) -> bool {
    let Some(&(main, off)) = defaults.get(&hero) else { return false };
    let mut own: Vec<u32> = [main, off].into_iter().filter(|&w| w > 0).map(|w| w as u32).collect();
    let mut carried: Vec<u32> = carried.iter().map(|&w| base(w)).collect();
    own.sort_unstable();
    carried.sort_unstable();
    own == carried
}

/// 武器（本体 ID）-> [(原主人, 承载的类别)]；同一角色同类武器只算第一把（主手优先），同一把武器可有多个原主人
fn carriers(defaults: &BTreeMap<u32, (i32, i32)>, weapons: &WeaponInfo) -> HashMap<u32, Vec<(u32, Kind)>> {
    let mut out: HashMap<u32, Vec<(u32, Kind)>> = HashMap::new();
    for (&hero, &(main, off)) in defaults {
        let mut seen = Vec::new();
        for w in [main, off].into_iter().filter(|&w| w > 0).map(|w| w as u32) {
            let kind = if weapons.is_catalyst(w) { Kind::Magic } else { Kind::Skill };
            if !seen.contains(&kind) {
                seen.push(kind);
                out.entry(w).or_default().push((hero, kind));
            }
        }
    }
    out
}

fn bit(hero_id: u32) -> Option<u16> {
    hero_id.checked_sub(1).filter(|&b| b < 10).map(|b| 1 << b)
}

fn base(id: u32) -> u32 {
    id - id % 100
}

#[cfg(test)]
mod tests {
    use super::*;

    const WYLDER: u32 = 1;
    const IRONEYE: u32 = 3;
    const REVENANT: u32 = 6;
    const RECLUSE: u32 = 7;
    const UNDERTAKER: u32 = 10;
    const ALL: u16 = 0x3FF;

    fn gate(table: GateTable, id: u32, offset: usize, kind: Kind, original: u16) -> Gate {
        let mut row = [0u8; 64];
        for (h, &(byte, bit)) in table.bits().iter().enumerate() {
            if original >> h & 1 != 0 {
                row[byte] |= 1 << bit;
            }
        }
        // 角色位以外的位要原样保留
        row[38] |= 0x80;
        Gate::read(table, id, offset, &row, kind).unwrap()
    }

    fn sample() -> (StartRelics, BTreeMap<u32, (i32, i32)>, WeaponInfo) {
        let relics = StartRelics {
            gates: vec![
                // 改祷告：复仇者；箭雨：铁之眼；某近战战技：追踪者、送葬者；附加：隐士以外
                gate(GateTable::AttachEffect, 7370400, 0x1000, Kind::Magic, bit(REVENANT).unwrap()),
                gate(GateTable::AttachEffect, 7124700, 0x2000, Kind::Skill, bit(IRONEYE).unwrap()),
                gate(GateTable::SwordArts, 227, 0x1000, Kind::Skill, bit(WYLDER).unwrap() | bit(UNDERTAKER).unwrap()),
                gate(GateTable::AttachEffect, 7120100, 0x3000, Kind::Skill, ALL & !bit(RECLUSE).unwrap()),
            ],
        };
        let defaults = BTreeMap::from([
            (WYLDER, (3750000, 30750000)),
            (IRONEYE, (41750000, -1)),
            (REVENANT, (21750000, 34750000)),
            (RECLUSE, (33750000, -1)),
            (UNDERTAKER, (11750000, -1)),
        ]);
        let weapons = WeaponInfo::from_rows([
            (3750000, 5, 0),
            (30750000, 65, 0),
            (41750000, 51, 0),
            (21750000, 35, 0),
            (34750000, 61, 0),
            (33750000, 57, 0),
            (11750000, 21, 0),
        ]);
        (relics, defaults, weapons)
    }

    fn masks(relics: &StartRelics, edits: &[ByteEdit]) -> Vec<u16> {
        relics
            .gates
            .iter()
            .map(|g| {
                let mut row = [0u8; 64];
                for e in edits.iter().filter(|e| e.table == g.table && e.offset >= g.offset && e.offset < g.offset + 64) {
                    row[e.offset - g.offset] = e.value;
                }
                hero_mask(g.table, &row).unwrap()
            })
            .collect()
    }

    #[test]
    fn vanilla_keeps_original_bytes() {
        let (r, d, w) = sample();
        let edits = r.plan(&BTreeMap::new(), &d, &w);
        assert!(edits.iter().all(|e| e.value == e.original));
        // 自己的武器原样带着，结果也不变
        let own = BTreeMap::from([(REVENANT, vec![21750000, 34750000]), (WYLDER, vec![3750000, 30750000])]);
        assert_eq!(r.plan(&own, &d, &w), edits);
    }

    #[test]
    fn swapped_weapons_follow_their_owner() {
        let (r, d, w) = sample();
        let u = bit(UNDERTAKER).unwrap();
        let rev = bit(REVENANT).unwrap();
        // 送葬者拿复仇者的圣印记：获得改祷告，失去近战战技与附加
        let edits = r.plan(&BTreeMap::from([(UNDERTAKER, vec![34750000])]), &d, &w);
        assert_eq!(masks(&r, &edits), [rev | u, bit(IRONEYE).unwrap(), bit(WYLDER).unwrap(), ALL & !bit(RECLUSE).unwrap() & !u]);
        // 送葬者拿铁之眼的弓（带强化等级的 ID 也认）：获得箭雨
        let edits = r.plan(&BTreeMap::from([(UNDERTAKER, vec![41750002])]), &d, &w);
        assert_eq!(masks(&r, &edits)[1], bit(IRONEYE).unwrap() | u);
        // 拿追踪者的小盾不算：追踪者的战技只算在主手大剑上
        let edits = r.plan(&BTreeMap::from([(UNDERTAKER, vec![30750000])]), &d, &w);
        assert_eq!(masks(&r, &edits)[2], bit(WYLDER).unwrap());
        // 角色位以外的位不动
        assert!(edits.iter().filter(|e| e.offset == 0x1000 + 38).all(|e| e.value & 0x80 != 0));
    }

    #[test]
    fn shared_default_weapon_counts_for_every_owner() {
        let (r, mut d, w) = sample();
        // 空城 0.46：送葬者默认副手就是复仇者的圣印记，后写入的原主人不能顶掉复仇者
        d.insert(UNDERTAKER, (11750000, 34750000));
        let (rev, u, wy) = (bit(REVENANT).unwrap(), bit(UNDERTAKER).unwrap(), bit(WYLDER).unwrap());
        let prayer = |carried: BTreeMap<u32, Vec<u32>>| masks(&r, &r.plan(&carried, &d, &w))[0];
        assert_eq!(prayer(BTreeMap::from([(WYLDER, vec![34750000, 30750000])])), rev | wy);
        assert_eq!(prayer(BTreeMap::from([(UNDERTAKER, vec![34750000, 30750000])])), rev | u);
        // 换回自己原版组合（顺序不同也算）视同没换
        let own = BTreeMap::from([(UNDERTAKER, vec![34750000, 11750000])]);
        assert!(r.plan(&own, &d, &w).iter().all(|e| e.value == e.original));
    }

    #[test]
    fn imbue_follows_non_catalyst_weapons() {
        let (r, d, w) = sample();
        let imbue = |carried: BTreeMap<u32, Vec<u32>>| masks(&r, &r.plan(&carried, &d, &w))[3];
        let vanilla = ALL & !bit(RECLUSE).unwrap();
        // 隐士换追踪者的大剑：获得附加，其他角色不变
        assert_eq!(imbue(BTreeMap::from([(RECLUSE, vec![3750000])])), ALL);
        // 隐士换铁之眼的弓：铁之眼原本可附加，同样获得
        assert_eq!(imbue(BTreeMap::from([(RECLUSE, vec![41750000])])), ALL);
        // 追踪者主手换隐士的法杖、留小盾：小盾不算追踪者的武器开关，附加关掉
        assert_eq!(imbue(BTreeMap::from([(WYLDER, vec![33750000, 30750000])])), vanilla & !bit(WYLDER).unwrap());
    }
}
