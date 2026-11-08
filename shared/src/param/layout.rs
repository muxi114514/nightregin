//! 本模组用到的参数表与字段偏移。来自 Paramdex 的黑夜君临 paramdef（按 regulation 版本过滤字段、位域打包后计算），
//! 并用原版 regulation（10350000）逐项核对过行大小与实际取值。游戏更新改了表结构时，DLL 与补丁工具会因行大小不符而拒绝工作。

/// 一张参数表：类型名（PARAM 头里的结构体名，在 regulation 中唯一）与行大小
#[derive(Debug, Clone, Copy)]
pub struct Table {
    pub struct_name: &'static str,
    pub row_size: usize,
}

pub mod sp_effect {
    use super::Table;

    pub const TABLE: Table = Table { struct_name: "SP_EFFECT_PARAM_ST", row_size: 1024 };
    /// f32 血量低于该百分比时才生效（-1 为不限；原版「血量偏低时」效果都是 40）
    pub const CONDITION_HP: usize = 4;
    /// f32 持续时间（秒），-1 为永久
    pub const EFFECT_ENDURANCE: usize = 8;
    /// f32 最大血量倍率（深夜敌人强度效果用）
    pub const MAX_HP_RATE: usize = 16;
    /// f32 物理 / 魔法 / 火 / 雷 / 暗 攻击倍率
    pub const ATTACK_RATES: [usize; 5] = [72, 76, 80, 84, 480];
    /// f32 血量 / 专注值 / 精力变化率（%，-100 为回满）
    pub const CHANGE_HP_RATE: usize = 156;
    pub const CHANGE_MP_RATE: usize = 164;
    pub const CHANGE_STAMINA_RATE: usize = 176;
    /// s16 效果类别（20：同 ID 重复施加只重置时长）
    pub const SP_CATEGORY: usize = 318;
    /// s8 存档类别（-1：不写入存档）
    pub const SAVE_CATEGORY: usize = 321;
    /// s32 整表替换成长（HeroStatusParam 起始 ID）
    pub const HERO_STATUS_ID: usize = 620;
    /// s8 生命 集中 耐力 力气 灵巧 智力 信仰 感应 的加减值
    pub const ADD_STATUS: [usize; 8] = [798, 799, 800, 802, 803, 804, 805, 806];
    /// s32 成长差值表（属性改变类遗物）
    pub const HERO_STATUS_MODIFIER: usize = 1000;
    /// s32 出击时把可用武器的战技 / 魔法改成该 ID（改战技、改魔法类遗物），-1 为不改
    pub const START_SWORD_ARTS_ID: usize = 624;
    pub const START_MAGIC_ID: usize = 1020;
    /// u16 状态值：动作事件按它判断条件（遗物标记 2001~2130 段，2101 为出击时的武器附加）
    pub const STATE_INFO: usize = 342;
    /// f32 每个周期直接增加的绝招量表（守护者持续释放绝招的 701515 为 -1）
    pub const ULTIMATE_ART_GAUGE: usize = 916;
}

pub mod player_common {
    use super::Table;

    /// 只有一行（ID 0）
    pub const TABLE: Table = Table { struct_name: "PLAYER_COMMON_PARAM_ST", row_size: 752 };
    pub const ROW: u32 = 0;
    /// u8 护符栏数量（原版 2）、道具栏数量（原版 4；紧跟着的 +15 是道具栏上限 8）
    pub const ACC_SLOT_NUM: usize = 13;
    pub const ITEM_SLOTS: usize = 14;
    /// 学者道具升 2 级 / 3 级要的经验：s16 × 2 × 9 类（投掷物、罐子、丸药、石、泪滴、香、油脂、食物、工具），共 36 字节
    pub const BAGCRAFT_XP: usize = 564;
    pub const BAGCRAFT_XP_LEN: usize = 36;
    /// f32 复仇者死灵术的发动概率（%，paramdef 未命名的 unknown_18，紧跟三个家人召唤字段，原版 35）
    pub const NECROMANCY_CHANCE: usize = 256;
    /// s32 追踪者第六感「可用」/「已用」的效果号（paramdef unknown_27 / unknown_28，原版 700600 / 700601）
    pub const SIXTH_SENSE_EFFECT: usize = 292;
    pub const SIXTH_SENSE_USED_EFFECT: usize = 296;
    /// s32 冷冽海市蜃楼「可用」/「已用」的效果号（paramdef fh_unknown_67 / 69，原版 8970040 / 8970042，引擎每帧现读）
    pub const COLD_MIRAGE_READY_EFFECT: usize = 696;
    pub const COLD_MIRAGE_USED_EFFECT: usize = 704;
    /// s32 学者观察速度（paramdef analyseBaseSpeed_unknown，原版 100）
    pub const ANALYSE_SPEED: usize = 396;
    /// s32 观察到第三级时给敌人挂的易伤（原版 708020）
    pub const ANALYSE_TARGET_LEVEL3_EFFECT: usize = 420;
    /// f32 送葬者恍惚 / 强化恍惚的持续时间（秒，原版 8 / 15）
    pub const TRANCE_DURATION: usize = 484;
    pub const TRANCE_ENHANCE_DURATION: usize = 488;
}

