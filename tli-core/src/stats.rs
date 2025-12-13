//! 属性池模块
//!
//! 实现属性聚合、条件解析和修正应用
//!
//! ## 迁移说明
//!
//! 本模块正在向 `modifiers::ModDB` 迁移。当前 `StatAggregator` 会同时产出：
//! - `StatPool`: 旧版属性池（向后兼容）
//! - `ModDB`: 新版结构化修正存储（用于溯源和条件评估）

use crate::mechanics::{is_per_stack_stat, MechanicsProcessor};
use crate::modifiers::{ModDB, Modifier, ModifierStore};
use crate::tags::ContextTags;
use crate::types::*;
use std::collections::HashMap;

/// 属性池 - 聚合所有属性修正
#[derive(Debug, Clone, Default)]
pub struct StatPool {
    /// 基础值
    base: HashMap<String, f64>,
    /// Increased 修正（累加）
    increased: HashMap<String, f64>,
    /// More 修正（按 bucket 分组）
    more: HashMap<String, Vec<MoreModifier>>,
    /// 最终计算值缓存
    final_values: HashMap<String, f64>,
    /// 是否需要重新计算
    dirty: bool,
}

/// More 修正器
#[derive(Debug, Clone)]
pub struct MoreModifier {
    pub value: f64,
    pub bucket_id: u32,
    pub source: String,
}

impl StatPool {
    /// 创建新的属性池
    pub fn new() -> Self {
        Self::default()
    }

    /// 添加基础值
    pub fn add_base(&mut self, key: &str, value: f64) {
        *self.base.entry(key.to_string()).or_insert(0.0) += value;
        self.dirty = true;
    }

    /// 设置基础值（覆盖）
    pub fn set_base(&mut self, key: &str, value: f64) {
        self.base.insert(key.to_string(), value);
        self.dirty = true;
    }

    /// 添加 Increased 修正
    pub fn add_increased(&mut self, key: &str, value: f64) {
        *self.increased.entry(key.to_string()).or_insert(0.0) += value;
        self.dirty = true;
    }

    /// 添加 More 修正
    pub fn add_more(&mut self, key: &str, value: f64, bucket_id: u32, source: &str) {
        self.more
            .entry(key.to_string())
            .or_insert_with(Vec::new)
            .push(MoreModifier {
                value,
                bucket_id,
                source: source.to_string(),
            });
        self.dirty = true;
    }

    /// 获取基础值
    pub fn get_base(&self, key: &str) -> f64 {
        self.base.get(key).copied().unwrap_or(0.0)
    }

    /// 获取 Increased 总和
    pub fn get_increased(&self, key: &str) -> f64 {
        self.increased.get(key).copied().unwrap_or(0.0)
    }

    /// 获取 More 乘积
    pub fn get_more_multiplier(&self, key: &str) -> f64 {
        let mods = match self.more.get(key) {
            Some(m) => m,
            None => return 1.0,
        };

        if mods.is_empty() {
            return 1.0;
        }

        // 按 bucket_id 分组，同 bucket 内相乘
        let mut buckets: HashMap<u32, f64> = HashMap::new();
        for m in mods {
            let entry = buckets.entry(m.bucket_id).or_insert(1.0);
            *entry *= 1.0 + m.value;
        }

        // 所有 bucket 相乘
        buckets.values().product()
    }

    /// 计算最终值
    /// final = base * (1 + sum(increased)) * product(1 + more)
    pub fn calculate_final(&mut self, key: &str) -> f64 {
        if !self.dirty {
            if let Some(&cached) = self.final_values.get(key) {
                return cached;
            }
        }

        let base = self.get_base(key);
        let inc = self.get_increased(key);
        let more = self.get_more_multiplier(key);

        let result = base * (1.0 + inc) * more;
        self.final_values.insert(key.to_string(), result);
        
        result
    }

    /// 重新计算所有最终值
    pub fn recalculate_all(&mut self) {
        self.final_values.clear();
        let keys: Vec<String> = self.base.keys().cloned().collect();
        for key in keys {
            self.calculate_final(&key);
        }
        self.dirty = false;
    }

