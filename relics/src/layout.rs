//! 新增行涉及的参数表与字段偏移：Smithbox NR paramdef 按 regulation 10350000 计算，行大小与原版文件核对过。

pub struct Table {
    /// regulation 里的文件名（不含 .param）
    pub file: &'static str,
    pub struct_name: &'static str,
    pub row_size: usize,
}

pub const ATTACH_EFFECT: Table = Table { file: "AttachEffectParam", struct_name: "ATTACHEFFECT_PARAM_ST", row_size: 68 };
pub const SP_EFFECT: Table = Table { file: "SpEffectParam", struct_name: "SP_EFFECT_PARAM_ST", row_size: 1024 };
/// paramdef 写 332 字节，NR 实际 348
pub const BULLET: Table = Table { file: "Bullet", struct_name: "BULLET_PARAM_ST", row_size: 348 };
pub const BEHAVIOR_PC: Table = Table { file: "BehaviorParam_PC", struct_name: "BEHAVIOR_PARAM_ST", row_size: 32 };
pub const ATK_PC: Table = Table { file: "AtkParam_Pc", struct_name: "ATK_PARAM_ST", row_size: 464 };
pub const SP_EFFECT_VFX: Table = Table { file: "SpEffectVfxParam", struct_name: "SP_EFFECT_VFX_PARAM_ST", row_size: 164 };
pub const ATTACH_EFFECT_FILTER: Table =
    Table { file: "AttachEffectFilterParam", struct_name: "ATTACHEFFECT_FILTER_PARAM_ST", row_size: 20 };
pub const EQUIP_ACCESSORY: Table = Table { file: "EquipParamAccessory", struct_name: "EQUIP_PARAM_ACCESSORY_ST", row_size: 64 };
pub const PERMANENT_BUFF: Table = Table { file: "PermanentBuffParam", struct_name: "PERMANENT_BUFF_PARAM_ST", row_size: 24 };
/// 锻体在掉落里按道具（类别 1）出现：每个锻体还要有同号的道具行（类型 11），否则选项显示 ?GoodsName?
pub const EQUIP_GOODS: Table = Table { file: "EquipParamGoods", struct_name: "EQUIP_PARAM_GOODS_ST", row_size: 224 };
pub const MAP_POINT_ICON: Table = Table { file: "WorldMapPointIconParam", struct_name: "WORLD_MAP_POINT_ICON_PARAM_ST", row_size: 4 };
pub const SMALL_BASE: Table = Table { file: "SmallBaseAndSpotDefine", struct_name: "SMALLBASE_AND_SPOT_DEFINE_ST", row_size: 100 };
/// 武器词条抽奖表：同一表 ID 占连续多行（每行一个词条），由 tiered::passives 按行追加
pub const ATTACH_EFFECT_TABLE: Table = Table { file: "AttachEffectTableParam", struct_name: "ATTACHEFFECT_TABLE_PARAM_ST", row_size: 12 };
/// 同一表 ID 占连续多行（每行一个候选），不走 RowSpec，由 pool.rs 按行增删
pub const ITEM_TABLE: Table = Table { file: "ItemTableParam", struct_name: "ITEMTABLE_PARAM_ST", row_size: 36 };
/// 每个地图模式的夜晚首领，由 play_area.rs 按格替换
pub const PLAY_AREA: Table = Table { file: "LotResultPlayAreaParam", struct_name: "LOT_RESULT_PLAYAREA_PARAM_ST", row_size: 40 };
/// 掉落行（敌人 / 地图共用类型名，paramdef 写 290 字节，实际 224），由 tiered::loot 改写；字段见 shared 的 item_lot
pub const ITEM_LOT_ENEMY: Table = Table { file: "ItemLotParam_enemy", struct_name: "ITEMLOT_PARAM_ST", row_size: 224 };
pub const ITEM_LOT_MAP: Table = Table { file: "ItemLotParam_map", struct_name: "ITEMLOT_PARAM_ST", row_size: 224 };
/// 只读：按敌人引用掉落行的字段区分普通 / 深夜红皮 / 入侵据点
pub const NPC: Table = Table { file: "NpcParam", struct_name: "NPC_PARAM_ST", row_size: 848 };