pub mod game_system_common {
    use super::Table;

    /// 只有一行（ID 0）
    pub const TABLE: Table = Table { struct_name: "GAME_SYSTEM_COMMON_PARAM_ST", row_size: 1200 };
    pub const ROW: u32 = 0;
    /// s32 地上分组掉落球（首领三选一等）的特效号（paramdef unknown_78，原版 6000）
    pub const GROUP_ORB_FXR: usize = 120;
}

pub mod bullet {
    use super::Table;

    /// paramdef 写 332 字节，NR 实际 348
    pub const TABLE: Table = Table { struct_name: "BULLET_PARAM_ST", row_size: 348 };
    /// f32 寿命（秒），生成时读取
    pub const LIFE: usize = 16;
}

pub mod atk {
    use super::Table;

    /// AtkParam_Pc 与 AtkParam_Npc 共用此类型名
    pub const TABLE: Table = Table { struct_name: "ATK_PARAM_ST", row_size: 464 };
    /// u8 第二个攻击分类（第一个在 +78：122 技艺、123 绝招、112 战技），SpEffect 的 magicSubCategoryChange 按它们决定加成算不算
    pub const SUB_CATEGORY_2: usize = 79;
}

pub mod sword_arts {
    use super::Table;

    pub const TABLE: Table = Table { struct_name: "SWORD_ARTS_PARAM_ST", row_size: 60 };
    /// 位域 isStartSkill_<角色>（出击时可改成该战技的角色），按 heroId 1..=10 排的 (字节偏移, 位)。
    /// +24 的 8 位依次为女爵、追踪者、执行者、守护者、无赖、复仇者、铁之眼、隐士，+48 第 3 位学者、第 7 位送葬者
    pub const START_SKILL_BITS: [(usize, u8); 10] = [(24, 1), (24, 3), (24, 6), (24, 0), (24, 4), (24, 5), (24, 7), (24, 2), (48, 3), (48, 7)];
}

pub mod hero {
    use super::Table;

    pub const TABLE: Table = Table { struct_name: "HERO_PARAM_ST", row_size: 176 };
    /// s32 成长表在 HeroStatusParam 中的起始 ID
    pub const STATUS_PARAM_ID: usize = 108;
}

pub mod hero_status {
    use super::Table;

    pub const TABLE: Table = Table { struct_name: "HERO_STATUS_PARAM", row_size: 28 };
    /// u16 该关键帧对应的等级
    pub const TOTAL_LEVEL: usize = 4;
    /// 8 个 s8，顺序同 ADD_STATUS
    pub const STATS: usize = 6;
}

pub mod chara_init {
    use super::Table;

    pub const TABLE: Table = Table { struct_name: "CHARACTER_INIT_PARAM", row_size: 320 };
    /// s32 初始主手 / 副手武器（EquipParamWeapon ID，-1 为空）
    pub const WEAPON_RIGHT_1: usize = 4;
    pub const WEAPON_LEFT_1: usize = 12;
    /// u8 角色（1..=10）
    pub const HERO_ID: usize = 222;
}

pub mod equip_weapon {
    use super::Table;

    pub const TABLE: Table = Table { struct_name: "EQUIP_PARAM_WEAPON_ST", row_size: 680 };
    /// u16 菜单图标编号
    pub const ICON_ID: usize = 190;
    /// u16 基础属性攻击力（attackBaseMagic / Fire / Thunder / Dark，Dark 即圣）
    pub const ATTACK_MAGIC: usize = 202;
    pub const ATTACK_FIRE: usize = 204;
    pub const ATTACK_THUNDER: usize = 206;
    pub const ATTACK_DARK: usize = 396;
    /// u16 武器类别（盾 65/67/69、法杖 57、圣印 61、弓弩 51/53/55/56、空手 33、箭矢 81/83/85/86 等）
    pub const WEP_TYPE: usize = 422;
    /// u8 本体稀有度（0 普通 / 1 少见 / 2 稀有 / 3 传说）
    pub const RARITY: usize = 576;
}

pub mod custom_weapon {
    use super::Table;

