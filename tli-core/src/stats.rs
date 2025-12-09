//! 属性池模块
//!
//! 实现属性聚合、条件解析和修正应用

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
pub struct StatAggregator<'a> {
    pool: StatPool,
    context: &'a ContextTags,
    local_pool: StatPool, // 用于武器等局部属性
}

impl<'a> StatAggregator<'a> {
    /// 创建新的聚合器
    pub fn new(context: &'a ContextTags) -> Self {
        Self {
            pool: StatPool::new(),
            context,
            local_pool: StatPool::new(),
        }
    }

    /// 聚合装备属性
    pub fn aggregate_items(&mut self, items: &[ItemData]) {
        for item in items {
            self.aggregate_single_item(item);
        }
    }

    /// 聚合单个装备
    fn aggregate_single_item(&mut self, item: &ItemData) {
        // 1. 处理基底固有属性
        for (key, value) in &item.implicit_stats {
            if is_local_stat(key) {
                self.local_pool.add_base(key, *value);
            } else {
                self.pool.add_base(key, *value);
            }
        }

        // 2. 处理词缀
        for affix in &item.affixes {
            // 检查词缀条件是否满足
            if !self.check_affix_requirements(affix) {
                continue;
            }

            for (key, value) in &affix.stats {
                if affix.is_local || is_local_stat(key) {
                    // 局部属性
                    Self::apply_stat_to_pool(&mut self.local_pool, key, *value);
                } else {
                    // 全局属性
                    self.apply_stat(key, *value);
                }
            }
        }
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
    fn apply_stat(&mut self, key: &str, value: f64) {
        Self::apply_stat_to_pool(&mut self.pool, key, value);
    }

    /// 应用属性到指定池
    fn apply_stat_to_pool(pool: &mut StatPool, key: &str, value: f64) {
        // 根据键名前缀判断类型
        if key.starts_with("mod.inc.") {
            pool.add_increased(&key.replace("mod.inc.", ""), value);
        } else if key.starts_with("mod.more.") {
            // More 修正默认使用 bucket 0
            pool.add_more(&key.replace("mod.more.", ""), value, 0, "item");
        } else {
            pool.add_base(key, value);
        }
    }

    /// 聚合技能属性
    pub fn aggregate_skill(&mut self, skill: &SkillData) {
        // 技能基础伤害
        for (key, value) in &skill.base_damage {
            self.pool.add_base(key, *value);
        }

        // 技能自带属性
        for (key, value) in &skill.stats {
            self.apply_stat(key, *value);
        }
    }

    /// 聚合辅助技能属性
    pub fn aggregate_support_skills(&mut self, supports: &[SkillData]) {
        for (idx, support) in supports.iter().enumerate() {
            for (key, value) in &support.stats {
                if key.starts_with("mod.more.") {
                    // 辅助技能的 More 使用独立 bucket
                    self.pool.add_more(
                        &key.replace("mod.more.", ""),
                        *value,
                        (idx + 100) as u32, // 辅助技能使用 100+ 的 bucket
                        &support.id,
                    );
                } else {
                    self.apply_stat(key, *value);
                }
            }
        }
    }

    /// 聚合全局覆盖
    pub fn aggregate_overrides(&mut self, overrides: &HashMap<String, f64>) {
        for (key, value) in overrides {
            self.apply_stat(key, *value);
        }
    }

    /// 应用局部属性到武器基础
    pub fn finalize_local_stats(&mut self) {
        // 武器物理伤害计算
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

        // 武器暴击率
        let base_crit = self.local_pool.get_base("crit.chance.local");
        if base_crit > 0.0 {
            self.pool.add_base("crit.chance", base_crit);
        }

        // 武器攻速（局部）
        let local_speed = self.local_pool.get_base("speed.attack.local");
        if local_speed > 0.0 {
            self.pool.set_base("weapon.base_speed", local_speed);
        }
    }

    /// 获取最终的属性池
    pub fn finalize(mut self) -> StatPool {
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
}