pub const TABLES: [&Table; 12] = [
    &ATTACH_EFFECT,
    &ATTACH_EFFECT_FILTER,
    &SP_EFFECT,
    &BULLET,
    &BEHAVIOR_PC,
    &ATK_PC,
    &SP_EFFECT_VFX,
    &EQUIP_ACCESSORY,
    &PERMANENT_BUFF,
    &EQUIP_GOODS,
    &MAP_POINT_ICON,
    &SMALL_BASE,
];

pub mod attach_effect {
    /// s32 词条常驻效果（SpEffect）
    pub const PASSIVE_SP_EFFECT: usize = 12;
    /// s32 词条名（item msgbnd 的 AttachEffectName）
    pub const TEXT_ID: usize = 28;
    /// s32 遗物筛选 / 排序用的筛选行
    pub const FILTER_ID: usize = 56;
    /// s32 界面显示的数值（如 +16%）
    pub const DISPLAYED_VALUE: usize = 48;
    /// s32 同一武器上互斥的分组、状态图标、同名词条的基准行 / 替换行（原版各档都填自身或一档）
    pub const COMPATIBILITY_ID: usize = 24;
    pub const STATUS_ICON: usize = 40;
    pub const OVERRIDE_BASE: usize = 44;
    pub const OVERRIDE_EFFECT: usize = 52;
    /// isPersistentEffect 位：持有时常驻（原版绝大多数武器词条都有）
    pub const FLAGS_BYTE: usize = 32;
    pub const PERSISTENT_BIT: u8 = 0;
}

pub mod attach_effect_table {
    /// s32 词条（AttachEffectParam ID）、u16 权重、s16 有 DLC 时的权重（-1 为沿用权重）
    pub const ATTACH_EFFECT_ID: usize = 4;
    pub const WEIGHT: usize = 8;
    pub const WEIGHT_DLC: usize = 10;
}

pub mod attach_effect_filter {
    /// s32 筛选列表里显示的词条名与排序值
    pub const TEXT_ID: usize = 4;
    pub const VALUE: usize = 12;
}