    pub const TABLE: Table = Table { struct_name: "EQUIP_PARAM_CUSTOM_WEAPON_ST", row_size: 48 };
    /// s32 对应的 EquipParamWeapon ID
    pub const TARGET_WEAPON_ID: usize = 4;
    /// s32 第一栏词条抽奖表（AttachEffectTableParam ID）
    pub const ATTACH_TABLE_1: usize = 12;
    /// s8 强化等级（加到 TARGET_WEAPON_ID 上即实际物品 ID）
    pub const WEAPON_LEVEL: usize = 24;
    /// u8 非 0 为深夜红色（[Unique]）武器
    pub const IS_CURSED: usize = 25;
}

pub mod item_lot {
    use super::Table;

    /// ItemLotParam_map 与 _enemy 共用此类型名；paramdef 写 290 字节，实际行大小 224（只用前面的字段）
    pub const TABLE: Table = Table { struct_name: "ITEMLOT_PARAM_ST", row_size: 224 };
    /// 8 个条目：s32 物品 ID、s32 类别（1 道具、7 ItemTableParam 等）、u16 权重、u8 数量
    pub const ITEM_ID: usize = 0;
    pub const CATEGORY: usize = 32;
    pub const WEIGHT: usize = 64;
    pub const NUM: usize = 138;
    pub const ENTRIES: usize = 8;
}

pub mod item_table {
    use super::Table;

    /// 同一表 ID 占连续多行，每行一个候选
    pub const TABLE: Table = Table { struct_name: "ITEMTABLE_PARAM_ST", row_size: 36 };
    /// s32 候选类别（1 = 锻体或道具，4 护符，7 嵌套掉落表）与候选 ID
    pub const CATEGORY: usize = 4;
    pub const ITEM_ID: usize = 8;
    /// u16 权重；u8 数量；u8 只给某角色抽（heroId 1 追踪者 … 10 送葬者，0 不限）；s32 选中时打开的旗标
    pub const WEIGHT: usize = 12;
    pub const QUANTITY: usize = 14;
    pub const ONLY_FOR_CLASS: usize = 15;
    pub const EVENT_FLAG: usize = 16;
}

pub mod permanent_buff {
    use super::Table;

    pub const TABLE: Table = Table { struct_name: "PERMANENT_BUFF_PARAM_ST", row_size: 24 };
    /// s32 锻体实际施加的 SpEffect（名字与说明取该效果的 TEXT_ID_1）
    pub const SP_EFFECT: usize = 0;
    /// s32 选择界面的图标
    pub const ICON_ID: usize = 4;
    /// u16 数值显示方式：2 不显示，3 显示 NUMERIC_VALUE；u16 是否带 %
    pub const DISPLAY_NUMERIC: usize = 12;
    pub const DISPLAY_PERCENT: usize = 14;
    pub const NUMERIC_VALUE: usize = 16;
    pub const DISPLAY_NONE: u16 = 2;
}

pub mod equip_goods {
    use super::Table;

    pub const TABLE: Table = Table { struct_name: "EQUIP_PARAM_GOODS_ST", row_size: 224 };
    /// s16 持有上限
    pub const MAX_NUM: usize = 58;
    /// s8 学者的持有上限（-1 为同 MAX_NUM；罐子、丸药、油脂、投掷物等消耗品才有）
    pub const BAGCRAFT_MAX_NUM: usize = 162;
}

pub mod attach_effect_table {
    use super::Table;

    /// 同一张抽奖表的各条目共用行 ID
    pub const TABLE: Table = Table { struct_name: "ATTACHEFFECT_TABLE_PARAM_ST", row_size: 12 };
    /// s32 词条（AttachEffectParam ID）
    pub const ATTACH_EFFECT_ID: usize = 4;
    /// u16 权重
    pub const CHANCE_WEIGHT: usize = 8;
    /// s16 有 DLC 时的权重，-1 为沿用 CHANCE_WEIGHT
    pub const CHANCE_WEIGHT_DLC: usize = 10;
}

pub mod equip_antique {
    use super::Table;

    pub const TABLE: Table = Table { struct_name: "EQUIP_PARAM_ANTIQUE_ST", row_size: 48 };
    /// u8 颜色（0 火燃、1 水滴、2 光耀、3 幽静）
    pub const COLOR: usize = 4;
    /// 位 0：深夜遗物
    pub const DEEP_BYTE: usize = 7;
    /// s32 菜单图标编号
    pub const ICON_ID: usize = 28;
    /// 3 个 s32：各栏正面词条抽奖表（AttachEffectTableParam ID，-1 为没有这一栏）
    pub const TABLES: usize = 16;
    /// 3 个 s32：各栏负面词条抽奖表（深夜遗物的强力栏才有）
    pub const CURSE_TABLES: usize = 32;
}

pub mod attach_effect_filter {
    use super::Table;

