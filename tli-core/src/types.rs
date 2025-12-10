//! 核心类型定义
//!
//! 使用 ts-rs 导出 TypeScript 类型绑定

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use ts_rs::TS;

// ============================================================
// 输入结构
// ============================================================

/// 计算器主输入结构
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../bindings/")]
pub struct CalculatorInput {
    /// 动态上下文标志 (如 "is_moving": true, "enemy_shocked": true)
    #[serde(default)]
    pub context_flags: HashMap<String, bool>,
    
    /// 动态上下文数值 (如 "enemy_range": 10.0)
    #[serde(default)]
    pub context_values: HashMap<String, f64>,
    
    /// 目标配置 (影响减伤公式)
    pub target_config: TargetConfig,
    
    /// 装备数据列表
    #[serde(default)]
    pub items: Vec<ItemData>,
    
    /// 主技能
    pub active_skill: SkillData,
    
    /// 辅助技能列表 (提供 More 和 Mana Multiplier)
    #[serde(default)]
    pub support_skills: Vec<SkillData>,
    
    /// 全局属性覆盖 (天赋盘/手动输入)
    #[serde(default)]
    pub global_overrides: HashMap<String, f64>,
    
    /// 预览槽位 (用于 Diff 计算)
    #[serde(default)]
    pub preview_slot: Option<PreviewSlot>,
}

/// 预览槽位
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../bindings/")]
pub struct PreviewSlot {
    pub slot_type: SlotType,
    pub item: ItemData,
}

/// 目标配置
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../bindings/")]
pub struct TargetConfig {
    /// 目标等级
    #[serde(default = "default_level")]
    pub level: u32,
    
    /// 防御常数
    #[serde(default)]
    pub defense_constant: f64,
    
    /// 抗性映射 {"fire": 0.3, "cold": 0.3, ...}
    #[serde(default)]
    pub resistances: HashMap<String, f64>,
    
    /// 通用减伤
    #[serde(default)]
    pub generic_dr: f64,
    
    /// 护甲值
    #[serde(default)]
    pub armor: u32,
    
    /// 闪避值
    #[serde(default)]
    pub evasion: u32,
}

fn default_level() -> u32 { 100 }

impl Default for TargetConfig {
    fn default() -> Self {
        Self {
            level: 100,
            defense_constant: 0.0,
            resistances: HashMap::new(),
            generic_dr: 0.0,
            armor: 0,
            evasion: 0,
        }
    }
}

/// 槽位类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../bindings/")]
#[serde(rename_all = "snake_case")]
pub enum SlotType {
    WeaponMain,
    WeaponOff,
    Helmet,
    Chest,
    Gloves,
    Boots,
    Amulet,
    Ring1,
    Ring2,
    Belt,
    Memory1,
    Memory2,
    Memory3,
    Memory4,
    Memory5,
    Memory6,
}

// ============================================================
// 装备数据
// ============================================================

/// 装备数据
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../bindings/")]
pub struct ItemData {
    /// 装备唯一 ID
    pub id: String,
    
    /// 基底类型
    pub base_type: String,
    
    /// 槽位
    pub slot: SlotType,
    
    /// 是否为双手武器
    #[serde(default)]
    pub is_two_handed: bool,
    
    /// 基底固有属性
    #[serde(default)]
    pub implicit_stats: HashMap<String, f64>,
    
    /// 词缀列表
    #[serde(default)]
    pub affixes: Vec<AffixData>,
    
    /// 装备标签
    #[serde(default)]
    pub tags: Vec<String>,
    
    /// 是否为侵蚀状态
    #[serde(default)]
    pub is_corrupted: bool,
}

/// 词缀数据
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../bindings/")]
pub struct AffixData {
    /// 词缀 ID
    pub id: String,
    
    /// 词缀组（同组互斥）
    pub group: String,
    
    /// 当前数值（在 min-max 范围内）
    pub value: f64,
    
    /// 属性效果
    pub stats: HashMap<String, f64>,
    
    /// 词缀标签
    #[serde(default)]
    pub tags: Vec<String>,
    