pub mod sp_effect {
    /// f32 持续时间（秒）
    pub const EFFECT_ENDURANCE: usize = 8;
    /// f32 周期触发间隔（秒），与 CYCLE_OCCURRENCE 配合
    pub const MOTION_INTERVAL: usize = 12;
    /// s16 效果类别：引擎只认固定几种（Smithbox 枚举 SP_EFFECT_SPCATEGORY），未定义的值不会顶掉任何效果。
    /// 200 = 顶掉「同类别且同优先级」的旧效果，原版成对开关（自动复活可用 / 不可用等）都用它 + 独占优先级
    pub const SP_CATEGORY: usize = 318;
    /// u8 类别内优先级
    pub const CATEGORY_PRIORITY: usize = 320;
    /// isDisableNetSync 位：周期施加的效果不同步给队友（同原版铁之眼毒箭遗物）
    pub const NET_SYNC_BYTE: usize = 601;
    pub const NET_SYNC_DISABLE_BIT: u8 = 4;
    /// effectTargetSelfTarget 位：来源是自己时也生效。周期触发是「带着效果的角色给自己挂」，被挂的效果没有这一位会被拒
    /// （原版血焰刀刃周期补挂的出血 1632002 有，命定之死降血量上限 6860 只允许对手施加）
    pub const TARGET_BYTE: usize = 364;
    pub const SELF_TARGET_BIT: u8 = 2;
    /// s32 周期触发的 SpEffect（触发类效果用它转成实际效果）
    pub const CYCLE_OCCURRENCE: usize = 296;
    /// s32 施加时发出的行为（BehaviorParam_PC，需状态值 275；原版「圣杯瓶也能治疗队友」遗物 7010280 用它向周围队友发子弹）
    pub const BEHAVIOR_ID: usize = 152;
    /// u16 状态值：动作事件按它判断条件（遗物标记 2001~2130 段）
    pub const STATE_INFO: usize = 342;
    /// s32 效果名（PermanentBuffName / menu SpEffectName）与第二条说明
    pub const TEXT_ID_1: usize = 924;
    pub const TEXT_ID_2: usize = 680;
    /// s32 效果期间的特效（SpEffectVfxParam）
    pub const VFX_ID: usize = 368;
    /// s32 附魔的火 / 圣属性攻击力加值
    pub const FIRE_ATTACK_POWER: usize = 96;
    pub const HOLY_ATTACK_POWER: usize = 484;
    /// s32 附魔命中时给目标施加的 SpEffect
    pub const ATK_OCCURRENCE: usize = 300;
    /// u8 附魔命中特效与音效（火 11、命定之死 13）及其变体（血焰 3）
    pub const SP_ATTRIBUTE: usize = 341;
    pub const SP_ATTRIBUTE_VARIATION: usize = 809;
    /// f32 受到的韧性伤害倍率：原版各种「提升强韧度」都只调它；为 0 也挡不住受击动作，免硬直要靠类别 1001 的受击等级替换
    pub const TOUGHNESS_DAMAGE_CUT_RATE: usize = 436;
    /// s32 状态界面 / 常驻效果的图标
    pub const ICON_ID: usize = 0;
    /// u8 存档类别：原版锻体效果都是 10
    pub const SAVE_CATEGORY: usize = 321;
    /// f32 受到的伤害倍率：斩击 / 打击 / 突刺 / 一般物理 / 魔力 / 火 / 雷 / 圣（同战技「忍耐」改的 8 项）
    pub const DAMAGE_CUT_RATES: [usize; 8] = [28, 32, 36, 40, 44, 48, 52, 464];
    /// f32 物理 / 魔力 / 火 / 雷 / 圣 攻击力倍率
    pub const ATTACK_RATES: [usize; 5] = [56, 60, 64, 68, 476];
    /// s32 抗性加值：中毒 / 猩红腐败 / 出血 / 咒死 / 冻伤 / 睡眠 / 发狂
    pub const RESIST_POINTS: [usize; 7] = [508, 512, 516, 520, 524, 840, 844];
    /// effectTarget 位：自己 / 友方 / 玩家 / AI / 活人 / 灵体（位 2 是敌方，不开）。纯标记效果原版都关着，带属性的原版都开着
    pub const EFFECT_TARGET_BYTE: usize = 351;
    pub const EFFECT_TARGET_BITS: [u8; 6] = [0, 1, 3, 4, 5, 6];
    /// s32 施加时给的猩红腐败 / 发狂累积值
    pub const DISEASE_ATTACK_POWER: usize = 208;
    pub const MADNESS_ATTACK_POWER: usize = 828;
    /// s32 施加时给的 中毒 / 猩红腐败 / 出血 / 冻伤 / 睡眠 / 发狂 累积值（同 AILMENT_DAMAGE_RATES 的顺序）
    pub const AILMENT_ATTACK_POWERS: [usize; 6] = [204, DISEASE_ATTACK_POWER, 212, 424, 824, MADNESS_ATTACK_POWER];
    /// u8 受到的 中毒 / 猩红腐败 / 出血 / 冻伤 / 睡眠 / 发狂 伤害倍率（%，学者遗物「绝招连结的目标异常伤害增加」为 130）
    pub const AILMENT_DAMAGE_RATES: [usize; 6] = [988, 989, 327, 350, 848, 600];
    /// f32 自己施加的 中毒 / 猩红腐败 / 冻伤 / 出血 / 咒死 / 睡眠 / 发狂 累积倍率（原版互动效果「强化腐败」1.3、「强化睡眠」1.5 / 2.5）
    pub const INFLICT_RATES: [usize; 7] = [144, 192, 196, 488, 492, 228, 232];
    /// dontDeleteOnDead 位：死亡时不移除
    pub const KEEP_ON_DEATH_BYTE: usize = 601;
    pub const KEEP_ON_DEATH_BIT: u8 = 2;
    /// f32 对弱点类型敌人的伤害倍率：B 死诞者，C 古龙、卡莉果，D 飞龙等龙
    pub const WEAK_DMG_RATE_B: usize = 444;
    pub const WEAK_DMG_RATE_C: usize = 448;
    pub const WEAK_DMG_RATE_D: usize = 452;
    /// f32 每次恢复最大血量 / 专注值的百分比（负数为恢复）
    pub const CHANGE_HP_RATE: usize = 156;
    pub const CHANGE_MP_RATE: usize = 164;
    /// s32 效果结束时换成的效果
    pub const REPLACE_SP_EFFECT: usize = 292;
    /// u8 受到伤害时提前结束（学者观察的易伤 708020 为 1，被打一下就换成 REPLACE_SP_EFFECT）
    pub const DELETE_CRITERIA_DAMAGE: usize = 807;
    /// f32 绝招量表自然累积速度倍率（原版遗物同时改它与 ULTIMATE_ART_GAUGE_CHARGE_RATE）
    pub const ULTIMATE_ART_GAUGE_RATE: usize = 908;
    pub const ULTIMATE_ART_GAUGE_CHARGE_RATE: usize = 976;
    /// f32 施加时直接增加的绝招量表（百分点，大空洞恩惠 6999611 为 100）
    pub const ULTIMATE_ART_GAUGE: usize = 916;
    /// f32 技艺冷却时间倍率（原版遗物 0.95 ~ 0.825）
    pub const CHARACTER_SKILL_COOLDOWN: usize = 912;
    /// s32 击杀时给自己施加的效果（需状态值 199）：实测自己击杀普通敌人只触发 GET_SOUL（得到卢恩时），KILL_SP 不触发；
    /// 原版击杀回量表遗物两个都填（+5 与 +1.65）
    pub const APPLY_ON_GET_SOUL: usize = 780;
    pub const APPLY_ON_KILL_SP: usize = 260;
    /// f32 防御时受击消耗的精力倍率（需状态值 158）
    pub const GUARD_STAMINA_MULT: usize = 904;
    /// s32 施加时给累积器（同状态值的 accumuOverVal 效果）加的值（执行者妖刀：防御 5、拨开 15 / 20，满 50 解放）
    pub const ACCUMU_VAL: usize = 388;
    /// f32 血量上限倍率
    pub const MAX_HP_RATE: usize = 16;
    /// f32 观察力加值（需状态值 66，原版遗物「提升观察力」为 0.4）
    pub const ITEM_DROP_RATE: usize = 504;
    /// f32 受到黑夜雨的伤害倍率
    pub const RAIN_DAMAGE_CORRECTION: usize = 992;
    /// s32 每次恢复的血量 / 精力点数（负数为恢复）
    pub const CHANGE_HP_POINT: usize = 160;
    pub const CHANGE_STAMINA_POINT: usize = 180;
    /// u16 持有该状态值时才生效（原版「周围出血时提升攻击力」用它做条件），三个之间是「或」
    pub const INVOCATION_STATE_1: usize = 696;
    pub const INVOCATION_STATE_2: usize = 698;
    pub const INVOCATION_STATE_3: usize = 700;
    /// s16 / s8 装备某类武器（武器表 wepType：小盾 65、中盾 67、大盾 69、法杖 57、圣印记 61）达到数量时才生效（需状态值 2100）
    pub const WEP_TYPE_TRIGGER: usize = 610;
    pub const WEP_TYPE_TRIGGER_COUNT: usize = 612;
    /// s8 生命力加点（原版「装备 3 把以上盾」遗物 +10）
    pub const ADD_VIGOR: usize = 798;
    /// f32 专注值 / 精力上限倍率
    pub const MAX_MP_RATE: usize = 20;
    pub const MAX_STAMINA_RATE: usize = 24;
    /// s32 复仇者召唤的家人（海伦 / 弗雷德里克 / 塞巴斯蒂安）身上施加的效果
    pub const REVENANT_FAMILY: [usize; 3] = [952, 956, 960];
    /// u8 攻击力倍率只对这两个攻击分类生效（AtkParam subCategory：112 战技、111 蓄力战技、130 近战攻击等；0 为不限）
    pub const MAGIC_SUB_CATEGORY: [usize; 2] = [314, 315];
    /// f32 被敌人选为目标的优先度（复仇者塞巴斯蒂安的嘲讽 +600，武器词条「敌人比较不会攻击自己」-300 ~ -600）
    pub const TARGET_PRIORITY: usize = 248;
}