    /// 合并另一个属性池
    pub fn merge(&mut self, other: &StatPool) {
        for (key, value) in &other.base {
            self.add_base(key, *value);
        }
        for (key, value) in &other.increased {
            self.add_increased(key, *value);
        }
        for (key, mods) in &other.more {
            for m in mods {
                self.add_more(key, m.value, m.bucket_id, &m.source);
            }
        }
    }

    /// 获取所有基础键
    pub fn base_keys(&self) -> impl Iterator<Item = &String> {
        self.base.keys()
    }
}

/// 属性聚合器 - 从各种来源收集属性
/// 
/// 同时产出 `StatPool`（向后兼容）和 `ModDB`（新版结构化存储）
pub struct StatAggregator<'a> {
    pool: StatPool,
    context: &'a ContextTags,
    local_pool: StatPool, // 用于武器等局部属性
    /// 每件装备的局部属性池（用于暗金装备基底+词缀合并计算）
    item_local_pools: HashMap<String, ItemLocalStats>,
    /// 机制处理器（用于处理 .per_xxx 属性）
    mechanics: Option<&'a MechanicsProcessor>,
    /// 结构化修正存储（新版，用于溯源）
    mod_db: ModDB,
}

/// 单件装备的局部属性
#[derive(Debug, Clone, Default)]
pub struct ItemLocalStats {
    /// 基底属性（来自 items_meta）
    pub base_armor: f64,
    pub base_es: f64,
    pub base_evasion: f64,
    /// 词缀提供的平面属性
    pub affix_armor: f64,
    pub affix_es: f64,
    pub affix_evasion: f64,
    /// 该装备上的百分比加成
    pub armor_percent: f64,
    pub es_percent: f64,
    pub evasion_percent: f64,
}

impl<'a> StatAggregator<'a> {
    /// 创建新的聚合器
    pub fn new(context: &'a ContextTags) -> Self {
        Self {
            pool: StatPool::new(),
            context,
            local_pool: StatPool::new(),
            item_local_pools: HashMap::new(),
            mechanics: None,
            mod_db: ModDB::new(),
        }
    }
    
    /// 创建带机制处理器的聚合器
    pub fn with_mechanics(context: &'a ContextTags, mechanics: &'a MechanicsProcessor) -> Self {
        Self {
            pool: StatPool::new(),
            context,
            local_pool: StatPool::new(),
            item_local_pools: HashMap::new(),
            mechanics: Some(mechanics),
            mod_db: ModDB::new(),
        }
    }
    
    /// 设置机制处理器
    pub fn set_mechanics(&mut self, mechanics: &'a MechanicsProcessor) {
        self.mechanics = Some(mechanics);
    }

    /// 获取 ModDB 引用
    pub fn mod_db(&self) -> &ModDB {
        &self.mod_db
    }

    /// 聚合装备属性
    pub fn aggregate_items(&mut self, items: &[ItemData]) {
        for item in items {
            self.aggregate_single_item(item);
        }
    }

