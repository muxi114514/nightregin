//! 右栏：抽取范围。只列出选中词条能抽到的分组，顺序同原版遗物筛选（全部 → 本角色 → 武器 → 减益）；
//! 每组显示可抽词条数与占全部候选权重的比例，悬停查看组内词条在当前勾选下每次抽到的概率。

use std::collections::{BTreeMap, HashSet};

use hudhook::imgui::{Direction, Ui};
use nr_stat_alloc_shared::relic_reroll::{GroupKind, RelicCatalog, Target};

use crate::data::relic_names;
use crate::relic_reroll::{EquippedRelic, RelicView};

use super::super::sfx::{self, Cue};
use super::super::skin::Skin;
use super::super::widgets::{self, Check};
use super::effects::effect_name;
use super::{RerollPanel, tx};

/// 一个分组在本次选中词条下的候选
struct GroupStat {
    id: u32,
    kind: GroupKind,
    /// (词条, 权重)；多条一起重铸时同一词条的权重相加
    effects: Vec<(u32, u64)>,
    weight: u64,
}

/// 权重的合计：全部候选与勾选后
#[derive(Clone, Copy)]
struct Totals {
    all: u64,
    selected: u64,
}

fn group_stats(catalog: &RelicCatalog, relic: &EquippedRelic, targets: &[Target], hero: usize) -> Vec<GroupStat> {
    let mut map: BTreeMap<(GroupKind, i32, u32), GroupStat> = BTreeMap::new();
    for &t in targets {
        let others: Vec<Target> = targets.iter().copied().filter(|&o| o != t).collect();
        for c in catalog.candidates(relic.relic, &relic.effects, t, hero as u32 + 1, &others) {
            let rank = catalog.group_rank(c.group);
            let g = map.entry(rank).or_insert_with(|| GroupStat { id: c.group, kind: rank.0, effects: Vec::new(), weight: 0 });
            g.weight += c.weight as u64;
            match g.effects.iter_mut().find(|e| e.0 == c.effect) {
                Some(e) => e.1 += c.weight as u64,
                None => g.effects.push((c.effect, c.weight as u64)),
            }
        }
    }
    map.into_values().collect()
}

fn state_of(ids: &[u32], excluded: &HashSet<u32>) -> Check {
    let on = ids.iter().filter(|id| !excluded.contains(id)).count();
    match on {
        0 => Check::Off,
        n if n == ids.len() => Check::On,
        _ => Check::Partial,
    }
}

fn percent(part: u64, whole: u64) -> f64 {
    part as f64 * 100.0 / whole.max(1) as f64
}

impl RerollPanel {
    pub(super) fn pools(&mut self, ui: &Ui, skin: &Skin, view: &RelicView, catalog: &RelicCatalog) {
        widgets::section(ui, skin, tx().section_pools);
        let Some(relic) = self.selected().cloned() else { return };
        if !catalog.rerollable(relic.relic) {
            ui.text_disabled(tx().fixed);
            return;
        }
        if self.targets.is_empty() {
            ui.text_disabled(tx().no_target);
            return;
        }
        let hero = view.hero;
        let stats = group_stats(catalog, &relic, &self.targets, hero);
        let excluded = self.excluded_of(hero).clone();
        let totals = Totals {
            all: stats.iter().map(|s| s.weight).sum(),
            selected: stats.iter().filter(|s| !excluded.contains(&s.id)).map(|s| s.weight).sum(),
        };
        let ids: Vec<u32> = stats.iter().map(|s| s.id).collect();
        let included = ids.iter().filter(|id| !excluded.contains(id)).count();
        ui.text_disabled(format!("{} {included} / {}", tx().selected_count, ids.len()));
        {
            let _wrap = ui.push_text_wrap_pos();
            ui.text_disabled(tx().pools_hint);
        }
        ui.spacing();
        let all = state_of(&ids, &excluded);
        if widgets::check(ui, &format!("{}##all", tx().all), all) {
            self.set_groups(hero, &ids, all != Check::On);
        }
        let of = |kind: fn(&GroupKind) -> bool| stats.iter().filter(move |s| kind(&s.kind));
        for g in of(|k| *k == GroupKind::General) {
            self.group_row(ui, catalog, hero, g, totals, 0.0);
        }
        let heroes: Vec<&GroupStat> = of(|k| matches!(k, GroupKind::Hero(_))).collect();
        self.labelled_rows(ui, catalog, hero, tx().hero_group, &heroes, totals);
        let weapons: Vec<&GroupStat> = of(|k| *k == GroupKind::Weapon).collect();
        self.weapon_block(ui, catalog, hero, &weapons, &excluded, totals);
        let debuffs: Vec<&GroupStat> = of(|k| *k == GroupKind::Debuff).collect();
        self.labelled_rows(ui, catalog, hero, tx().debuff_group, &debuffs, totals);
        let others: Vec<&GroupStat> = of(|k| *k == GroupKind::Other).collect();
        self.labelled_rows(ui, catalog, hero, tx().other_group, &others, totals);
    }

