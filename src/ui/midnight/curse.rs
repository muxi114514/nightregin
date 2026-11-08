//! 诅咒的显示：选角界面的揭晓横幅、出征中的角标，以及 16 条诅咒的中英文文字（本文件列在 GLYPH_SOURCES 里）。

use hudhook::imgui::{Condition, Ui};
use nr_stat_alloc_shared::midnight::curse::CurseId;

use super::text::tx;
use crate::lang::{self, Lang};
use crate::ui::theme;

/// 按编号 1~16 排列：(中文名, 中文提示, 英文名, 英文提示)
const TEXTS: [(&str, &str, &str, &str); 16] = [
    ("受伤时累积出血", "别硬吃连段，血条留余量", "Bleed on Hit", "Avoid full combos and keep an HP buffer"),
    ("受伤时累积猩红腐败", "被打后尽快拉开距离", "Scarlet Rot on Hit", "Back away quickly after being hit"),
    ("持续减少血量", "别在路上拖时间", "Constant HP Loss", "Don't dawdle on the road"),
    ("圣杯瓶恢复量降低", "靠走位，少指望喝药", "Weaker Flasks", "Rely on footwork, not healing"),
    ("绝招积累变慢", "绝招留给夜王和救人", "Slower Ultimate Gauge", "Save ultimates for nightlords and rescues"),
    ("获得的卢恩减少", "优先打高价值目标", "Fewer Runes", "Prioritize high-value targets"),
    ("闪避瞬间减伤降低", "别贴脸乱滚", "Exposed After Dodging", "Don't roll carelessly up close"),
    ("血量不满时攻击降低", "被打后先补血再输出", "Weak When Hurt", "Heal before dealing damage"),
    ("攻击消耗更多精力", "留一口精力翻滚", "Costly Attacks", "Keep enough stamina to roll"),
    ("濒死时血量上限降低", "别让血线掉到底", "Frail at Death's Door", "Don't let your HP hit bottom"),
    ("全部敌人血量 +20%", "少清小怪，挑值得打的", "Enemy HP +20%", "Skip trash and pick your fights"),
    ("全部敌人攻击 +15%", "稳着打，别贪刀", "Enemy Attack +15%", "Play safe and don't get greedy"),
    ("夜王血量 +25%", "绝招和药留到最后", "Nightlord HP +25%", "Save ultimates and flasks for the end"),
    ("地图不可见", "提前记路线，跟紧队友", "Map Hidden", "Memorize routes and stay with the team"),
    ("不知道夜王是谁", "配置别只堆一种属性", "Unknown Nightlord", "Don't stack a single damage type"),
    ("必定两处灾变地点", "路线绕开或提前准备", "Two Cataclysms", "Route around them or come prepared"),
];

pub struct CurseText {
    pub name: &'static str,
    pub hint: &'static str,
}

pub fn curse_text(id: CurseId) -> Option<CurseText> {
    let (zh_name, zh_hint, en_name, en_hint) = TEXTS.get(usize::from(id).checked_sub(1)?)?;
    Some(match lang::current() {
        Lang::Zh => CurseText { name: zh_name, hint: zh_hint },
        Lang::En => CurseText { name: en_name, hint: en_hint },
    })
}

/// 选角界面的揭晓横幅：画面上方居中
pub fn draw_banner(ui: &Ui, id: CurseId) {
    let Some(text) = curse_text(id) else { return };
    let [w, h] = ui.io().display_size;
    ui.window("##nr_midnight_curse_banner")
        .position([w * 0.5, h * 0.14], Condition::Always)
        .position_pivot([0.5, 0.5])
        .no_decoration()
        .always_auto_resize(true)
        .focus_on_appearing(false)
        .no_inputs()
        .build(|| {
            ui.text_colored(theme::ACCENT, tx().curse_title);
            ui.text_colored(theme::WARN, text.name);
            ui.text(text.hint);
        });
}

/// 出征中的角标：放在午夜队伍角标上方，避开左上角的血条和状态栏
pub fn draw_badge(ui: &Ui, id: CurseId) {
    let Some(text) = curse_text(id) else { return };
    let [w, h] = ui.io().display_size;
    ui.window("##nr_midnight_curse_badge")
        .position([w * 0.985, h * 0.76], Condition::Always)
        .position_pivot([1.0, 1.0])
        .no_decoration()
        .always_auto_resize(true)
        .movable(false)
        .focus_on_appearing(false)
        .no_inputs()
        .build(|| ui.text_colored(theme::WARN, format!("{}{}", tx().curse_prefix, text.name)));
}