    /// 聚合单个装备
    pub fn aggregate_single_item(&mut self, item: &ItemData) {
        // 为每件装备创建局部属性池
        let mut item_local = ItemLocalStats::default();
        
        // 1. 处理基底固有属性（来自 items_meta）
        for (key, value) in &item.base_implicit_stats {
            match key.as_str() {
                "def.armor" => item_local.base_armor += *value,
                "base.es" => item_local.base_es += *value,
                "def.evasion" => item_local.base_evasion += *value,
                _ => {
                    if is_local_stat(key) {
                        self.local_pool.add_base(key, *value);
                    } else {
                        // 通过 apply_stat 支持 per_xxx 机制解析
                        self.apply_stat(key, *value, &format!("{}:base", item.id));
                    }
                }
            }
        }
        
        // 2. 处理暗金/传奇装备的隐性词缀
        for (key, value) in &item.implicit_stats {
            match key.as_str() {
                "def.armor" => item_local.affix_armor += *value,
                "base.es" => item_local.affix_es += *value,
                "def.evasion" => item_local.affix_evasion += *value,
                _ => {
                    if is_local_stat(key) {
                        self.local_pool.add_base(key, *value);
                    } else {
                        // 通过 apply_stat 支持 per_xxx 机制解析
                        self.apply_stat(key, *value, &format!("{}:implicit", item.id));
                    }
                }
            }
        }

        // 3. 处理词缀
        for affix in &item.affixes {
            // 检查词缀条件是否满足
            if !self.check_affix_requirements(affix) {
                continue;
            }

            for (key, value) in &affix.stats {
                // 处理该装备的局部百分比加成
                match key.as_str() {
                    "mod.inc.def.armor.local" => {
                        item_local.armor_percent += *value;
                        continue;
                    }
                    "mod.inc.base.es.local" => {
                        item_local.es_percent += *value;
                        continue;
                    }
                    "mod.inc.def.evasion.local" => {
                        item_local.evasion_percent += *value;
                        continue;
                    }
                    // 平面局部属性
                    "def.armor" => {
                        item_local.affix_armor += *value;
                        continue;
                    }
                    "base.es" => {
                        item_local.affix_es += *value;
                        continue;
                    }
                    "def.evasion" => {
                        item_local.affix_evasion += *value;
                        continue;
                    }
                    _ => {}
                }
                
                if affix.is_local || is_local_stat(key) {
                    // 其他局部属性（如武器物理伤害）
                    Self::apply_stat_to_pool(&mut self.local_pool, key, *value);
                } else {
                    // 全局属性
                    self.apply_stat(key, *value, &format!("{}:{}", item.id, affix.id));
                }
            }
        }
        
        // 保存该装备的局部属性池
        self.item_local_pools.insert(item.id.clone(), item_local);
    }

    /// 检查词缀条件是否满足
    fn check_affix_requirements(&self, affix: &AffixData) -> bool {
        if affix.requirements.is_empty() {
            return true;
        }

        // 将字符串需求转换为 ID
        let req_ids: Vec<u32> = affix
            .requirements
            .iter()
            .filter_map(|name| self.context.registry().get_id(name))
            .collect();

        self.context.matches_requirements(&req_ids)
    }

    /// 应用属性到池
    /// 
    /// 如果是 .per_xxx 类型的属性，会根据机制层数计算实际值
    fn apply_stat(&mut self, key: &str, value: f64, source: &str) {
        // 检查是否是 per_xxx 类型的属性
        if is_per_stack_stat(key) {
            if let Some(mechanics) = &self.mechanics {
                if let Some((base_key, total_value)) = mechanics.calculate_per_stack_value(key, value) {
                    Self::apply_stat_to_pool(&mut self.pool, &base_key, total_value);
                    self.add_to_mod_db(&base_key, total_value, source);
                }
                // 如果机制未激活或层数为0，跳过该属性
            }
            // 如果没有机制处理器，也跳过（无法计算层数）
        } else {
            Self::apply_stat_to_pool(&mut self.pool, key, value);
            self.add_to_mod_db(key, value, source);
        }
    }

    /// 应用属性到指定池（静态方法，仅更新 StatPool）
    fn apply_stat_to_pool(pool: &mut StatPool, key: &str, value: f64) {
        // 根据键名前缀判断类型
        if key.starts_with("mod.inc.") {
            pool.add_increased(&key.replace("mod.inc.", ""), value);
        } else if key.starts_with("mod.more.") {
            // More 修正默认使用 bucket 0
            pool.add_more(&key.replace("mod.more.", ""), value, 0, "item");
        } else if key.starts_with("speed.") {
            // 速度类统一视为 Increased
            pool.add_increased(key, value);
        } else if key.starts_with("crit.dmg") {
            // 暴击伤害类统一视为 Increased
            pool.add_increased("crit.dmg", value);
        } else {
            pool.add_base(key, value);
        }
    }

    /// 添加到 ModDB（结构化存储）
    fn add_to_mod_db(&mut self, key: &str, value: f64, source: &str) {
        let modifier = if key.starts_with("mod.inc.") {
            let stripped_key = key.replace("mod.inc.", "");
            Modifier::inc(&stripped_key, value, source)
        } else if key.starts_with("mod.more.") {
            let stripped_key = key.replace("mod.more.", "");
            Modifier::more(&stripped_key, value, source)
        } else if key.starts_with("speed.") {
            // 速度类视为 Inc
            Modifier::inc(key, value, source)
        } else if key.starts_with("crit.dmg") {
            // 暴击伤害类视为 Inc
            Modifier::inc("crit.dmg", value, source)
        } else {
            Modifier::base(key, value, source)
        };
        self.mod_db.add(modifier);
    }
    
