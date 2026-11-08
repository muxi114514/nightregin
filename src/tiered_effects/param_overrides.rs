//! 改内存参数表的锻体（shared::tiered_params）：启动时记下各字段的地址与原值，持有标记时按份数写入新值，回到圆桌还原。
//! 参数表只在本机生效，不影响队友。

use log::{info, warn};
use nr_stat_alloc_shared::tiered_params::OVERRIDES;

use crate::game::param_patch::{PatchField, locate_rows};
use crate::game::params::ParamRepository;

use super::Frame;

/// 一个锻体标记要改的全部字段
struct Group {
    marker: i32,
    fields: Vec<PatchField>,
    /// 已按几份写入（0 = 原值）
    applied: u32,
}

pub struct ParamOverrides {
    groups: Vec<Group>,
}

impl ParamOverrides {
    /// 启动时（参数表还是原值）定位全部字段；一个都找不到时为 None
    pub fn locate(repository: &ParamRepository) -> Option<Self> {
        let mut groups: Vec<Group> = Vec::new();
        for o in OVERRIDES {
            let fields = locate_rows(repository, o.table, o.rows, o.offset, o.change);
            if fields.len() < o.rows.len() {
                warn!("tiered params: buff {} found {}/{} rows of {}", o.marker, fields.len(), o.rows.len(), o.table.struct_name);
            }
            match groups.iter_mut().find(|g| g.marker == o.marker) {
                Some(g) => g.fields.extend(fields),
                None => groups.push(Group { marker: o.marker, fields, applied: 0 }),
            }
        }
        groups.retain(|g| !g.fields.is_empty());
        info!("tiered params: {} buffs, {} fields", groups.len(), groups.iter().map(|g| g.fields.len()).sum::<usize>());
        (!groups.is_empty()).then_some(Self { groups })
    }

    /// 出击中只升不降：标记偶尔不在列表里时不来回切
    pub fn tick(&mut self, frame: &Frame) {
        for g in &mut self.groups {
            let stacks = frame.ids.iter().filter(|&&id| id == g.marker).count() as u32;
            if stacks > g.applied {
                g.set(stacks);
            }
        }
    }

    /// 回到圆桌：还原原值
    pub fn reset(&mut self) {
        for g in self.groups.iter_mut().filter(|g| g.applied > 0) {
            g.set(0);
        }
    }
}

impl Group {
    fn set(&mut self, stacks: u32) {
        let failed = self.fields.iter().filter(|f| !if stacks == 0 { f.restore() } else { f.write(stacks) }).count();
        if failed > 0 {
            warn!("tiered params: buff {}: {failed} writes failed", self.marker);
        }
        info!("tiered params: buff {} x{} -> x{stacks}", self.marker, self.applied);
        self.applied = stacks;
    }
}