    /// 生效条件标签
    #[serde(default)]
    pub requirements: Vec<String>,
    
    /// 是否为局部属性
    #[serde(default)]
    pub is_local: bool,
}

// ============================================================
// 技能数据
// ============================================================

/// 技能数据
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../bindings/")]
pub struct SkillData {
    /// 技能 ID
    pub id: String,
    
    /// 技能类型
    pub skill_type: SkillType,
    
    /// 主伤害类型
    #[serde(default)]
    pub damage_type: Option<String>,
    
    /// 是否为攻击（影响使用攻速还是施法速度）
    #[serde(default)]
    pub is_attack: bool,
    
    /// 技能等级
    #[serde(default = "default_skill_level")]
    pub level: u32,
    
    /// 基础伤害 (1级默认值，实际计算时由等级数据覆盖)
    #[serde(default)]
    pub base_damage: HashMap<String, f64>,
    
    /// 基础时间（秒）
    #[serde(default = "default_base_time")]
    pub base_time: f64,
    
    /// 冷却时间（秒）
    #[serde(default)]
    pub cooldown: Option<f64>,
    
    /// 魔力消耗
    #[serde(default)]
    pub mana_cost: u32,
    
    /// Damage Effectiveness (1级默认值，实际计算时由等级数据覆盖)
    #[serde(default = "default_effectiveness")]
    pub effectiveness: f64,
    
    /// 技能标签
    #[serde(default)]
    pub tags: Vec<String>,
    
    /// 技能自带属性
    #[serde(default)]
    pub stats: HashMap<String, f64>,
    
    /// 辅助技能：注入的标签
    #[serde(default)]
    pub injected_tags: Vec<String>,
    
    /// 辅助技能：魔力倍率
    #[serde(default = "default_mana_multiplier")]
    pub mana_multiplier: f64,
    
    /// 技能等级数据 (1-20级详细数据，可选)
    #[serde(default)]
    pub level_data: Option<SkillLevelData>,
    
    /// 缩放规则 (21级及以上)
    #[serde(default)]
    pub scaling_rules: Vec<SkillScalingRule>,
}

/// 技能等级数据
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../bindings/")]
pub struct SkillLevelData {
    /// 伤害倍率 (Damage Effectiveness)，如 1.63 = 163%
    pub effectiveness: f64,
    
    /// 基础伤害
    pub base_damage: HashMap<String, f64>,
    
    /// 魔力消耗 (可选，覆盖默认值)
    #[serde(default)]
    pub mana_cost: Option<u32>,
    
    /// 施法/攻击时间 (可选，覆盖默认值)
    #[serde(default)]
    pub base_time: Option<f64>,
    
    /// 额外效果 (如弹射次数、AOE 半径等)
    #[serde(default)]
    pub extra_effects: HashMap<String, f64>,
    
    /// 该等级额外属性加成
    #[serde(default)]
    pub stats: HashMap<String, f64>,
}

/// 技能等级缩放规则
/// 
/// 用于 21 级及以上的伤害缩放计算：
/// - 21-30 级：每级额外 +10% 伤害（叠乘）
/// - 31 级及以上：每级额外 +8% 伤害（叠乘）
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../bindings/")]
pub struct SkillScalingRule {
    /// 等级范围起始（含）
    pub level_start: u32,
    
    /// 等级范围结束（含），None 表示无上限
    #[serde(default)]
    pub level_end: Option<u32>,
    
    /// 每级伤害乘数，如 1.10 = +10%
    pub multiplier_per_level: f64,
}

fn default_skill_level() -> u32 { 1 }
fn default_base_time() -> f64 { 1.0 }
fn default_effectiveness() -> f64 { 1.0 }
fn default_mana_multiplier() -> f64 { 1.0 }

/// 技能类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../bindings/")]
#[serde(rename_all = "snake_case")]
pub enum SkillType {
    Active,
    Support,
    Aura,
}

// ============================================================
// 输出结构
// ============================================================

/// 计算结果
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../bindings/")]
pub struct CalculatorOutput {
    /// 理论 DPS (Hit Dmg * Rate)
    pub dps_theoretical: f64,
    
