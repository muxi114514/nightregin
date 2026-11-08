//! 遗物：当前角色装备的 6 件遗物与遗物实例（CSAtqGaitemIns）的词条读写。
//! 实测布局（2026-10-27，背包 1502 件逐件核对）：PlayerGameData+0x2F4 为当前角色 6 个遗物句柄（3 普通 + 3 深夜，0 为空）；
//! 实例 +0x08 句柄、+0x0C 物品 ID（高 4 位类别）、+0x18 起 3 个正面词条、+0x40 起 3 个负面词条（第 i 个负面配第 i 个正面），
//! +0x5C 起 18 个出征时施加的 SpEffect（按词条生成的缓存）、+0xA4 低字节为缓存长度。
//! 只改词条不改缓存，界面会显示新词条但出征生效的仍是旧效果，所以两处一起写（写入后界面、属性与读档实测都正确）。

use nr_stat_alloc_shared::relic_reroll::{RelicEffects, SLOTS};

use super::memory::{read, write};

const PGD_EQUIPPED: usize = 0x2F4;
pub const EQUIPPED_SLOTS: usize = 6;
/// 遗物实例句柄的类别（高 4 位）
const RELIC_HANDLE: u32 = 0xC;
const INS_ITEM_ID: usize = 0xC;
const POSITIVE: usize = 0x18;
const CURSE: usize = 0x40;
const CACHE: usize = 0x5C;
pub const CACHE_SLOTS: usize = 18;
const CACHE_LEN: usize = 0xA4;
const ITEM_ID_MASK: u32 = 0x0FFF_FFFF;

/// 当前角色装备的遗物句柄，空槽为 None
pub fn equipped(game_data: usize) -> Option<[Option<u32>; EQUIPPED_SLOTS]> {
    let raw = read::<[u32; EQUIPPED_SLOTS]>(game_data + PGD_EQUIPPED)?;
    Some(raw.map(|h| (h >> 28 == RELIC_HANDLE && h != u32::MAX).then_some(h)))
}

#[derive(Debug, Clone, Copy)]
pub struct RelicIns(pub usize);

impl RelicIns {
    /// EquipParamAntique 行号
    pub fn item_id(&self) -> Option<u32> {
        read::<u32>(self.0 + INS_ITEM_ID).map(|id| id & ITEM_ID_MASK)
    }

    pub fn effects(&self) -> Option<RelicEffects> {
        let slot = |id: i32| u32::try_from(id).ok();
        let positive = read::<[i32; SLOTS]>(self.0 + POSITIVE)?;
        let curse = read::<[i32; SLOTS]>(self.0 + CURSE)?;
        Some(RelicEffects { positive: positive.map(slot), curse: curse.map(slot) })
    }

    /// 写入词条与 SpEffect 缓存（缓存超过 18 个时拒绝，不写任何东西）
    pub fn write(&self, effects: &RelicEffects, cache: &[i32]) -> bool {
        if cache.len() > CACHE_SLOTS {
            return false;
        }
        let ids = |slots: &[Option<u32>; SLOTS]| slots.map(|s| s.map_or(-1, |id| id as i32));
        let mut padded = [-1i32; CACHE_SLOTS];
        padded[..cache.len()].copy_from_slice(cache);
        write(self.0 + CACHE, padded)
            && write(self.0 + CACHE_LEN, cache.len() as u8)
            && write(self.0 + POSITIVE, ids(&effects.positive))
            && write(self.0 + CURSE, ids(&effects.curse))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_and_writes_instance_layout() {
        let mut ins = [-1i32; 0xA8 / 4];
        ins[INS_ITEM_ID / 4] = 0x800F_5DAEu32 as i32;
        ins[POSITIVE / 4] = 7036800;
        ins[CURSE / 4 + 2] = 6850900;
        ins[CACHE_LEN / 4] = 0xFFFF_FF02u32 as i32;
        let r = RelicIns(ins.as_mut_ptr() as usize);
        assert_eq!(r.item_id(), Some(1007022));
        let e = r.effects().unwrap();
        assert_eq!((e.positive, e.curse), ([Some(7036800), None, None], [None, None, Some(6850900)]));

        let new = RelicEffects { positive: [Some(7000002), Some(7001400), None], curse: [None; SLOTS] };
        assert!(r.write(&new, &[7000002, 7001400]));
        assert_eq!(r.effects(), Some(new));
        assert_eq!(&ins[CACHE / 4..CACHE / 4 + 3], &[7000002, 7001400, -1]);
        // 只改长度字节，同一个 u32 的高位保持原样
        assert_eq!(ins[CACHE_LEN / 4] as u32, 0xFFFF_FF02);
        assert!(!r.write(&new, &[1; CACHE_SLOTS + 1]));
    }

    #[test]
    fn equipped_filters_non_relic_handles() {
        let pgd = [0u8; 0x320];
        let mut buf = pgd;
        for (i, h) in [0xC080_0337u32, 0, 0x8080_063E, 0xC080_0215, u32::MAX, 0xC080_02E0].iter().enumerate() {
            buf[PGD_EQUIPPED + i * 4..PGD_EQUIPPED + i * 4 + 4].copy_from_slice(&h.to_le_bytes());
        }
        let got = equipped(buf.as_ptr() as usize).unwrap();
        assert_eq!(got, [Some(0xC080_0337), None, None, Some(0xC080_0215), None, Some(0xC080_02E0)]);
    }
}
