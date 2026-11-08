//! 写回 schemes.toml：用 toml_edit 只改动目标那条方案（按"角色 + 器皿"匹配），注释与其它方案原样保留。
//! 先写临时文件再替换；本次进程第一次写入前备份为 schemes.toml.bak。

use std::io::ErrorKind;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use toml_edit::{ArrayOfTables, DocumentMut, InlineTable, Item, Table, value};

use crate::stats::heroes::{HERO_NAMES_ZH, parse_hero};
use crate::stats::rules::SchemeSpec;

use super::{FILE_NAME, STAT_KEYS};

static BACKED_UP: AtomicBool = AtomicBool::new(false);

/// 写入目标：(角色序号, 器皿 ID；None 为角色默认方案)
pub type Target = (usize, Option<u32>);

/// 器皿名与 ID 的互查（由游戏数据提供，测试里可替换）
pub trait VesselLookup {
    /// 方案里的器皿写法（中文名或 ID）-> 器皿 ID；不属于该角色时为 None
    fn resolve(&self, hero: usize, key: &str) -> Option<u32>;
    fn name(&self, id: u32) -> Option<&str>;
}

pub fn save(dir: &Path, vessels: &dyn VesselLookup, target: Target, spec: &SchemeSpec) -> Result<(), String> {
    edit(dir, |doc| {
        let schemes = schemes_mut(doc)?;
        let found = matching(schemes, vessels, target);
        // 同一目标写了多条时只保留最后一条（读取时也是后者生效）
        for &i in found.iter().rev().skip(1) {
            schemes.remove(i);
        }
        match found.last() {
            Some(&last) => fill(schemes.get_mut(last - (found.len() - 1)).expect("index checked"), spec),
            None => {
                let mut t = Table::new();
                t.decor_mut().set_prefix("\n");
                t["hero"] = value(HERO_NAMES_ZH[target.0]);
                if let Some(id) = target.1 {
                    let name = vessels.name(id).map_or_else(|| id.to_string(), str::to_string);
                    t["vessel"] = value(name);
                }
                fill(&mut t, spec);
                schemes.push(t);
            }
        }
        Ok(())
    })
}

/// 删除目标方案；返回是否删掉了东西
pub fn remove(dir: &Path, vessels: &dyn VesselLookup, target: Target) -> Result<bool, String> {
    edit(dir, |doc| {
        let schemes = schemes_mut(doc)?;
        let found = matching(schemes, vessels, target);
        for &i in found.iter().rev() {
            schemes.remove(i);
        }
        Ok(!found.is_empty())
    })
}

fn edit<R>(dir: &Path, f: impl FnOnce(&mut DocumentMut) -> Result<R, String>) -> Result<R, String> {
    let path = dir.join(FILE_NAME);
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) if e.kind() == ErrorKind::NotFound => String::new(),
        Err(e) => return Err(crate::tr!("读取 {FILE_NAME} 失败：{e}", "Failed to read {FILE_NAME}: {e}")),
    };
    let mut doc: DocumentMut = text.parse().map_err(|e| crate::tr!("{FILE_NAME} 格式错误：{e}", "{FILE_NAME} has invalid format: {e}"))?;
    let result = f(&mut doc)?;
    if path.exists() && !BACKED_UP.swap(true, Ordering::AcqRel) {
        std::fs::copy(&path, path.with_extension("toml.bak")).map_err(|e| crate::tr!("备份 {FILE_NAME} 失败：{e}", "Failed to back up {FILE_NAME}: {e}"))?;
    }
    let tmp = path.with_extension("toml.tmp");
    std::fs::write(&tmp, doc.to_string()).map_err(|e| crate::tr!("写入 {FILE_NAME} 失败：{e}", "Failed to write {FILE_NAME}: {e}"))?;
    std::fs::rename(&tmp, &path).map_err(|e| crate::tr!("替换 {FILE_NAME} 失败：{e}", "Failed to replace {FILE_NAME}: {e}"))?;
    Ok(result)
}

fn schemes_mut(doc: &mut DocumentMut) -> Result<&mut ArrayOfTables, String> {
    doc.as_table_mut()
        .entry("scheme")
        .or_insert(Item::ArrayOfTables(ArrayOfTables::new()))
        .as_array_of_tables_mut()
        .ok_or_else(|| crate::tr!("{FILE_NAME} 里的 scheme 不是 [[scheme]] 写法，无法自动保存", "{FILE_NAME}: scheme is not written as [[scheme]], cannot save automatically"))
}

/// 与目标相同的方案下标（升序）；器皿写错、角色写错的条目不会匹配
fn matching(schemes: &ArrayOfTables, vessels: &dyn VesselLookup, (hero, vessel): Target) -> Vec<usize> {
    let key_of = |t: &Table| -> Option<Target> {
        let h = parse_hero(t.get("hero")?.as_str()?)?;
        let v = match t.get("vessel").and_then(Item::as_str).map(str::trim) {
            None | Some("") => None,
            Some(name) => Some(vessels.resolve(h, name)?),
        };
        Some((h, v))
    };
    schemes.iter().enumerate().filter(|(_, t)| key_of(t) == Some((hero, vessel))).map(|(i, _)| i).collect()
}

