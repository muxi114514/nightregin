//! 配置：DLL 同目录 nr_stat_alloc.toml；缺失或解析失败时使用默认值并记录日志。

use std::path::Path;

use nr_stat_alloc_shared::relic_reroll::{Cost, Prices};
use serde::Deserialize;

pub const FILE_NAME: &str = "nr_stat_alloc.toml";
const DEFAULT_LEASE_REFRESH_MS: u64 = 2000;
/// regulation 中组件时长 6 秒，续期间隔上限留出一半余量给网络延迟与卡顿
const MAX_LEASE_REFRESH_MS: u64 = 3000;
/// 第三选项里局内词条原本占 95%，至少给它留一些
const MAX_STONE_PERCENT: u16 = 90;
const MAX_PRICE_EXPONENT: f32 = 2.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// 按 schemes.toml 逐级分配
    Scheme,
    /// 使用 [test] 的固定差值（调试用）
    Test,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    /// error / warn / info / debug / trace
    pub log_level: String,
    pub mode: Mode,
    /// 施加组件时是否同步给联机队友（对应 apply_speffect 的 dont_sync 取反）
    pub sync_network: bool,
    /// 组件续期间隔（毫秒）。组件在 regulation 中时长为 6 秒，续期间隔必须明显小于它
    pub lease_refresh_ms: u64,
    /// 对账间隔（毫秒）
    pub reconcile_interval_ms: u64,
    pub rules: Rules,
    pub hotkeys: Hotkeys,
    pub ui: UiConfig,
    pub weapon_passive: WeaponPassiveConfig,
    pub starting_weapons: StartingWeaponsConfig,
    pub boss_loot: BossLootConfig,
    pub legendary_upgrade: LegendaryUpgradeConfig,
    pub relic_reroll: RelicRerollConfig,
    pub passive_inherit: PassiveInheritConfig,
    pub midnight: MidnightConfig,
    pub tiered_buffs: TieredBuffsConfig,
    pub diag: DiagConfig,
    pub test: TestDelta,
    pub offsets: OffsetOverrides,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Rules {
    pub min: u32,
    pub cap: u32,
}

/// 虚拟键码（VK_*），默认 F6 面板 / F7 重载方案 / F8 校准 / F9 转储 / F10 开关
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Hotkeys {
    pub panel: u32,
    pub reload: u32,
    pub calibrate: u32,
    pub dump: u32,
    pub toggle: u32,
}

/// 叠加层界面
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct UiConfig {
    pub enabled: bool,
    /// 在"按分辨率自适应"的基础上再乘的缩放
    pub scale: f32,
    /// 中文字体文件路径，留空自动使用系统的华文中宋 / 宋体 / 微软雅黑
    pub font: String,
    /// 界面语言："auto" 跟随 Steam 中本游戏的语言，"zh" 中文，"en" 英文
    pub language: String,
    /// 手柄开关面板的按键组合（遗物装配界面打开时有效），如 "L3"、"Back"、"LB+RB"
    pub pad_toggle: String,
    /// 操作面板时播放游戏菜单的光标 / 确定 / 返回音效
    pub sound: bool,
}

/// 锻造石强化武器时同步升级词条
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct WeaponPassiveConfig {
    pub enabled: bool,
    /// 每次强化把武器实例的内容写进日志（排查问题用）
    pub dump: bool,
}

/// 初始武器（互换主副手在方案里按角色 + 器皿设置，见 schemes.toml 的 weapons）
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct StartingWeaponsConfig {
    /// 出征时给初始武器（主手、副手各）随机 1 条效力 1 的通用词条
    pub random_passive: bool,
}

/// 首领掉落球第三个选项（局内词条 / 徘徊赐福）
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct BossLootConfig {
    /// 锻造石【３】出现的概率（%，从局内词条里扣，0 为不加）
    pub smithing_stone_percent: u16,
}

/// 锻造台可用锻造石【３】把紫色武器强化成金色
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct LegendaryUpgradeConfig {
    pub enabled: bool,
}