    /// 应用机制基础效果
    /// 
    /// 将所有激活机制的基础效果（每层提供的属性）应用到属性池
    pub fn apply_mechanic_base_effects(&mut self) {
        if let Some(mechanics) = &self.mechanics {
            let effects = mechanics.calculate_base_effects();
            for (key, value) in effects {
                Self::apply_stat_to_pool(&mut self.pool, &key, value);
                self.add_to_mod_db(&key, value, "mechanic_effect");
            }
        }
    }
    
    /// 获取属性池的可变引用（内部使用）
    pub fn pool_mut(&mut self) -> &mut StatPool {
        &mut self.pool
    }

    /// 聚合技能属性
    pub fn aggregate_skill(&mut self, skill: &SkillData) {
        // 技能基础伤害
        for (key, value) in &skill.base_damage {
            self.pool.add_base(key, *value);
            self.mod_db.add(Modifier::base(key, *value, &format!("skill:{}", skill.id)));
        }

        // 技能自带属性
        for (key, value) in &skill.stats {
            self.apply_stat(key, *value, &format!("skill:{}", skill.id));
        }
    }

    /// 聚合辅助技能属性
    pub fn aggregate_support_skills(&mut self, supports: &[SkillData]) {
        for (idx, support) in supports.iter().enumerate() {
            let bucket_id = (idx + 100) as u32; // 辅助技能使用 100+ 的 bucket
            let source = format!("support:{}", support.id);
            
            for (key, value) in &support.stats {
                if key.starts_with("mod.more.") {
                    // 辅助技能的 More 使用独立 bucket
                    let stripped_key = key.replace("mod.more.", "");
                    self.pool.add_more(&stripped_key, *value, bucket_id, &support.id);
                    // 同时添加到 ModDB（带 bucket）
                    self.mod_db.add(Modifier::more_with_bucket(&stripped_key, *value, bucket_id, &source));
                } else {
                    self.apply_stat(key, *value, &source);
                }
            }
        }
    }

    /// 聚合全局覆盖
    pub fn aggregate_overrides(&mut self, overrides: &HashMap<String, f64>) {
        for (key, value) in overrides {
            self.apply_stat(key, *value, "global_override");
        }
    }

    /// 应用局部属性到最终池
    /// 
    /// 关键规则：暗金装备 = 基底装备属性 + 暗金词缀属性
    /// 例如：玛格努斯的旧律的护甲 = 1777（基底）+ 2880~3456（暗金词缀）
    /// 如果有 "+X% 该装备护甲" 词缀，则：最终护甲 = (基底 + 词缀) * (1 + X%)
    pub fn finalize_local_stats(&mut self) {
        // 1. 计算所有装备的局部防御属性并汇总
        let mut total_armor: f64 = 0.0;
        let mut total_es: f64 = 0.0;
        let mut total_evasion: f64 = 0.0;
        
        for (_item_id, local_stats) in &self.item_local_pools {
            // 合并基底和词缀属性，然后应用百分比加成
            // final = (base + affix_flat) * (1 + percent)
            let item_armor = (local_stats.base_armor + local_stats.affix_armor) 
                * (1.0 + local_stats.armor_percent);
            let item_es = (local_stats.base_es + local_stats.affix_es) 
                * (1.0 + local_stats.es_percent);
            let item_evasion = (local_stats.base_evasion + local_stats.affix_evasion) 
                * (1.0 + local_stats.evasion_percent);
            
            total_armor += item_armor;
            total_es += item_es;
            total_evasion += item_evasion;
        }
        
        // 将装备提供的防御值添加到全局池
        if total_armor > 0.0 {
            self.pool.add_base("def.armor", total_armor);
        }
        if total_es > 0.0 {
            self.pool.add_base("base.es", total_es);
        }
        if total_evasion > 0.0 {
            self.pool.add_base("def.evasion", total_evasion);
        }
        
        // 2. 武器物理伤害计算
        // final_phys = base_phys * (1 + local_inc)
        let base_phys_min = self.local_pool.get_base("dmg.phys.min");
        let base_phys_max = self.local_pool.get_base("dmg.phys.max");
        let local_phys_inc = self.local_pool.get_increased("dmg.phys");

        if base_phys_min > 0.0 || base_phys_max > 0.0 {
            let final_phys_min = base_phys_min * (1.0 + local_phys_inc);
            let final_phys_max = base_phys_max * (1.0 + local_phys_inc);
            self.pool.set_base("dmg.phys.min", final_phys_min);
            self.pool.set_base("dmg.phys.max", final_phys_max);
        }

        // 3. 武器暴击率
        let base_crit = self.local_pool.get_base("crit.chance.local");
        if base_crit > 0.0 {
            self.pool.add_base("crit.chance", base_crit);
        }

        // 4. 武器攻速（局部）
        let local_speed = self.local_pool.get_base("speed.attack.local");
        if local_speed > 0.0 {
            self.pool.set_base("weapon.base_speed", local_speed);
        }
    }

