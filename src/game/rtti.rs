//! MSVC RTTI：通过 vtable[-1] 的 Complete Object Locator 读出类型名，用来校验对象类型。

use super::memory::{read, read_cstr, read_ptr};
use super::module::GameModule;

/// vtable 对应的修饰类型名，例如 ".?AVPlayerIns@CS@@"
pub fn vtable_type_name(module: &GameModule, vtable: usize) -> Option<String> {
    if !module.contains(vtable) {
        return None;
    }
    let col = read_ptr(vtable.checked_sub(8)?)?;
    if !module.contains(col) || read::<u32>(col)? != 1 {
        return None;
    }
    let type_descriptor_rva = read::<u32>(col + 12)? as usize;
    read_cstr(module.va(type_descriptor_rva) + 0x10, 256)
}

/// 对象（首字段为 vtable 指针）的类型名
pub fn object_type_name(module: &GameModule, object: usize) -> Option<String> {
    vtable_type_name(module, read_ptr(object)?)
}

/// 对象的 vtable 是否等于期望值（热路径用，比读类型名便宜）
pub fn has_vtable(object: usize, expected: usize) -> bool {
    read_ptr(object) == Some(expected)
}