pub use nr_stat_alloc_shared::param::layout::{item_table, permanent_buff};

pub mod equip_goods {
    /// u16 选项与道具栏图标（锻体用 20xxx 段）；u8 稀有度：原版一级锻体 1、二级与特殊 2、传说道具 3
    pub const ICON_ID: usize = 48;
    pub const RARITY: usize = 120;
    /// s32 锻体道具对应的锻体行（paramdef 叫 vagrantItemLotId）：原版锻体道具行都填自身编号，游戏按它取锻体的效果与文本
    pub const BUFF_ID: usize = 84;
}

pub mod map_point_icon {
    /// s32 菜单图标号：界面按它取 MENU_MenuIcon_<号>，图不在已加载的基础图集里时整个据点图标不显示
    pub const ICON_ID: usize = 0;
}

pub mod small_base {
    /// s16 地图上显示的图标行（WorldMapPointIconParam ID，也是地图界面图标影片的帧号）
    pub const MAP_ICON: usize = 44;
}

pub mod play_area {
    /// s16 第一夜首领、第二夜首领、额外首领 1 / 2（值是首领场地地图号，如 4984 = m49_84；-1 为空）
    pub const BOSS_COLUMNS: [usize; 4] = [16, 18, 20, 22];
}

pub mod npc {
    /// s32 普通掉落、深夜模式（红皮）掉落、入侵据点掉落的首行（选项按 +0 / +1 / +2 连续排列）
    pub const REWARD_LOT: usize = 728;
    pub const CHAOS_REWARD_LOT: usize = 824;
    pub const CONDEMNED_REWARD_LOT: usize = 108;
}