    /// 获取最终的属性池和 ModDB
    /// 
    /// 返回值: (StatPool, ModDB)
    /// - StatPool: 向后兼容的属性池
    /// - ModDB: 结构化修正存储（用于溯源和条件评估）
    pub fn finalize(mut self) -> (StatPool, ModDB) {
        self.finalize_local_stats();
        self.pool.recalculate_all();
        (self.pool, self.mod_db)
    }

    /// 获取最终的属性池（仅 StatPool，向后兼容）
    pub fn finalize_pool_only(mut self) -> StatPool {
        self.finalize_local_stats();
        self.pool.recalculate_all();
        self.pool
    }
}

/// 判断是否为局部属性
fn is_local_stat(key: &str) -> bool {
    key.ends_with(".local") || 
    key.starts_with("dmg.phys.") && !key.contains("mod.") ||
    key == "crit.chance.local" ||
    key == "speed.attack.local"
}

/// 条件表达式解析器
pub struct ConditionParser;

impl ConditionParser {
    /// 解析并评估条件表达式
    /// 支持简单的比较: "life_percent <= 0.35", "is_moving == true"
    pub fn evaluate(condition: &str, context_flags: &HashMap<String, bool>, context_values: &HashMap<String, f64>) -> bool {
        let condition = condition.trim();
        
        // 尝试解析布尔条件: "key == true/false"
        if let Some((key, expected)) = Self::parse_bool_condition(condition) {
            return context_flags.get(&key).copied().unwrap_or(false) == expected;
        }
        
        // 尝试解析数值条件: "key <= value"
        if let Some((key, op, threshold)) = Self::parse_numeric_condition(condition) {
            let actual = context_values.get(&key).copied().unwrap_or(0.0);
            return match op.as_str() {
                "<=" => actual <= threshold,
                ">=" => actual >= threshold,
                "<" => actual < threshold,
                ">" => actual > threshold,
                "==" => (actual - threshold).abs() < f64::EPSILON,
                "!=" => (actual - threshold).abs() >= f64::EPSILON,
                _ => false,
            };
        }
        
        // 尝试解析字符串条件: "key == 'value'"
        if let Some((key, expected)) = Self::parse_string_condition(condition) {
            // 字符串条件暂时用 context_flags 模拟
            let flag_key = format!("{}_{}", key, expected);
            return context_flags.get(&flag_key).copied().unwrap_or(false);
        }
        
        // 默认为假
        false
    }

    fn parse_bool_condition(condition: &str) -> Option<(String, bool)> {
        let parts: Vec<&str> = condition.split("==").collect();
        if parts.len() != 2 {
            return None;
        }
        
        let key = parts[0].trim().to_string();
        let value = parts[1].trim();
        
        match value {
            "true" => Some((key, true)),
            "false" => Some((key, false)),
            _ => None,
        }
    }

