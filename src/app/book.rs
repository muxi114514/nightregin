//! 方案簿：把 schemes.toml 展开成每个"角色 + 器皿"1~15 级的目标属性；F7 重载时整体替换。
//! 查找顺序：该器皿专属方案 -> 该角色默认方案 -> 原版成长。不合规的方案只影响自己，并在日志写明原因。

use std::collections::HashMap;
use std::path::Path;

use log::{error, info, warn};

use crate::data::GameData;
use crate::scheme_file;
use crate::stats::heroes::HERO_NAMES_ZH;
use crate::stats::rules::{self, Allocation, Limits, SchemeSpec, WeaponPick};

/// (角色序号, 器皿 ID；None 为默认方案)
type Key = (usize, Option<u32>);

pub struct SchemeBook {
    allocations: HashMap<Key, Allocation>,
    /// 原始方案，供面板编辑
    specs: HashMap<Key, SchemeSpec>,
}

impl SchemeBook {
    pub fn empty() -> Self {
        Self { allocations: HashMap::new(), specs: HashMap::new() }
    }

    pub fn load(dir: &Path, data: &GameData, limits: Limits) -> Self {
        let mut book = Self::empty();
        let (schemes, problems) = match scheme_file::load(dir) {
            Ok(r) => r,
            Err(e) => {
                error!("{e}; all heroes use vanilla growth");
                return book;
            }
        };
        for p in problems {
            warn!("{p}");
        }
        for s in schemes {
            let label = format!("{} / {}", HERO_NAMES_ZH[s.hero], s.vessel.as_deref().unwrap_or("默认"));
            let vessel = match s.vessel.as_deref().map(|v| data.resolve_vessel(s.hero, v)).transpose() {
                Ok(v) => v,
                Err(e) => {
                    error!("scheme {label} rejected: {e}");
                    continue;
                }
            };
            match rules::build(&s.spec, &data.growth[s.hero], &[], limits) {
                Ok((alloc, notes)) => {
                    info!("scheme {label}: Lv1 {:?} -> Lv15 {:?}", alloc.at(1), alloc.at(15));
                    for n in notes {
                        info!("scheme {label}: {n}");
                    }
                    book.allocations.insert((s.hero, vessel), alloc);
                    book.specs.insert((s.hero, vessel), s.spec);
                }
                Err(e) => error!("scheme {label} rejected, using vanilla growth: {e}"),
            }
        }
        info!("{} scheme(s) active", book.allocations.len());
        book
    }

    /// 精确查找某个目标的原始方案（不回退）
    pub fn spec(&self, hero: usize, vessel: Option<u32>) -> Option<&SchemeSpec> {
        self.specs.get(&(hero, vessel))
    }

    /// 初始武器选择：器皿专属方案优先，其次角色默认方案（与属性的查找顺序一致）
    pub fn weapons(&self, hero: usize, vessel: Option<u32>) -> Option<&WeaponPick> {
        vessel.and_then(|v| self.specs.get(&(hero, Some(v)))).or_else(|| self.specs.get(&(hero, None))).map(|s| &s.weapons)
    }

    /// 返回方案与它是否为器皿专属方案
    pub fn get(&self, hero: usize, vessel: Option<u32>) -> Option<(&Allocation, bool)> {
        if let Some(v) = vessel {
            if let Some(a) = self.allocations.get(&(hero, Some(v))) {
                return Some((a, true));
            }
        }
        self.allocations.get(&(hero, None)).map(|a| (a, false))
    }
}
