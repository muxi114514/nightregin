//! 重新写出 PARAM 文件，布局与 SoulsFormats 相同（不改数据时可逐字节还原原版文件）：
//! 头 0x40 → 行描述 → 行数据 → 类型名 → 共享的空行名 → 有名字的行依次写 UTF-16 名字 → 结尾 2 字节 0。

use nr_stat_alloc_shared::param::{DESCRIPTOR_SIZE, HEADER_SIZE, ParamFile};

pub struct OwnedRow {
    pub id: u32,
    pub data: Vec<u8>,
    pub name: Option<String>,
}

/// 原表全部行（保留行名）
pub fn rows_of(file: &ParamFile) -> Vec<OwnedRow> {
    file.rows().map(|r| OwnedRow { id: r.id, data: r.data.to_vec(), name: file.name(r.index) }).collect()
}

/// 把 added 合进 rows：同 ID 的行原位替换，其余加入。
/// rows 有序且不重复时结果按 ID 排序（与只做插入的旧输出逐字节相同）；
/// rows 乱序或有重复时（如空城包）原有行顺序不动，新行按 ID 升序追加在末尾。
/// 游戏加载参数表时用 std::map 按 ID 重建有序索引（nightreign.exe 0x286AB50，重复 ID 取文件中先出现的行），文件内顺序不影响查找
pub fn merge(mut rows: Vec<OwnedRow>, added: Vec<OwnedRow>) -> Vec<OwnedRow> {
    let ordered = rows.windows(2).all(|w| w[0].id < w[1].id);
    let mut pending: std::collections::BTreeMap<u32, OwnedRow> = added.into_iter().map(|r| (r.id, r)).collect();
    for row in rows.iter_mut() {
        if let Some(new) = pending.remove(&row.id) {
            *row = new;
        }
    }
    rows.extend(pending.into_values());
    if ordered {
        rows.sort_by_key(|r| r.id);
    }
    rows
}

/// 以 original 的头为模板写出 rows（调用方保证按 ID 升序且不重复）
pub fn write(original: &ParamFile, rows: &[OwnedRow]) -> Result<Vec<u8>, String> {
    if rows.windows(2).any(|w| w[0].id >= w[1].id) {
        return Err("rows must be sorted by unique ID".into());
    }
    write_ordered(original, rows)
}

/// 按给定顺序写出，不检查排序与重复（用于保留其他模组表里原有的乱序 / 重复行）
pub fn write_ordered(original: &ParamFile, rows: &[OwnedRow]) -> Result<Vec<u8>, String> {
    if rows.len() > u16::MAX as usize {
        return Err(format!("{} rows exceed the PARAM limit of 65535", rows.len()));
    }
    let row_size = original.row_size();
    if let Some(r) = rows.iter().find(|r| r.data.len() != row_size) {
        return Err(format!("row {} has {} bytes, expected {row_size}", r.id, r.data.len()));
    }
    let type_name = original.struct_name().ok_or("param type name missing")?;

    let data_start = HEADER_SIZE + rows.len() * DESCRIPTOR_SIZE;
    let strings = data_start + rows.len() * row_size;
    let empty_name = strings + type_name.len() + 1;
    let mut names = Vec::new();
    let mut name_offsets = Vec::with_capacity(rows.len());
    for r in rows {
        match r.name.as_deref().filter(|n| !n.is_empty()) {
            Some(n) => {
                name_offsets.push(empty_name + 2 + names.len());
                names.extend(n.encode_utf16().chain([0]).flat_map(u16::to_le_bytes));
            }
            None => name_offsets.push(empty_name),
        }
    }

    let mut out = original.bytes()[..HEADER_SIZE].to_vec();
    out[0x00..0x04].copy_from_slice(&(strings as u32).to_le_bytes());
    out[0x0A..0x0C].copy_from_slice(&(rows.len() as u16).to_le_bytes());
    out[0x10..0x18].copy_from_slice(&(strings as u64).to_le_bytes());
    out[0x30..0x38].copy_from_slice(&(data_start as u64).to_le_bytes());
    for (i, r) in rows.iter().enumerate() {
        out.extend_from_slice(&r.id.to_le_bytes());
        out.extend_from_slice(&[0; 4]);
        out.extend_from_slice(&((data_start + i * row_size) as u64).to_le_bytes());
        out.extend_from_slice(&(name_offsets[i] as u64).to_le_bytes());
    }
    for r in rows {
        out.extend_from_slice(&r.data);
    }
    out.extend_from_slice(type_name.as_bytes());
    out.extend_from_slice(&[0, 0, 0]);
    out.extend_from_slice(&names);
    out.extend_from_slice(&[0, 0]);
    Ok(out)
}

#[cfg(test)]
mod merge_tests {
    use super::*;

    fn rows(ids: &[u32], tag: u8) -> Vec<OwnedRow> {
        ids.iter().map(|&id| OwnedRow { id, data: vec![tag], name: None }).collect()
    }

    fn shape(rows: &[OwnedRow]) -> Vec<(u32, u8)> {
        rows.iter().map(|r| (r.id, r.data[0])).collect()
    }

    #[test]
    fn sorted_table_stays_sorted() {
        let merged = merge(rows(&[10, 20, 30], 0), rows(&[25, 20, 5], 1));
        assert_eq!(shape(&merged), [(5, 1), (10, 0), (20, 1), (25, 1), (30, 0)]);
    }

    #[test]
    fn unsorted_table_keeps_order_and_appends() {
        // 空城包式乱序 + 重复：原有行位置不动，同 ID 只替换先出现的那行，新行升序追加
        let merged = merge(rows(&[900, 100, 500, 500], 0), rows(&[999, 500, 950], 1));
        assert_eq!(shape(&merged), [(900, 0), (100, 0), (500, 1), (500, 0), (950, 1), (999, 1)]);
    }
}
