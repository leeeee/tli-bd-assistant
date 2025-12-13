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
    
    /// 机制状态列表 (祝福、球类等)
    #[serde(default)]
    pub mechanic_states: Vec<MechanicState>,
    
    /// 机制定义（从数据库预加载）
    #[serde(default)]
    pub mechanic_definitions: Vec<MechanicDefinition>,
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
    
    /// 基底类型（关联 items_meta 表）
    pub base_type: String,
    
    /// 槽位
    pub slot: SlotType,
    
    /// 是否为双手武器
    #[serde(default)]
    pub is_two_handed: bool,
    
    /// 基底固有属性（来自 items_meta，如护甲基底值 1777）
    /// 这是装备基底本身的属性，与暗金词缀分开
    #[serde(default)]
    pub base_implicit_stats: HashMap<String, f64>,
    
    /// 暗金/传奇装备的隐性词缀属性
    /// 例如暗金装备提供的额外护甲 2880-3456
    #[serde(default)]
    pub implicit_stats: HashMap<String, f64>,
    
    /// 词缀列表
    #[serde(default)]
    pub affixes: Vec<AffixData>,
    
    /// 装备标签
    #[serde(default)]
    pub tags: Vec<String>,
    
    /// 是否为暗金/传奇装备
    #[serde(default)]
    pub is_unique: bool,
    
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
// 机制系统
// ============================================================

/// 机制状态
/// 
/// 表示玩家当前的机制状态（如祝福层数、球类数量等）
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../bindings/")]
pub struct MechanicState {
    /// 机制 ID (如 "focus_blessing", "fortify_blessing")
    pub id: String,
    
    /// 当前层数/值
    #[serde(default)]
    pub current_stacks: u32,
    
    /// 当前上限（可能被装备/天赋修改）
    #[serde(default = "default_max_stacks")]
    pub max_stacks: u32,
    
    /// 是否激活（满足获取条件）
    #[serde(default)]
    pub is_active: bool,
}

fn default_max_stacks() -> u32 { 4 }

impl Default for MechanicState {
    fn default() -> Self {
        Self {
            id: String::new(),
            current_stacks: 0,
            max_stacks: 4,
            is_active: false,
        }
    }
}

/// 机制定义
/// 
/// 描述一种机制的元数据和基础效果（从数据库加载）
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../bindings/")]
pub struct MechanicDefinition {
    /// 机制 ID
    pub id: String,
    
    /// 显示名称
    pub display_name: String,
    
    /// 机制分类 (blessing, charge, resource)
    #[serde(default)]
    pub category: String,
    
    /// 关联的标签键 (如 "Mech_Blessing")
    #[serde(default)]
    pub tag_key: String,
    
    /// 默认最大层数
    #[serde(default = "default_max_stacks")]
    pub default_max_stacks: u32,
    
    /// 每层基础效果
    /// 如 {"mod.inc.dmg.all": 0.04} 表示每层 +4% 全伤害
    #[serde(default)]
    pub base_effect_per_stack: HashMap<String, f64>,
    
    /// 描述
    #[serde(default)]
    pub description: String,
}

impl Default for MechanicDefinition {
    fn default() -> Self {
        Self {
            id: String::new(),
            display_name: String::new(),
            category: "blessing".to_string(),
            tag_key: String::new(),
            default_max_stacks: 4,
            base_effect_per_stack: HashMap::new(),
            description: String::new(),
        }
    }
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

/// 伤害乘区明细
/// 
/// 借鉴 ZSim 架构，将伤害计算拆分为独立乘区，便于验证和调试
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../bindings/")]
pub struct MultiplierBreakdown {
    /// 基础伤害区 (技能基础伤害 × 效用)
    pub base_damage_zone: f64,
    
    /// 增伤区 (1 + sum(increased))
    /// 包含：属性增伤、技能类型增伤、标签增伤、全增伤
    pub increased_zone: f64,
    
    /// More 乘区 (product of more multipliers)
    /// 各 bucket 的 more 效果相乘
    pub more_zone: f64,
    
    /// 暴击期望区 (1 + crit_chance × crit_damage)
    pub crit_zone: f64,
    
    /// 速度区 (攻击/施法速度)
    pub speed_zone: f64,
    
    /// 命中区 (命中率)
    pub hit_zone: f64,
    
    /// 防御区 (敌人护甲减伤系数)
    /// 公式: 攻击方等级基数 / (敌人有效防御 + 攻击方等级基数)
    pub defense_zone: f64,
    
    /// 抗性区 (1 - 敌人抗性 + 抗性降低 + 抗性穿透)
    pub resistance_zone: f64,
    
    /// 易伤区 (敌人受到额外伤害)
    pub vulnerability_zone: f64,
    
    /// 机制特殊区 (祝福、球类等机制提供的额外乘区)
    pub mechanics_zone: f64,
    
    /// 各乘区的详细来源追踪
    #[serde(default)]
    pub zone_sources: HashMap<String, Vec<ZoneSource>>,
}

/// 乘区来源详情
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../bindings/")]
pub struct ZoneSource {
    /// 来源名称 (装备名、技能名等)
    pub source: String,
    /// 贡献值
    pub value: f64,
    /// 属性键
    pub stat_key: String,
}

impl Default for ZoneSource {
    fn default() -> Self {
        Self {
            source: String::new(),
            value: 0.0,
            stat_key: String::new(),
        }
    }
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
    
    /// 乘区明细 (新增)
    /// 提供各计算阶段的详细乘区分解
    #[serde(default)]
    pub multipliers: MultiplierBreakdown,
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

/// 属性修正器（旧版，已废弃）
/// 
/// 请使用 `modifiers::Modifier` 替代
#[deprecated(note = "Use modifiers::Modifier instead")]
#[derive(Debug, Clone)]
pub struct LegacyModifier {
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