/// 三档锻体里靠 DLL 实现的词条（只对带三档锻体的 regulation 起作用，其余整合包身上不会有标记）
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct TieredBuffsConfig {
    pub enabled: bool,
    /// 卢恩随机倍数的范围（对数均匀）与封顶
    pub rune_gamble_min: f64,
    pub rune_gamble_max: f64,
    pub rune_gamble_cap: u32,
    /// 消耗卢恩时返还的概率
    pub rune_refund_chance: f64,
    /// 致命伤害免死消耗的卢恩
    pub rune_shield_cost: u32,
    /// 「装备界面快速打开锻造台」的热键（虚拟键码，0 为关闭）：强化界面、词条继承面板
    pub quick_forge_key: u32,
    pub quick_inherit_key: u32,
    /// 三选一界面「刷新选项」每次的卢恩价格，依次为第 1、2、3… 次，长度就是每个球的次数；空数组关闭
    pub orb_refresh_costs: Vec<u32>,
}

/// 遗物重铸（遗物装配界面的「重铸遗物」按钮）
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct RelicRerollConfig {
    pub enabled: bool,
    /// 倍率 = (全部候选权重 / 勾选后候选权重)^指数；1 为筛选只省事不省钱，0 为筛选不加价
    pub price_exponent: f32,
    /// 本模组新增的追踪者词条随原版模板词条出现（与模板平分权重）
    pub include_custom: bool,
    /// 基础价，0 为读游戏商店里情景原石的价格
    pub murk_standard: u32,
    pub murk_deep: u32,
    pub sigils_standard: u32,
    pub sigils_deep: u32,
}

/// 局内武器词条继承（锻造台的「继承词条」选项）
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct PassiveInheritConfig {
    pub enabled: bool,
}

/// 午夜模式（深夜栏上的开关）：积分规则与等级曲线
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct MidnightConfig {
    pub enabled: bool,
    /// 切换午夜的热键（虚拟键码，0 = 关闭）
    pub toggle_key: u32,
    pub win_points: u32,
    pub everdark_multiplier: f32,
    pub loss_points: u32,
    /// 前几级每级固定积分
    pub early_levels: u32,
    pub early_cost: u32,
    /// 之后每级比上一级多的积分
    pub cost_step: u32,
    /// 联机时重写自己午夜状态的间隔（秒；状态变化时立即写）
    pub resend_seconds: u64,
}

/// 诊断开关（排查问题用，平时保持关闭）
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct DiagConfig {
    /// 把游戏播放的音效事件写进日志（找菜单音效用）
    pub sound_trace: bool,
    /// 词条继承面板测试热键（虚拟键码，0 为关闭）：出征中按下时不用去锻造台直接打开面板
    pub inherit_panel_key: u32,
    /// 强制午夜诅咒的编号（实机逐条验证用，0 为随机）
    pub midnight_curse: u8,
}

/// test 模式下的固定差值（范围 ±127）
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct TestDelta {
    pub vigor: i32,
    pub mind: i32,
    pub endurance: i32,
    pub strength: i32,
    pub dexterity: i32,
    pub intelligence: i32,
    pub faith: i32,
    pub arcane: i32,
}

/// 运行时自动发现失败时的手动偏移（toml 支持 0x 十六进制）
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct OffsetOverrides {
    pub player_game_data: Option<usize>,
    pub stats: Option<usize>,
    pub level: Option<usize>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            log_level: "info".into(),
            mode: Mode::Scheme,
            sync_network: true,
            lease_refresh_ms: DEFAULT_LEASE_REFRESH_MS,
            reconcile_interval_ms: 500,
            rules: Rules::default(),
            hotkeys: Hotkeys::default(),
            ui: UiConfig::default(),
            weapon_passive: WeaponPassiveConfig::default(),
            starting_weapons: StartingWeaponsConfig::default(),
            boss_loot: BossLootConfig::default(),
            legendary_upgrade: LegendaryUpgradeConfig::default(),
            relic_reroll: RelicRerollConfig::default(),
            passive_inherit: PassiveInheritConfig::default(),
            midnight: MidnightConfig::default(),
            tiered_buffs: TieredBuffsConfig::default(),
            diag: DiagConfig::default(),
            test: TestDelta { strength: 10, vigor: 10, ..Default::default() },
            offsets: OffsetOverrides::default(),
        }
    }
}