    /// 有效 DPS (考虑命中、抗性等)
    pub dps_effective: f64,
    
    /// 单次命中伤害
    pub hit_damage: f64,
    
    /// 攻击/施法速率
    pub rate: f64,
    
    /// 暴击率
    pub crit_chance: f64,
    
    /// 暴击伤害倍率
    pub crit_multiplier: f64,
    
    /// 命中率
    pub hit_chance: f64,
    
    /// EHP 系列
    pub ehp_series: EhpSeries,
    
    /// 伤害构成明细
    pub damage_breakdown: DamageBreakdown,
    
    /// 调试追踪（标签匹配溯源）
    #[serde(default)]
    pub debug_trace: Vec<TraceEntry>,
}

/// EHP 系列
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../bindings/")]
pub struct EhpSeries {
    /// 物理 EHP
    pub physical: f64,
    /// 火焰 EHP
    pub fire: f64,
    /// 冰霜 EHP
    pub cold: f64,
    /// 闪电 EHP
    pub lightning: f64,
    /// 混沌 EHP
    pub chaos: f64,
}

/// 伤害构成明细
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../bindings/")]
pub struct DamageBreakdown {
    /// 各伤害类型的最终伤害
    pub by_type: HashMap<String, f64>,
    
    /// 基础伤害（转化前）
    pub base_damage: f64,
    
    /// Inc 总和
    pub total_increased: f64,
    
    /// More 乘积
    pub total_more: f64,
    
    /// 转化后的伤害分布
    pub after_conversion: HashMap<String, DamageWithHistory>,
}

/// 带历史标签的伤害
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../bindings/")]
pub struct DamageWithHistory {
    /// 当前伤害值
    pub damage: f64,
    /// 历史标签（用于 Tag Retention）
    pub history_tags: Vec<String>,
}

/// 调试追踪条目
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../bindings/")]
pub struct TraceEntry {
    /// 阶段名称
    pub phase: String,
    /// 描述
    pub description: String,
    /// 相关数值
    #[serde(default)]
    pub values: HashMap<String, f64>,
    /// 匹配的标签
    #[serde(default)]
    pub matched_tags: Vec<String>,
}

// ============================================================
// 内部计算类型
// ============================================================

/// 属性修正器
#[derive(Debug, Clone)]
pub struct Modifier {
    /// 属性键
    pub key: String,
    /// 数值
    pub value: f64,
    /// 修正类型
    pub mod_type: ModifierType,
    /// 生效条件标签
    pub requirements: Vec<u32>, // 整数化的标签 ID
    /// 来源
    pub source: String,
    /// 是否为局部属性
    pub is_local: bool,
    /// More 修正的 bucket ID（同 bucket 内相乘）
    pub bucket_id: Option<u32>,
}

/// 修正类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModifierType {
    /// 基础值
    Base,
    /// 增加（累加）
    Increased,
    /// 提高（相乘）
    More,
    /// 覆盖
    Override,
}

/// 伤害池（用于转化计算）
#[derive(Debug, Clone, Default)]
pub struct DamagePool {
    /// 物理伤害
    pub physical: DamageEntry,
    /// 火焰伤害
    pub fire: DamageEntry,
    /// 冰霜伤害
    pub cold: DamageEntry,
    /// 闪电伤害
    pub lightning: DamageEntry,
    /// 混沌伤害
    pub chaos: DamageEntry,
}

/// 伤害条目（带历史标签）
#[derive(Debug, Clone, Default)]
pub struct DamageEntry {
    /// 伤害下限
    pub min: f64,
    /// 伤害上限
    pub max: f64,
    /// 历史标签 ID 集合（BitSet）
    pub history_tags: fixedbitset::FixedBitSet,
}

impl DamageEntry {
    pub fn new(min: f64, max: f64, tag_count: usize) -> Self {
        Self {
            min,
            max,
            history_tags: fixedbitset::FixedBitSet::with_capacity(tag_count),
        }
    }
    
    pub fn average(&self) -> f64 {
        (self.min + self.max) / 2.0
    }
}

