//! 重平衡用到的参数表与字段偏移：Smithbox NR paramdef 按 regulation 10350000 过滤字段后计算，行大小与原版文件核对过。

pub struct Table {
    /// regulation 里的文件名（不含 .param）
    pub file: &'static str,
    pub struct_name: &'static str,
    pub row_size: usize,
}

pub mod npc {
    use super::Table;

    pub const TABLE: Table = Table { file: "NpcParam", struct_name: "NPC_PARAM_ST", row_size: 848 };
    /// s32 行为变体（BehaviorParam.variationId）
    pub const BEHAVIOR_VARIATION: usize = 4;
    /// u32 血量
    pub const HP: usize = 36;
    /// s32 spEffectID0..31：常驻 SpEffect 槽（分三段存放）
    pub const SP_EFFECT_SLOTS: [usize; 32] = [
        76, 80, 84, 88, 92, 96, 100, 104, 376, 380, 384, 388, 392, 396, 400, 404, 488, 492, 496, 500, 504, 508, 512, 516, 520, 524,
        528, 532, 536, 540, 544, 548,
    ];
    /// s32 强度档位相关的 5 个字段：spEffectID3（档位倍率）、multiPlayCorrectionParamId、day2SpEffectID、
    /// chaosMatchingCorrectParamId（深夜各层修正）、chaosMatchingSpEffectSetParamId
    pub const TIER_FIELDS: [usize; 5] = [88, 472, 732, 816, 820];
    pub const CHAOS_CORRECT: usize = 816;
}

pub mod behavior {
    use super::Table;

    pub const TABLE: Table = Table { file: "BehaviorParam", struct_name: "BEHAVIOR_PARAM_ST", row_size: 32 };
    pub const VARIATION: usize = 0;
    /// u8 引用类型：0 攻击（AtkParam_Npc）、1 子弹（Bullet）
    pub const REF_TYPE: usize = 9;
    pub const REF_ID: usize = 12;
    pub const REF_ATTACK: u8 = 0;
    pub const REF_BULLET: u8 = 1;
}

pub mod bullet {
    use super::Table;

    /// paramdef 写 332 字节，NR 实际 348
    pub const TABLE: Table = Table { file: "Bullet", struct_name: "BULLET_PARAM_ST", row_size: 348 };
    pub const ATK_ID: usize = 0;
}

pub mod atk {
    use super::Table;

    pub const TABLE: Table = Table { file: "AtkParam_Npc", struct_name: "ATK_PARAM_ST", row_size: 464 };
    /// u16 物理 / 魔力 / 火 / 雷 / 圣 攻击力
    pub const DAMAGE: [usize; 5] = [80, 82, 84, 86, 392];
}

pub mod sp_effect {
    use super::Table;

    pub const TABLE: Table = Table { file: "SpEffectParam", struct_name: "SP_EFFECT_PARAM_ST", row_size: 1024 };
    /// f32 物理 / 魔力 / 火 / 雷 / 圣 攻击力倍率（与档位、深夜修正连乘）
    pub const ATTACK_POWER_RATES: [usize; 5] = [72, 76, 80, 84, 480];
    /// 原版 "Enemy Scaling: Minimal Threat"：常驻、全部倍率为 1，作降攻效果的模板
    pub const NEUTRAL_TEMPLATE: u32 = 7700;
}