    /// 行 ID = AttachEffectParam 的 FILTER_PARAM
    pub const TABLE: Table = Table { struct_name: "ATTACHEFFECT_FILTER_PARAM_ST", row_size: 20 };
    /// s32 所属筛选子类（AttachEffectFilterSubCategoryParam ID）
    pub const SUB_CATEGORY: usize = 8;
}

pub mod attach_effect_filter_sub {
    use super::Table;

    pub const TABLE: Table = Table { struct_name: "ATTACHEFFECT_FILTER_SUB_CATEGORY_PARAM_ST", row_size: 32 };
    /// s32 子类名（menu CL_MenuText）
    pub const TEXT_ID: usize = 0;
    /// s32 大类：10 仅限特定角色、20 仅限特定武器、30 全部、40 减益
    pub const CATEGORY: usize = 4;
    /// s32 排序值；角色子类里就是 heroId
    pub const ORDER: usize = 8;
}

pub mod shop_lineup {
    use super::Table;

    pub const TABLE: Table = Table { struct_name: "SHOP_LINEUP_PARAM", row_size: 64 };
    /// s32 价格
    pub const VALUE: usize = 8;
}

pub mod antique_stand {
    use super::Table;

    pub const TABLE: Table = Table { struct_name: "ANTIQUE_STAND_PARAM_ST", row_size: 24 };
    /// s8 适用角色：1..=10 专属，11 通用
    pub const HERO_TYPE: usize = 8;
    /// s32 对应的道具 ID
    pub const GOODS_ID: usize = 16;
}

pub mod attach_effect {
    use super::Table;

    pub const TABLE: Table = Table { struct_name: "ATTACHEFFECT_PARAM_ST", row_size: 68 };
    /// 3 个 s32：词条常驻施加的 SpEffect（passiveSpEffectId_1~3，-1 为空）
    pub const PASSIVE_SP_EFFECT: usize = 12;
    /// s32 同类词条组（-1 为不分组）
    pub const COMPATIBILITY: usize = 24;
    /// s32 词条文本（AttachEffectName FMG），同一词条的各档共用
    pub const ATTACH_TEXT_ID: usize = 28;
    /// s32 界面显示的数值，负面词条为负数
    pub const DISPLAYED_VALUE: usize = 48;
    /// 位域：+37 的 8 位依次为追踪者~执行者，+38 低 2 位为学者、送葬者（heroId 1..=10 对应第 0..=9 位）
    pub const ALLOW_HEROES: usize = 37;
    /// 同上，按 heroId 1..=10 排的 (字节偏移, 位)
    pub const ALLOW_BITS: [(usize, u8); 10] = [(37, 0), (37, 1), (37, 2), (37, 3), (37, 4), (37, 5), (37, 6), (37, 7), (38, 0), (38, 1)];
    /// s32 遗物上词条的排序键（与 ID 一起比较）
    pub const SORT_KEY: usize = 52;
    /// s32 遗物筛选行（AttachEffectFilterParam ID）
    pub const FILTER_PARAM: usize = 56;
}

/// 深夜各深度的世界规则（行 ID 1~5 = 深度 1~5），字段都是 u8
pub mod chaos_rank {
    use super::Table;

    pub const TABLE: Table = Table { struct_name: "CHAOS_MATCHING_RANK_CONTROL_PARAM_ST", row_size: 12 };
    pub const ROWS: [u32; 5] = [1, 2, 3, 4, 5];
    /// 地图挑战权重：地图不可见 / 夜王未知 / 无
    pub const MAP_WEIGHTS: [usize; 3] = [2, 3, 4];
    /// 灾变地点数量权重：0 / 1 / 2 处
    pub const CATACLYSM_WEIGHTS: [usize; 3] = [5, 6, 7];
}

/// 深夜敌人强度表：按威胁分组，每行 5 个 s32 效果编号（对应深度 1~5）
pub mod chaos_correct {
    use super::Table;

    pub const TABLE: Table = Table { struct_name: "CHAOS_MATCHING_CORRECT_PARAM_ST", row_size: 20 };
    pub const EFFECTS: [usize; 5] = [0, 4, 8, 12, 16];
}

pub mod correct_graph {
    use super::Table;

    pub const TABLE: Table = Table { struct_name: "CACL_CORRECT_GRAPH_ST", row_size: 80 };
    /// 各 5 个 f32
    pub const STAGE_MAX_VAL: usize = 0;
    pub const STAGE_MAX_GROW_VAL: usize = 20;
    pub const ADJ_PT_MAX_GROW_VAL: usize = 40;
    /// 生命力 -> 血量、集中力 -> 专注值、耐力 -> 精力
    pub const HP: u32 = 100;
    pub const FP: u32 = 101;
    pub const STAMINA: u32 = 104;
}
