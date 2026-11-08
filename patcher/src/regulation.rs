//! regulation.bin 的完整读写：AES 解密 → DCX(ZSTD) 解压 → BND4 → SpEffectParam 加行 → 逆序写回。
//! 输入里已有本模组 ID 的行会被替换，所以对同一个文件重复打补丁结果相同（升级时也直接重打即可）。

use std::collections::HashSet;

use nr_stat_alloc_shared::param::ParamFile;
use nr_stat_alloc_shared::param::layout::sp_effect;

use crate::bnd4::Bnd4;
use crate::dcx::Dcx;
use crate::param_write::{self, OwnedRow};
use crate::{components, crypto};

const SP_EFFECT_FILE: &str = "\\SpEffectParam.param";
/// 本模组写入的行名前缀，用来区分"上次打的补丁"和"其他模组恰好用了同一个 ID"
const NAME_PREFIX: &str = "[NRStatAlloc]";

pub struct Report {
    pub version: String,
    pub rows_before: usize,
    pub rows_after: usize,
    /// 输入里原有、被替换掉的同 ID 行
    pub replaced: usize,
    /// 其中不是本模组写入的行（其他模组占用了同一个 ID）
    pub foreign: usize,
    pub added: usize,
}

pub struct Regulation {
    dcx: Dcx,
    bnd: Bnd4,
}

impl Regulation {
    pub fn decode(file: &[u8]) -> Result<Self, String> {
        let plain = crypto::decrypt(file)?;
        let (dcx, raw) = Dcx::decompress(&plain)?;
        Ok(Self { dcx, bnd: Bnd4::parse(raw)? })
    }

    pub fn version(&self) -> String {
        self.bnd.version()
    }

    /// SpEffectParam 原始数据
    pub fn sp_effect(&self) -> Result<&[u8], String> {
        self.bnd.find(SP_EFFECT_FILE).map(|(_, d)| d).ok_or("SpEffectParam not found in regulation".into())
    }

    /// 按文件名（不含扩展名，如 "NpcParam"）找参数表原始数据
    pub fn param(&self, file_name: &str) -> Option<&[u8]> {
        self.bnd.find(&format!("\\{file_name}.param")).map(|(_, d)| d)
    }

    /// 用新数据替换若干参数表（文件名同 param）后编码为新的 regulation.bin
    pub fn encode_with(&self, replacements: &[(&str, Vec<u8>)]) -> Result<Vec<u8>, String> {
        let mut indexed = Vec::with_capacity(replacements.len());
        for (name, data) in replacements {
            let (index, _) = self.bnd.find(&format!("\\{name}.param")).ok_or(format!("{name} not found in regulation"))?;
            indexed.push((index, data.as_slice()));
        }
        let packed = self.dcx.compress(&self.bnd.rebuild_many(&indexed))?;
        crypto::encrypt(&packed)
    }

    /// 加入组件后编码为新的 regulation.bin
    pub fn patch(&self) -> Result<(Vec<u8>, Report), String> {
        let (index, data) = self.bnd.find(SP_EFFECT_FILE).ok_or("SpEffectParam not found in regulation")?;
        let table = ParamFile::parse(data, sp_effect::TABLE.row_size).map_err(|e| format!("SpEffectParam: {e}"))?;
        if table.struct_name() != Some(sp_effect::TABLE.struct_name) {
            return Err(format!("SpEffectParam has unexpected type {:?}", table.struct_name()));
        }
        let added = components::rows(&table)?;
        let ids: HashSet<u32> = added.iter().map(|r| r.id).collect();
        let rows = param_write::rows_of(&table);
        let existing: Vec<&OwnedRow> = rows.iter().filter(|r| ids.contains(&r.id)).collect();
        let foreign = existing.iter().filter(|r| !r.name.as_deref().is_some_and(|n| n.starts_with(NAME_PREFIX))).count();
        let replaced = existing.len();
        let added_count = added.len();
        let rows = param_write::merge(rows, added);
        let report = Report {
            version: self.version(),
            rows_before: table.len(),
            rows_after: rows.len(),
            replaced,
            foreign,
            added: added_count,
        };
        let param = param_write::write_ordered(&table, &rows)?;
        let raw = self.bnd.rebuild(index, &param);
        let packed = self.dcx.compress(&raw)?;
        Ok((crypto::encrypt(&packed)?, report))
    }
}
