//! 强化时武器实例的诊断输出：按 4 字节列出非空字段，标出其中的词条 ID 与上位档，用来确认词条槽偏移。

use nr_stat_alloc_shared::passive_tiers::PassiveTiers;

pub fn annotate(bytes: &[u8], tiers: &PassiveTiers) -> String {
    bytes
        .chunks_exact(4)
        .enumerate()
        .filter_map(|(i, c)| {
            let v = i32::from_le_bytes(c.try_into().unwrap());
            if v == 0 || v == -1 {
                return None;
            }
            let off = i * 4;
            let id = v as u32;
            Some(match (v > 0 && tiers.is_known(id), tiers.next(id)) {
                (true, Some(up)) => format!("+{off:02x}={v}[AE->{up}]"),
                (true, None) => format!("+{off:02x}={v}[AE top]"),
                _ => format!("+{off:02x}={v:#x}"),
            })
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marks_attach_effect_ids() {
        let tiers = PassiveTiers::from_rows([(8670000, 8670000, 7), (8670001, 8670000, 10)]);
        let words: [i32; 4] = [0, 8670000, -1, 8670001];
        let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
        assert_eq!(annotate(&bytes, &tiers), "+04=8670000[AE->8670001] +0c=8670001[AE top]");
    }
}