/// 用 spec 覆盖条目里的方案字段（hero / vessel 及条目前的注释保留）
fn fill(t: &mut Table, spec: &SchemeSpec) {
    // arcane_release 是已废弃的旧写法，顺手清掉
    for key in ["arcane_release", "level1", "growth", "levels", "weapons"] {
        t.remove(key);
    }
    let w = &spec.weapons;
    if !w.is_vanilla() {
        let mut weapons = InlineTable::new();
        for (key, token) in [("main", &w.main), ("off", &w.off)] {
            if let Some(token) = token {
                weapons.insert(key, token.as_str().into());
            }
        }
        weapons.fmt();
        t["weapons"] = value(weapons);
    }
    let level1 = inline(STAT_KEYS.iter().zip(spec.level1).filter_map(|(k, v)| v.map(|v| (*k, v))));
    if !level1.is_empty() {
        t["level1"] = value(level1);
    }
    let growth = inline(STAT_KEYS.iter().zip(spec.weights).filter(|(_, w)| *w > 0).map(|(k, w)| (*k, w)));
    if !growth.is_empty() {
        t["growth"] = value(growth);
    }
    if !spec.overrides.is_empty() {
        let mut levels = Table::new();
        for (level, gains) in &spec.overrides {
            // 全 0 的手动等级写成 {}，读回来仍是"手动、待按权重补齐"
            levels.insert(&level.to_string(), value(inline(STAT_KEYS.iter().zip(gains).filter(|(_, g)| **g > 0).map(|(k, g)| (*k, *g)))));
        }
        t["levels"] = Item::Table(levels);
    }
}

fn inline<'a>(pairs: impl Iterator<Item = (&'a str, u32)>) -> InlineTable {
    let mut table = InlineTable::new();
    for (k, v) in pairs {
        table.insert(k, i64::from(v).into());
    }
    table.fmt();
    table
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::rules::WeaponPick;

    struct Vessels;

    impl VesselLookup for Vessels {
        fn resolve(&self, hero: usize, key: &str) -> Option<u32> {
            match (hero, key) {
                (7, "执行者杯盅" | "8001") => Some(8001),
                (_, "黄金树圣杯" | "19000") => Some(19000),
                _ => None,
            }
        }

        fn name(&self, id: u32) -> Option<&str> {
            match id {
                8001 => Some("执行者杯盅"),
                19000 => Some("黄金树圣杯"),
                _ => None,
            }
        }
    }

    const ORIGINAL: &str = "# 顶部注释\n\n# 执行者默认\n[[scheme]]\nhero = \"执行者\"\ngrowth = { vigor = 7 }\n\n# 杯盅方案\n[[scheme]]\nhero = \"Executor\"\nvessel = \"8001\"\nlevel1 = { strength = 10, dexterity = 3 }\n";

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("nr_stat_alloc_test_{name}_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(FILE_NAME), ORIGINAL).unwrap();
        dir
    }

    fn loaded(dir: &Path) -> Vec<(usize, Option<String>, SchemeSpec)> {
        super::super::load(dir).unwrap().0.into_iter().map(|s| (s.hero, s.vessel, s.spec)).collect()
    }

    #[test]
    fn update_keeps_comments_and_round_trips() {
        let dir = temp_dir("update");
        let mut spec = SchemeSpec { weights: [2, 0, 1, 3, 0, 0, 0], ..Default::default() };
        spec.level1 = [Some(7), Some(2), Some(3), Some(10), Some(3), Some(2), Some(2), Some(28)];
        spec.overrides.insert(5, [4, 0, 0, 6, 0, 0, 0]);
        spec.overrides.insert(6, [0; 7]);
        spec.weapons = WeaponPick { main: Some("追踪者".into()), off: Some("无".into()) };
        save(&dir, &Vessels, (7, Some(8001)), &spec).unwrap();
        let text = std::fs::read_to_string(dir.join(FILE_NAME)).unwrap();
        assert!(text.contains("# 顶部注释") && text.contains("# 杯盅方案") && text.contains("vessel = \"8001\""), "{text}");
        let all = loaded(&dir);
        assert_eq!(all.len(), 2);
        assert_eq!(all[1].2, spec);
        assert_eq!(all[0].2.weights[0], 7);
        assert!(dir.join("schemes.toml.bak").exists() || BACKED_UP.load(Ordering::Acquire));
    }

    #[test]
    fn new_target_is_appended_and_removable() {
        let dir = temp_dir("append");
        let spec = SchemeSpec { weights: [1, 0, 0, 0, 1, 0, 0], ..Default::default() };
        save(&dir, &Vessels, (7, Some(19000)), &spec).unwrap();
        let all = loaded(&dir);
        assert_eq!(all.len(), 3);
        assert_eq!((all[2].0, all[2].1.as_deref()), (7, Some("黄金树圣杯")));
        assert_eq!(all[2].2, spec);
        assert!(remove(&dir, &Vessels, (7, None)).unwrap());
        assert!(!remove(&dir, &Vessels, (8, None)).unwrap());
        let all = loaded(&dir);
        assert_eq!(all.len(), 2);
        assert!(all.iter().all(|s| s.1.is_some()));
    }
}