impl Default for Rules {
    fn default() -> Self {
        Self { min: 1, cap: 70 }
    }
}

impl Default for Hotkeys {
    fn default() -> Self {
        Self { panel: 0x75, reload: 0x76, calibrate: 0x77, dump: 0x78, toggle: 0x79 }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self { enabled: true, scale: 1.0, font: String::new(), language: "auto".into(), pad_toggle: "L3".into(), sound: true }
    }
}

impl Default for WeaponPassiveConfig {
    fn default() -> Self {
        Self { enabled: true, dump: false }
    }
}

impl Default for StartingWeaponsConfig {
    fn default() -> Self {
        Self { random_passive: true }
    }
}

impl Default for BossLootConfig {
    fn default() -> Self {
        Self { smithing_stone_percent: 10 }
    }
}

impl Default for LegendaryUpgradeConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

impl Default for TieredBuffsConfig {
    fn default() -> Self {
        Self { enabled: true, rune_gamble_min: 0.1, rune_gamble_max: 10.0, rune_gamble_cap: 1_000_000, rune_refund_chance: 0.4, rune_shield_cost: 20_000, quick_forge_key: 0x72, quick_inherit_key: 0x73, orb_refresh_costs: vec![0, 10_000, 50_000] }
    }
}

impl Default for RelicRerollConfig {
    fn default() -> Self {
        Self { enabled: true, price_exponent: 0.5, include_custom: true, murk_standard: 0, murk_deep: 0, sigils_standard: 0, sigils_deep: 0 }
    }
}

impl Default for PassiveInheritConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

impl Default for MidnightConfig {
    fn default() -> Self {
        Self { enabled: true, toggle_key: 0x7A, win_points: 100, everdark_multiplier: 1.5, loss_points: 50, early_levels: 5, early_cost: 120, cost_step: 10, resend_seconds: 5 }
    }
}

impl MidnightConfig {
    pub fn curve(&self) -> nr_stat_alloc_shared::midnight::Curve {
        nr_stat_alloc_shared::midnight::Curve { early_levels: self.early_levels, early_cost: self.early_cost, cost_step: self.cost_step }
    }

    pub fn scoring(&self) -> nr_stat_alloc_shared::midnight::Scoring {
        nr_stat_alloc_shared::midnight::Scoring { win: self.win_points, everdark_multiplier: self.everdark_multiplier, loss: self.loss_points }
    }
}

impl RelicRerollConfig {
    /// 商店价格套上配置里的覆盖值
    pub fn prices(&self, shop: Prices) -> Prices {
        let pick = |custom: u32, shop: u32| if custom > 0 { custom } else { shop };
        Prices {
            standard: Cost { murk: pick(self.murk_standard, shop.standard.murk), sigils: pick(self.sigils_standard, shop.standard.sigils) },
            deep: Cost { murk: pick(self.murk_deep, shop.deep.murk), sigils: pick(self.sigils_deep, shop.deep.sigils) },
        }
    }
}

impl TestDelta {
    /// 顺序：生命 精神 耐力 力量 敏捷 智力 信仰 感应
    pub fn as_array(&self) -> [i32; 8] {
        [self.vigor, self.mind, self.endurance, self.strength, self.dexterity, self.intelligence, self.faith, self.arcane]
    }
}

impl Config {
    pub fn log_filter(&self) -> log::LevelFilter {
        self.log_level.parse().unwrap_or(log::LevelFilter::Info)
    }

