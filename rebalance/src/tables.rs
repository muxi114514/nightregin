//! 从一份 regulation 打开重平衡需要的参数表，并按 ID 读写行内字段

use nr_stat_alloc_patcher::regulation::Regulation;
use nr_stat_alloc_shared::param::ParamFile;

use crate::layout::{Table, atk, behavior, bullet, npc, sp_effect};

pub struct Tables<'a> {
    pub npc: ParamFile<'a>,
    pub behavior: ParamFile<'a>,
    pub bullet: ParamFile<'a>,
    pub atk: ParamFile<'a>,
    pub sp_effect: ParamFile<'a>,
}

impl<'a> Tables<'a> {
    pub fn load(reg: &'a Regulation) -> Result<Self, String> {
        Ok(Self {
            npc: open(reg, &npc::TABLE)?,
            behavior: open(reg, &behavior::TABLE)?,
            bullet: open(reg, &bullet::TABLE)?,
            atk: open(reg, &atk::TABLE)?,
            sp_effect: open(reg, &sp_effect::TABLE)?,
        })
    }
}

pub fn open<'a>(reg: &'a Regulation, t: &Table) -> Result<ParamFile<'a>, String> {
    let bytes = reg.param(t.file).ok_or(format!("{} not found in regulation", t.file))?;
    let file = ParamFile::parse(bytes, t.row_size).map_err(|e| format!("{}: {e}", t.file))?;
    if file.struct_name() != Some(t.struct_name) {
        return Err(format!("{} has unexpected type {:?}", t.file, file.struct_name()));
    }
    Ok(file)
}

pub fn i32_at(row: &[u8], offset: usize) -> i32 {
    i32::from_le_bytes(row[offset..offset + 4].try_into().unwrap())
}

pub fn u32_at(row: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(row[offset..offset + 4].try_into().unwrap())
}

pub fn put(row: &mut [u8], offset: usize, value: [u8; 4]) {
    row[offset..offset + 4].copy_from_slice(&value);
}
