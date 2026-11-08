//! 物品实例仓库 CSGaitemImp：按句柄找到武器实例（CSWepGaitemIns）并读写其词条；遗物实例见 relic.rs。
//! CSGaitemImp +0x08 为 0x1400 个实例指针，+0xA008 为同序号的 (句柄, 引用计数)；实例 +0x08 句柄、+0x0C 物品 ID，
//! 武器实例大小 0x108（构造函数 0x6A2640 与反序列化 0x6A28D0 静态分析）。
//! 词条在 +0x18 起的武器数据块里（反序列化读的就是这一块），+0x60 是装备时据此重建的缓存（2026-10-26 实机验证）。

use super::addresses::GaitemAddresses;
use super::memory::{read, read_bytes, read_ptr, write};
use super::rtti::has_vtable;

const SLOTS: usize = 0x1400;
const INSTANCES: usize = 0x8;
const INS_HANDLE: usize = 0x8;
const INS_ITEM_ID: usize = 0xC;
const WEAPON_SIZE: usize = 0x108;
/// 3 个词条 ID，空槽为 -1；界面直接读这里
const PASSIVE_SLOTS: [usize; 3] = [0x20, 0x24, 0x28];
pub const PASSIVE_COUNT: usize = PASSIVE_SLOTS.len();
/// 物品 ID 高 4 位是类别
const ITEM_ID_MASK: u32 = 0x0FFF_FFFF;

#[derive(Debug, Clone)]
pub struct Gaitems {
    imp_static: usize,
    vt_imp: usize,
    vt_weapon: usize,
    vt_relic: Option<usize>,
}

#[derive(Debug, Clone, Copy)]
pub struct WeaponIns(usize);

impl Gaitems {
    pub fn new(addrs: &GaitemAddresses) -> Self {
        Self { imp_static: addrs.gaitem_imp_static, vt_imp: addrs.vt_gaitem_imp, vt_weapon: addrs.vt_wep_gaitem_ins, vt_relic: addrs.vt_atq_gaitem_ins }
    }

    /// 句柄对应的武器实例；不是武器或已失效时返回 None
    pub fn weapon(&self, handle: u32) -> Option<WeaponIns> {
        self.find(handle, self.vt_weapon).map(WeaponIns)
    }

    /// 句柄对应的遗物实例地址（CSAtqGaitemIns）；遗物 vtable 未校验通过时总是 None
    pub fn relic(&self, handle: u32) -> Option<usize> {
        self.find(handle, self.vt_relic?)
    }

    fn find(&self, handle: u32, vtable: usize) -> Option<usize> {
        let imp = read_ptr(self.imp_static).filter(|&p| has_vtable(p, self.vt_imp))?;
        let is_match = |p: usize| p != 0 && has_vtable(p, vtable) && read::<u32>(p + INS_HANDLE) == Some(handle);
        // 句柄低 16 位通常就是槽位序号，先直接查；对不上再整表找（只在强化 / 重铸时会走到，开销可忽略）
        let index = (handle & 0xFFFF) as usize;
        if index < SLOTS
            && let Some(p) = read_ptr(imp + INSTANCES + index * 8).filter(|&p| is_match(p))
        {
            return Some(p);
        }
        let table = read_bytes(imp + INSTANCES, SLOTS * 8)?;
        table.chunks_exact(8).map(|c| usize::from_le_bytes(c.try_into().unwrap())).find(|&p| is_match(p))
    }
}

impl WeaponIns {
    pub fn address(&self) -> usize {
        self.0
    }

    /// 去掉类别位的物品 ID（EquipParamWeapon 行号，末位是强化等级）
    pub fn item_id(&self) -> Option<u32> {
        read::<u32>(self.0 + INS_ITEM_ID).map(|id| id & ITEM_ID_MASK)
    }

    /// 第 slot 个词条（AttachEffectParam ID）；空槽或不可读时为 None
    pub fn passive(&self, slot: usize) -> Option<u32> {
        let offset = *PASSIVE_SLOTS.get(slot)?;
        read::<i32>(self.0 + offset).and_then(|id| u32::try_from(id).ok())
    }

    pub fn set_passive(&self, slot: usize, id: u32) -> bool {
        PASSIVE_SLOTS.get(slot).is_some_and(|&offset| write(self.0 + offset, id as i32))
    }

    /// 三个词条槽的原始值（空槽 -1），整把覆盖用
    pub fn passive_slots(&self) -> Option<[i32; PASSIVE_COUNT]> {
        read(self.0 + PASSIVE_SLOTS[0])
    }

    pub fn set_passive_slots(&self, slots: [i32; PASSIVE_COUNT]) -> bool {
        write(self.0 + PASSIVE_SLOTS[0], slots)
    }

    /// 整个实例的副本（诊断用）
    pub fn snapshot(&self) -> Option<Vec<u8>> {
        read_bytes(self.0, WEAPON_SIZE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_and_writes_passive_slots() {
        let mut ins = [-1i32; WEAPON_SIZE / 4];
        ins[0x20 / 4] = 8330400;
        let w = WeaponIns(ins.as_mut_ptr() as usize);
        assert_eq!((w.passive(0), w.passive(1), w.passive(PASSIVE_COUNT)), (Some(8330400), None, None));
        assert!(w.set_passive(0, 8330401) && !w.set_passive(PASSIVE_COUNT, 1));
        assert_eq!(ins[0x20 / 4], 8330401);
        assert_eq!(w.passive_slots(), Some([8330401, -1, -1]));
        assert!(w.set_passive_slots([-1, 8200200, 8350000]));
        assert_eq!(&ins[0x20 / 4..0x2C / 4], &[-1, 8200200, 8350000]);
        assert_eq!(ins[0x2C / 4], -1);
    }
}
