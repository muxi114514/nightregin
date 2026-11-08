//! 隐士的混合魔法（静态逆向，2026-11-02）。PlayerIns+0x1B8（角色模块表）+0x180 是 CSChrMixcraftMagicModule，模块 +0x10 起三个字节是属性槽。
//! 0x45C640 数非空槽（HKS 的 GetMagicCocktailAffinityCount，等于 3 时技艺放魔法）、0x45B480 吸收时填第一个空槽、
//! 0x45C5F0 按三个槽查 MixcraftMagicParam 得魔法号（64 种排列都有行）、0x45C700 是放魔法动作里清空三槽的事件。

use super::addresses::Addresses;
use super::memory::{looks_like_pointer, read, read_ptr, write};
use super::rtti::has_vtable;

const CHR_MODULES: usize = 0x1B8;
const MIXCRAFT_MODULE: usize = 0x180;
const SLOTS: usize = 0x10;

pub fn module(addrs: &Addresses, player: usize) -> Option<usize> {
    let vtable = addrs.vt_mixcraft_module?;
    read_ptr(player + CHR_MODULES).and_then(|m| read_ptr(m + MIXCRAFT_MODULE)).filter(|&m| looks_like_pointer(m) && has_vtable(m, vtable))
}

pub fn slots(module: usize) -> Option<[u8; 3]> {
    read::<[u8; 3]>(module + SLOTS)
}

pub fn set_slots(module: usize, slots: [u8; 3]) -> bool {
    write(module + SLOTS, slots)
}