pub mod equip_accessory {
    /// s32 护符效果：paramdef 叫 spEffectId_1，实际填的是词条（AttachEffectParam）ID，由词条再挂常驻 SpEffect
    pub const ATTACH_EFFECT_1: usize = 32;
}

pub mod sp_effect_vfx {
    /// s32 效果持续期间播放的特效与 s16 挂点
    pub const MIDST_SFX: usize = 0;
    pub const MIDST_DMY: usize = 36;
    /// s32 持续音效；u8 附魔刀光拖尾的类型（原版黑焰 17、血焰 18）
    pub const MIDST_SE: usize = 4;
    /// s32 附魔特效的 8 个生成挂点（与终点挂点交替排列，间隔 8 字节）：右手 10300 / 11300 / 10310 / 11310 … 10330 / 11330，左手 2xxxx
    pub const ENCHANT_START_DMY: usize = 60;
    pub const ENCHANT_DMY_STRIDE: usize = 8;
    pub const ENCHANT_SOUL: usize = 43;
    pub const ENCHANT_TRACE: usize = 58;
}

pub mod bullet {
    pub const ATK_ID: usize = 0;
    pub const SFX_ID: usize = 4;
    /// f32 判定半径
    pub const HIT_RADIUS: usize = 68;
    /// 只给发射者加效果的子弹用原版同类子弹的半径（2190600 等：No Target、0.1），不罩住周围敌人
    pub const SELF_EFFECT_RADIUS: f32 = 0.1;
    /// s32 子弹发射时施加给发射者的 SpEffect
    pub const SP_EFFECT_FOR_SHOOTER: usize = 96;
    /// f32 判定半径随时间扩大到的最大值
    pub const HIT_RADIUS_MAX: usize = 72;
    pub const HIT_BULLET_ID: usize = 104;
    /// s32 命中目标时施加的 SpEffect
    pub const SP_EFFECT_0: usize = 108;
    pub const SP_EFFECT_1: usize = 112;
    /// f32 命中记录保留时间：过了这段时间同一目标能再被打中（持续伤害的间隔，0 为整个寿命只打一次）
    pub const DMG_HIT_RECORD_LIFE_TIME: usize = 88;
    /// s32 存活期间定时发射的子弹与 f32 间隔（最小 / 最大）
    pub const INTERVAL_CREATE_BULLET: usize = 172;
    pub const INTERVAL_CREATE_TIME_MIN: usize = 176;
    pub const INTERVAL_CREATE_TIME_MAX: usize = 180;
}