    /// 修正越界值，返回修正说明
    fn sanitize(&mut self) -> Vec<String> {
        let mut notes = Vec::new();
        if self.lease_refresh_ms == 0 || self.lease_refresh_ms > MAX_LEASE_REFRESH_MS {
            notes.push(format!("lease_refresh_ms {} out of range, using {DEFAULT_LEASE_REFRESH_MS}", self.lease_refresh_ms));
            self.lease_refresh_ms = DEFAULT_LEASE_REFRESH_MS;
        }
        if self.reconcile_interval_ms == 0 {
            self.reconcile_interval_ms = 500;
        }
        if self.rules.min == 0 || self.rules.cap < self.rules.min || self.rules.cap > 127 {
            notes.push(format!("rules min {} / cap {} invalid, using 1 / 70", self.rules.min, self.rules.cap));
            self.rules = Rules::default();
        }
        if !["auto", "zh", "en"].contains(&self.ui.language.to_ascii_lowercase().as_str()) {
            notes.push(format!("ui language \"{}\" unknown, using auto", self.ui.language));
            self.ui.language = "auto".into();
        }
        if !(0.5..=3.0).contains(&self.ui.scale) {
            notes.push(format!("ui scale {} out of range 0.5~3.0, using 1.0", self.ui.scale));
            self.ui.scale = 1.0;
        }
        if self.boss_loot.smithing_stone_percent > MAX_STONE_PERCENT {
            notes.push(format!("boss_loot smithing_stone_percent {} above {MAX_STONE_PERCENT}, using {MAX_STONE_PERCENT}", self.boss_loot.smithing_stone_percent));
            self.boss_loot.smithing_stone_percent = MAX_STONE_PERCENT;
        }
        if !(0.0..=MAX_PRICE_EXPONENT).contains(&self.relic_reroll.price_exponent) {
            notes.push(format!("relic_reroll price_exponent {} out of range 0~{MAX_PRICE_EXPONENT}, using 0.5", self.relic_reroll.price_exponent));
            self.relic_reroll.price_exponent = 0.5;
        }
        notes
    }
}

/// 返回配置与待写入日志的说明（日志初始化之前调用，所以不能直接打日志）
pub fn load(dir: &Path) -> (Config, String) {
    let path = dir.join(FILE_NAME);
    match std::fs::read_to_string(&path) {
        Ok(text) => match toml::from_str::<Config>(&text) {
            Ok(mut cfg) => {
                let notes = cfg.sanitize();
                let suffix = if notes.is_empty() { String::new() } else { format!(" ({})", notes.join("; ")) };
                (cfg, format!("config loaded: {}{suffix}", path.display()))
            }
            Err(e) => (Config::default(), format!("config parse error, using defaults: {e}")),
        },
        Err(_) => (Config::default(), format!("config not found, using defaults: {}", path.display())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shipped_template_parses_without_corrections() {
        let mut cfg: Config = toml::from_str(include_str!("../config/nr_stat_alloc.toml")).unwrap();
        assert!(cfg.sanitize().is_empty());
        assert_eq!(cfg.hotkeys.panel, 0x75);
        assert!(cfg.ui.enabled);
        assert_eq!(cfg.midnight.toggle_key, 0x7A);
    }

    #[test]
    fn diag_midnight_curse_defaults_to_zero() {
        assert_eq!(DiagConfig::default().midnight_curse, 0);
        let diag: DiagConfig = toml::from_str("midnight_curse = 14\n").unwrap();
        assert_eq!(diag.midnight_curse, 14);
    }

    #[test]
    fn midnight_defaults_match_spec() {
        let m = MidnightConfig::default();
        assert!(m.enabled);
        assert_eq!((m.toggle_key, m.win_points, m.loss_points, m.resend_seconds), (0x7A, 100, 50, 5));
        assert_eq!(m.everdark_multiplier, 1.5);
        assert_eq!(m.curve(), nr_stat_alloc_shared::midnight::Curve::default());
        assert_eq!(m.scoring(), nr_stat_alloc_shared::midnight::Scoring::default());
    }

    /// 本机整合包里实际部署的配置（文件不存在时跳过）：解析失败会让整份配置退回默认值
    #[test]
    fn deployed_configs_parse() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        for rel in ["me3/MMV/me3/StatAlloc", "me3/NightreignPLUS/StatAlloc", "me3/kongcheng/StatAlloc"] {
            let Ok(text) = std::fs::read_to_string(root.join(rel).join(FILE_NAME)) else { continue };
            let mut cfg: Config = toml::from_str(&text).unwrap_or_else(|e| panic!("{rel}: {e}"));
            assert!(cfg.sanitize().is_empty(), "{rel}");
        }
    }
}