    fn parse_numeric_condition(condition: &str) -> Option<(String, String, f64)> {
        let operators = ["<=", ">=", "!=", "==", "<", ">"];
        
        for op in operators {
            if condition.contains(op) {
                let parts: Vec<&str> = condition.split(op).collect();
                if parts.len() != 2 {
                    continue;
                }
                
                let key = parts[0].trim().to_string();
                if let Ok(value) = parts[1].trim().parse::<f64>() {
                    return Some((key, op.to_string(), value));
                }
            }
        }
        
        None
    }

    fn parse_string_condition(condition: &str) -> Option<(String, String)> {
        let parts: Vec<&str> = condition.split("==").collect();
        if parts.len() != 2 {
            return None;
        }
        
        let key = parts[0].trim().to_string();
        let value = parts[1].trim().trim_matches('\'').trim_matches('"').to_string();
        
        Some((key, value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stat_pool_calculation() {
        let mut pool = StatPool::new();
        
        // 基础值 100
        pool.add_base("dmg.fire", 100.0);
        // +50% increased
        pool.add_increased("dmg.fire", 0.5);
        // +20% more (bucket 0)
        pool.add_more("dmg.fire", 0.2, 0, "skill");
        // +30% more (bucket 1)
        pool.add_more("dmg.fire", 0.3, 1, "support");
        
        let result = pool.calculate_final("dmg.fire");
        // 100 * 1.5 * 1.2 * 1.3 = 234
        assert!((result - 234.0).abs() < 0.01);
    }

    #[test]
    fn test_condition_parser() {
        let mut flags = HashMap::new();
        flags.insert("is_moving".to_string(), true);
        flags.insert("enemy_range_near".to_string(), true);
        
        let mut values = HashMap::new();
        values.insert("life_percent".to_string(), 0.3);
        
        // 布尔条件
        assert!(ConditionParser::evaluate("is_moving == true", &flags, &values));
        assert!(!ConditionParser::evaluate("is_moving == false", &flags, &values));
        
        // 数值条件
        assert!(ConditionParser::evaluate("life_percent <= 0.35", &flags, &values));
        assert!(!ConditionParser::evaluate("life_percent >= 0.5", &flags, &values));
    }
    
    #[test]
    fn test_unique_item_armor_calculation() {
        // 测试暗金装备护甲计算
        // 场景：玛格努斯的旧律
        // 基底护甲（来自 items_meta）: 1777
        // 暗金词缀护甲: 3000 (范围 2880-3456 的中间值)
        // 预期总护甲: 1777 + 3000 = 4777
        
        let item_local = ItemLocalStats {
            base_armor: 1777.0,
            affix_armor: 3000.0,
            armor_percent: 0.0,
            ..Default::default()
        };
        
        let final_armor = (item_local.base_armor + item_local.affix_armor) 
            * (1.0 + item_local.armor_percent);
        
        assert!((final_armor - 4777.0).abs() < 0.01);
    }
    
    #[test]
    fn test_unique_item_armor_with_percent() {
        // 测试带百分比加成的暗金装备护甲计算
        // 基底护甲: 1777
        // 暗金词缀护甲: 3000
        // +30% 该装备护甲值词缀
        // 预期总护甲: (1777 + 3000) * 1.30 = 6210.1
        
        let item_local = ItemLocalStats {
            base_armor: 1777.0,
            affix_armor: 3000.0,
            armor_percent: 0.30,
            ..Default::default()
        };
        
        let final_armor = (item_local.base_armor + item_local.affix_armor) 
            * (1.0 + item_local.armor_percent);
        
        assert!((final_armor - 6210.1).abs() < 0.1);
    }
    
    #[test]
    fn test_unique_item_es_calculation() {
        // 测试暗金装备护盾计算
        // 场景：伊斯拉菲尔的旧律
        // 基底护盾（来自 items_meta）: 120
        // 暗金词缀护盾: 370 (范围 340-408 的中间值)
        // 预期总护盾: 120 + 370 = 490
        
        let item_local = ItemLocalStats {
            base_es: 120.0,
            affix_es: 370.0,
            es_percent: 0.0,
            ..Default::default()
        };
        
        let final_es = (item_local.base_es + item_local.affix_es) 
            * (1.0 + item_local.es_percent);
        
        assert!((final_es - 490.0).abs() < 0.01);
    }
}