    fn labelled_rows(&mut self, ui: &Ui, catalog: &RelicCatalog, hero: usize, label: &str, rows: &[&GroupStat], totals: Totals) {
        if rows.is_empty() {
            return;
        }
        ui.spacing();
        ui.text_disabled(label);
        for g in rows {
            self.group_row(ui, catalog, hero, g, totals, 0.0);
        }
    }

    /// 「仅限特定武器」大组：可整组勾选，展开后是各武器类别
    fn weapon_block(&mut self, ui: &Ui, catalog: &RelicCatalog, hero: usize, rows: &[&GroupStat], excluded: &HashSet<u32>, totals: Totals) {
        if rows.is_empty() {
            return;
        }
        ui.spacing();
        let ids: Vec<u32> = rows.iter().map(|g| g.id).collect();
        let state = state_of(&ids, excluded);
        let [x0, _] = ui.cursor_pos();
        let right = x0 + ui.content_region_avail()[0];
        if ui.arrow_button("##weapons_open", if self.weapons_open { Direction::Down } else { Direction::Right }) {
            self.weapons_open = !self.weapons_open;
            sfx::request(Cue::Cursor);
        }
        ui.same_line();
        if widgets::check(ui, &format!("{}##weapons", tx().weapon_group), state) {
            self.set_groups(hero, &ids, state != Check::On);
        }
        let count: usize = rows.iter().map(|g| g.effects.len()).sum();
        let weight: u64 = rows.iter().map(|g| g.weight).sum();
        stat_text(ui, right, count, percent(weight, totals.all));
        if self.weapons_open {
            let indent = ui.current_font_size() * 1.5;
            for g in rows {
                self.group_row(ui, catalog, hero, g, totals, indent);
            }
        }
    }

    fn group_row(&mut self, ui: &Ui, catalog: &RelicCatalog, hero: usize, g: &GroupStat, totals: Totals, indent: f32) {
        let on = !self.excluded_of(hero).contains(&g.id);
        let name = match g.kind {
            GroupKind::Other => tx().other_group.to_string(),
            _ => catalog.group(g.id).and_then(|gr| relic_names::group(gr.text_id)).map_or_else(|| format!("#{}", g.id), str::to_string),
        };
        let [x0, y] = ui.cursor_pos();
        let right = x0 + ui.content_region_avail()[0];
        ui.set_cursor_pos([x0 + indent, y]);
        if widgets::check(ui, &format!("{name}##group{}", g.id), if on { Check::On } else { Check::Off }) {
            self.set_groups(hero, &[g.id], !on);
        }
        let hovered = ui.is_item_hovered();
        stat_text(ui, right, g.effects.len(), percent(g.weight, totals.all));
        if hovered {
            let mut effects = g.effects.clone();
            effects.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
            ui.tooltip(|| {
                for (id, weight) in effects {
                    let chance = if on { format!("{:>5.1}%", percent(weight, totals.selected)) } else { "   —  ".to_string() };
                    ui.text(format!("{chance}  {}", effect_name(catalog, id).replace('\n', "")));
                }
            });
        }
    }

    /// 把一批分组设为勾选或取消
    fn set_groups(&mut self, hero: usize, ids: &[u32], include: bool) {
        let excluded = self.excluded_of(hero);
        for id in ids {
            if include {
                excluded.remove(id);
            } else {
                excluded.insert(*id);
            }
        }
        self.touch();
    }
}

/// 在本行右端写「N 条 · P%」
fn stat_text(ui: &Ui, right: f32, count: usize, share: f64) {
    let info = format!("{count} {} · {share:.1}%", tx().count_unit);
    ui.same_line_with_pos(right - ui.calc_text_size(&info)[0]);
    ui.text_disabled(info);
}