pub mod behavior {
    pub const JUDGE_ID: usize = 4;
    pub const REF_ID: usize = 12;
}

pub mod atk {
    /// f32 第 0 / 1 个判定球半径
    pub const HIT0_RADIUS: usize = 0;
    pub const HIT1_RADIUS: usize = 4;
    /// s32 命中时施加的第 1~3 个 SpEffect（状态异常积累等）
    pub const SP_EFFECT_0: usize = 24;
    pub const SP_EFFECT_1: usize = 28;
    pub const SP_EFFECT_2: usize = 32;
    /// s16 第 0 个判定球的两端挂点（两端都填是胶囊，沿刀身）
    pub const HIT0_DMY_POLY1: usize = 44;
    pub const HIT0_DMY_POLY2: usize = 52;
    /// u8 第 1 个判定球的形状
    pub const HIT1_HIT_TYPE: usize = 107;
    /// u16 固定物理 / 火攻击力（道具类攻击，不乘武器攻击力）
    pub const ATK_PHYS: usize = 80;
    pub const ATK_FIRE: usize = 84;
    /// f32 削韧值、击退距离
    pub const ATK_SUPER_ARMOR: usize = 132;
    pub const KNOCKBACK_DIST: usize = 16;
    /// s16 第 1 个判定球的两端挂点
    pub const HIT1_DMY_POLY1: usize = 46;
    pub const HIT1_DMY_POLY2: usize = 54;
    /// u8 攻击属性、攻击类型（打击感 / 受击反应）、特效方向、攻击行为
    pub const ATK_ATTRIBUTE: usize = 117;
    pub const ATK_TYPE: usize = 119;
    pub const ATK_DIR_FOR_SFX: usize = 128;
    pub const ATK_BEHAVIOR_ID: usize = 130;
    /// u16 物理 / 魔力 / 火 / 雷 / 圣 攻击力修正（%，乘攻击者的基础攻击力；追踪者绝招只有物理和火）
    pub const PHYS_CORRECTION: usize = 62;
    pub const MAG_CORRECTION: usize = 64;
    pub const FIRE_CORRECTION: usize = 66;
    pub const THUN_CORRECTION: usize = 68;
    pub const DARK_CORRECTION: usize = 390;
    /// 只给自己加效果的子弹用的空攻击（原版「No Target」，不命中任何人，不留命中记录）
    pub const NO_TARGET: i32 = 1;
    /// u8 特殊属性（命中特效与音效）：火 11、魔力（灵火）10、命定之死 13；及其变体（血焰 3）
    pub const SP_ATTRIBUTE: usize = 118;
    pub const SP_ATTRIBUTE_VARIATION: usize = 429;
}
