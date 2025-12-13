//! Modifier 存储与查询模块
//!
//! 对标 POB2 的 ModStore/ModDB/ModList 架构，提供统一的修正存储与查询抽象。
//!
//! ## 核心设计
//!
//! - **Modifier**: 单个修正的结构化表示，包含类型、值、来源、条件等
//! - **ModifierStore**: 统一查询接口 trait
//! - **ModDB**: 按 key 分桶的 HashMap 存储，查询 O(1)
//! - **ModList**: 扁平数组存储，适合临时计算
//!
//! ## 使用示例
//!
//! ```ignore
//! let mut db = ModDB::new();
//! db.add(Modifier::inc("dmg.fire", 0.5, "装备"));
//! db.add(Modifier::more("dmg.all", 0.2, "技能"));
//!
//! let fire_inc = db.sum_inc("dmg.fire"); // 0.5
//! let all_more = db.product_more("dmg.all"); // 1.2
//! ```

use crate::condition_ast::{Condition, EvalContext};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Modifier 类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModifierKind {
    /// 基础值（累加）
    Base,
    /// 增加/减少（累加后乘算）
    Increased,
    /// 更多/更少（独立相乘）
    More,
    /// 布尔标志
    Flag,
    /// 覆盖值（优先级最高）
    Override,
}

/// Modifier 作用域
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum ModifierScope {
    /// 全局（玩家）
    #[default]
    Global,
    /// 技能特定
    Skill,
    /// 召唤物
    Minion,
    /// 目标/敌人
    Target,
}

/// PerStat 配置（每 X 点属性提供效果）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerStatConfig {
    /// 属性键
    pub stat: String,
    /// 每多少点
    pub per: f64,
}

/// 单个修正
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Modifier {
    /// 属性键（如 "dmg.fire", "crit.chance"）
    pub key: String,
    /// 修正类型
    pub kind: ModifierKind,
    /// 修正值
    pub value: f64,
    /// 来源描述（用于溯源）
    pub source: String,
    /// Bucket ID（More 类型用于区分独立乘区）
    pub bucket_id: u32,
    /// 作用域
    pub scope: ModifierScope,
    /// 条件表达式（AST）
    #[serde(skip)]
    pub condition: Option<Condition>,
    /// 条件表达式字符串（用于序列化）
    pub condition_str: Option<String>,
    /// 标签要求（tag IDs）
    pub requirements: Vec<u32>,
    /// PerStat 配置（可选）
    pub per_stat: Option<PerStatConfig>,
}

impl Modifier {
    /// 创建基础值修正
    pub fn base(key: &str, value: f64, source: &str) -> Self {
        Self {
            key: key.to_string(),
            kind: ModifierKind::Base,
            value,
            source: source.to_string(),
            bucket_id: 0,
            scope: ModifierScope::Global,
            condition: None,
            condition_str: None,
            requirements: vec![],
            per_stat: None,
        }
    }

    /// 创建 Increased 修正
    pub fn inc(key: &str, value: f64, source: &str) -> Self {
        Self {
            key: key.to_string(),
            kind: ModifierKind::Increased,
            value,
            source: source.to_string(),
            bucket_id: 0,
            scope: ModifierScope::Global,
            condition: None,
            condition_str: None,
            requirements: vec![],
            per_stat: None,
        }
    }

    /// 创建 More 修正
    pub fn more(key: &str, value: f64, source: &str) -> Self {
        Self {
            key: key.to_string(),
            kind: ModifierKind::More,
            value,
            source: source.to_string(),
            bucket_id: 0,
            scope: ModifierScope::Global,
            condition: None,
            condition_str: None,
            requirements: vec![],
            per_stat: None,
        }
    }

    /// 创建带 bucket 的 More 修正
    pub fn more_with_bucket(key: &str, value: f64, bucket_id: u32, source: &str) -> Self {
        Self {
            key: key.to_string(),
            kind: ModifierKind::More,
            value,
            source: source.to_string(),
            bucket_id,
            scope: ModifierScope::Global,
            condition: None,
            condition_str: None,
            requirements: vec![],
            per_stat: None,
        }
    }

    /// 创建 Flag 修正
    pub fn flag(key: &str, source: &str) -> Self {
        Self {
            key: key.to_string(),
            kind: ModifierKind::Flag,
            value: 1.0,
            source: source.to_string(),
            bucket_id: 0,
            scope: ModifierScope::Global,
            condition: None,
            condition_str: None,
            requirements: vec![],
            per_stat: None,
        }
    }

    /// 创建 Override 修正
    pub fn override_value(key: &str, value: f64, source: &str) -> Self {
        Self {
            key: key.to_string(),
            kind: ModifierKind::Override,
            value,
            source: source.to_string(),
            bucket_id: 0,
            scope: ModifierScope::Global,
            condition: None,
            condition_str: None,
            requirements: vec![],
            per_stat: None,
        }
    }

    /// 设置条件（字符串形式，自动解析为 AST）
    pub fn with_condition(mut self, condition_str: &str) -> Self {
        self.condition_str = Some(condition_str.to_string());
        self.condition = Condition::parse(condition_str).ok();
        self
    }

    /// 设置条件（AST 形式）
    pub fn with_condition_ast(mut self, condition: Condition) -> Self {
        self.condition = Some(condition);
        self
    }

    /// 设置标签要求
    pub fn with_requirements(mut self, requirements: Vec<u32>) -> Self {
        self.requirements = requirements;
        self
    }

    /// 设置作用域
    pub fn with_scope(mut self, scope: ModifierScope) -> Self {
        self.scope = scope;
        self
    }

    /// 设置 PerStat 配置
    pub fn with_per_stat(mut self, stat: &str, per: f64) -> Self {
        self.per_stat = Some(PerStatConfig {
            stat: stat.to_string(),
            per,
        });
        self
    }

    /// 检查条件是否满足
    pub fn check_condition(&self, ctx: &EvalContext) -> bool {
        match &self.condition {
            Some(cond) => cond.evaluate(ctx),
            None => true, // 无条件则始终生效
        }
    }

    /// 计算实际生效值（考虑 PerStat）
    pub fn effective_value(&self, ctx: &EvalContext) -> f64 {
        let base_value = self.value;
        
        match &self.per_stat {
            Some(ps) => {
                let stat_value = ctx.values.get(&ps.stat).copied().unwrap_or(0.0);
                let multiplier = (stat_value / ps.per).floor();
                base_value * multiplier
            }
            None => base_value,
        }
    }
}

/// Modifier 存储查询接口
pub trait ModifierStore {
    /// 添加修正
    fn add(&mut self, modifier: Modifier);

    /// 批量添加修正
    fn add_all(&mut self, modifiers: Vec<Modifier>) {
        for m in modifiers {
            self.add(m);
        }
    }

    /// 获取指定 key 的所有修正
    fn get(&self, key: &str) -> Vec<&Modifier>;

    /// 获取指定 key 和类型的修正
    fn get_by_kind(&self, key: &str, kind: ModifierKind) -> Vec<&Modifier>;

    /// 计算 Base 值总和
    fn sum_base(&self, key: &str) -> f64 {
        self.get_by_kind(key, ModifierKind::Base)
            .iter()
            .map(|m| m.value)
            .sum()
    }

    /// 计算 Base 值总和（带条件评估）
    fn sum_base_with_ctx(&self, key: &str, ctx: &EvalContext) -> f64 {
        self.get_by_kind(key, ModifierKind::Base)
            .iter()
            .filter(|m| m.check_condition(ctx))
            .map(|m| m.effective_value(ctx))
            .sum()
    }

    /// 计算 Increased 值总和
    fn sum_inc(&self, key: &str) -> f64 {
        self.get_by_kind(key, ModifierKind::Increased)
            .iter()
            .map(|m| m.value)
            .sum()
    }

    /// 计算 Increased 值总和（带条件评估）
    fn sum_inc_with_ctx(&self, key: &str, ctx: &EvalContext) -> f64 {
        self.get_by_kind(key, ModifierKind::Increased)
            .iter()
            .filter(|m| m.check_condition(ctx))
            .map(|m| m.effective_value(ctx))
            .sum()
    }

    /// 计算 More 乘积（按 bucket 分组）
    fn product_more(&self, key: &str) -> f64 {
        let mods = self.get_by_kind(key, ModifierKind::More);
        if mods.is_empty() {
            return 1.0;
        }

        // 按 bucket_id 分组
        let mut buckets: HashMap<u32, f64> = HashMap::new();
        for m in mods {
            let entry = buckets.entry(m.bucket_id).or_insert(1.0);
            *entry *= 1.0 + m.value;
        }

        // 所有 bucket 相乘
        buckets.values().product()
    }

    /// 计算 More 乘积（带条件评估，按 bucket 分组）
    fn product_more_with_ctx(&self, key: &str, ctx: &EvalContext) -> f64 {
        let mods: Vec<_> = self
            .get_by_kind(key, ModifierKind::More)
            .into_iter()
            .filter(|m| m.check_condition(ctx))
            .collect();

        if mods.is_empty() {
            return 1.0;
        }

        // 按 bucket_id 分组
        let mut buckets: HashMap<u32, f64> = HashMap::new();
        for m in mods {
            let entry = buckets.entry(m.bucket_id).or_insert(1.0);
            *entry *= 1.0 + m.effective_value(ctx);
        }

        // 所有 bucket 相乘
        buckets.values().product()
    }

    /// 检查 Flag 是否存在
    fn has_flag(&self, key: &str) -> bool {
        !self.get_by_kind(key, ModifierKind::Flag).is_empty()
    }

    /// 检查 Flag 是否存在（带条件评估）
    fn has_flag_with_ctx(&self, key: &str, ctx: &EvalContext) -> bool {
        self.get_by_kind(key, ModifierKind::Flag)
            .iter()
            .any(|m| m.check_condition(ctx))
    }

    /// 获取 Override 值（取最后一个）
    fn get_override(&self, key: &str) -> Option<f64> {
        self.get_by_kind(key, ModifierKind::Override)
            .last()
            .map(|m| m.value)
    }

    /// 获取 Override 值（带条件评估）
    fn get_override_with_ctx(&self, key: &str, ctx: &EvalContext) -> Option<f64> {
        self.get_by_kind(key, ModifierKind::Override)
            .iter()
            .filter(|m| m.check_condition(ctx))
            .last()
            .map(|m| m.value)
    }

    /// 计算最终值：override > base * (1 + inc) * more
    fn calculate_final(&self, key: &str) -> f64 {
        if let Some(override_val) = self.get_override(key) {
            return override_val;
        }

        let base = self.sum_base(key);
        let inc = self.sum_inc(key);
        let more = self.product_more(key);

        base * (1.0 + inc) * more
    }

    /// 计算最终值（带条件评估）
    fn calculate_final_with_ctx(&self, key: &str, ctx: &EvalContext) -> f64 {
        if let Some(override_val) = self.get_override_with_ctx(key, ctx) {
            return override_val;
        }

        let base = self.sum_base_with_ctx(key, ctx);
        let inc = self.sum_inc_with_ctx(key, ctx);
        let more = self.product_more_with_ctx(key, ctx);

        base * (1.0 + inc) * more
    }

    /// 获取所有唯一的 key
    fn keys(&self) -> Vec<String>;

    /// 获取所有修正（用于溯源）
    fn all_modifiers(&self) -> Vec<&Modifier>;

    /// 获取修正来源列表（用于 UI 展示）
    fn get_sources(&self, key: &str) -> Vec<ModifierSource> {
        self.get(key)
            .iter()
            .map(|m| ModifierSource {
                kind: m.kind,
                value: m.value,
                source: m.source.clone(),
                bucket_id: m.bucket_id,
            })
            .collect()
    }

    /// 获取满足条件的修正来源列表
    fn get_sources_with_ctx(&self, key: &str, ctx: &EvalContext) -> Vec<ModifierSource> {
        self.get(key)
            .iter()
            .filter(|m| m.check_condition(ctx))
            .map(|m| ModifierSource {
                kind: m.kind,
                value: m.effective_value(ctx),
                source: m.source.clone(),
                bucket_id: m.bucket_id,
            })
            .collect()
    }
}

/// 修正来源信息（用于 UI 展示）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModifierSource {
    pub kind: ModifierKind,
    pub value: f64,
    pub source: String,
    pub bucket_id: u32,
}

/// ModDB - 按 key 分桶的 HashMap 存储
///
/// 适合大规模数据，查询 O(1)
#[derive(Debug, Clone, Default)]
pub struct ModDB {
    /// 按 key 分桶存储
    data: HashMap<String, Vec<Modifier>>,
}

impl ModDB {
    /// 创建空的 ModDB
    pub fn new() -> Self {
        Self::default()
    }

    /// 合并另一个 ModDB
    pub fn merge(&mut self, other: &ModDB) {
        for (key, mods) in &other.data {
            let entry = self.data.entry(key.clone()).or_default();
            entry.extend(mods.iter().cloned());
        }
    }

    /// 获取修正数量
    pub fn len(&self) -> usize {
        self.data.values().map(|v| v.len()).sum()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

impl ModifierStore for ModDB {
    fn add(&mut self, modifier: Modifier) {
        self.data
            .entry(modifier.key.clone())
            .or_default()
            .push(modifier);
    }

    fn get(&self, key: &str) -> Vec<&Modifier> {
        self.data
            .get(key)
            .map(|v| v.iter().collect())
            .unwrap_or_default()
    }

    fn get_by_kind(&self, key: &str, kind: ModifierKind) -> Vec<&Modifier> {
        self.data
            .get(key)
            .map(|v| v.iter().filter(|m| m.kind == kind).collect())
            .unwrap_or_default()
    }

    fn keys(&self) -> Vec<String> {
        self.data.keys().cloned().collect()
    }

    fn all_modifiers(&self) -> Vec<&Modifier> {
        self.data.values().flatten().collect()
    }
}

/// ModList - 扁平数组存储
///
/// 适合临时计算和小规模数据
#[derive(Debug, Clone, Default)]
pub struct ModList {
    /// 扁平存储
    data: Vec<Modifier>,
}

impl ModList {
    /// 创建空的 ModList
    pub fn new() -> Self {
        Self::default()
    }

    /// 从 Vec 创建
    pub fn from_vec(data: Vec<Modifier>) -> Self {
        Self { data }
    }

    /// 获取修正数量
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// 转换为 ModDB
    pub fn to_mod_db(&self) -> ModDB {
        let mut db = ModDB::new();
        for m in &self.data {
            db.add(m.clone());
        }
        db
    }
}

impl ModifierStore for ModList {
    fn add(&mut self, modifier: Modifier) {
        self.data.push(modifier);
    }

    fn get(&self, key: &str) -> Vec<&Modifier> {
        self.data.iter().filter(|m| m.key == key).collect()
    }

    fn get_by_kind(&self, key: &str, kind: ModifierKind) -> Vec<&Modifier> {
        self.data
            .iter()
            .filter(|m| m.key == key && m.kind == kind)
            .collect()
    }

    fn keys(&self) -> Vec<String> {
        let mut keys: Vec<String> = self.data.iter().map(|m| m.key.clone()).collect();
        keys.sort();
        keys.dedup();
        keys
    }

    fn all_modifiers(&self) -> Vec<&Modifier> {
        self.data.iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mod_db_basic() {
        let mut db = ModDB::new();

        // 添加修正
        db.add(Modifier::base("dmg.fire", 100.0, "技能"));
        db.add(Modifier::inc("dmg.fire", 0.5, "装备1"));
        db.add(Modifier::inc("dmg.fire", 0.3, "装备2"));
        db.add(Modifier::more("dmg.fire", 0.2, "辅助技能"));

        // 验证查询
        assert!((db.sum_base("dmg.fire") - 100.0).abs() < 0.001);
        assert!((db.sum_inc("dmg.fire") - 0.8).abs() < 0.001);
        assert!((db.product_more("dmg.fire") - 1.2).abs() < 0.001);

        // 验证最终值: 100 * (1 + 0.8) * 1.2 = 216
        assert!((db.calculate_final("dmg.fire") - 216.0).abs() < 0.001);
    }

    #[test]
    fn test_mod_db_more_buckets() {
        let mut db = ModDB::new();

        // 添加不同 bucket 的 More
        db.add(Modifier::more_with_bucket("dmg.all", 0.2, 0, "技能"));
        db.add(Modifier::more_with_bucket("dmg.all", 0.3, 1, "辅助1"));
        db.add(Modifier::more_with_bucket("dmg.all", 0.1, 1, "辅助2")); // 同 bucket

        // bucket 0: 1.2
        // bucket 1: (1.3) * (1.1) = 1.43
        // 总乘积: 1.2 * 1.43 = 1.716
        let more = db.product_more("dmg.all");
        assert!((more - 1.716).abs() < 0.001);
    }

    #[test]
    fn test_mod_db_flag() {
        let mut db = ModDB::new();

        assert!(!db.has_flag("cannot_crit"));

        db.add(Modifier::flag("cannot_crit", "诅咒"));

        assert!(db.has_flag("cannot_crit"));
    }

    #[test]
    fn test_mod_db_override() {
        let mut db = ModDB::new();

        db.add(Modifier::base("crit.chance", 0.05, "基础"));
        db.add(Modifier::inc("crit.chance", 1.0, "装备"));
        db.add(Modifier::override_value("crit.chance", 0.5, "天赋覆盖"));

        // Override 优先
        assert!((db.calculate_final("crit.chance") - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_mod_list_basic() {
        let mut list = ModList::new();

        list.add(Modifier::base("dmg.cold", 50.0, "技能"));
        list.add(Modifier::inc("dmg.cold", 0.4, "装备"));
        list.add(Modifier::more("dmg.cold", 0.25, "辅助"));

        // 50 * (1 + 0.4) * 1.25 = 87.5
        assert!((list.calculate_final("dmg.cold") - 87.5).abs() < 0.001);
    }

    #[test]
    fn test_mod_list_to_db() {
        let mut list = ModList::new();
        list.add(Modifier::base("test", 10.0, "source"));
        list.add(Modifier::inc("test", 0.5, "source"));

        let db = list.to_mod_db();
        assert!((db.calculate_final("test") - 15.0).abs() < 0.001);
    }

    #[test]
    fn test_modifier_sources() {
        let mut db = ModDB::new();

        db.add(Modifier::inc("dmg.fire", 0.2, "头盔"));
        db.add(Modifier::inc("dmg.fire", 0.3, "手套"));
        db.add(Modifier::more("dmg.fire", 0.1, "辅助技能"));

        let sources = db.get_sources("dmg.fire");
        assert_eq!(sources.len(), 3);

        // 验证来源信息
        let inc_sources: Vec<_> = sources
            .iter()
            .filter(|s| s.kind == ModifierKind::Increased)
            .collect();
        assert_eq!(inc_sources.len(), 2);
    }

    #[test]
    fn test_mod_db_merge() {
        let mut db1 = ModDB::new();
        db1.add(Modifier::base("dmg.fire", 50.0, "db1"));

        let mut db2 = ModDB::new();
        db2.add(Modifier::base("dmg.fire", 30.0, "db2"));
        db2.add(Modifier::inc("dmg.cold", 0.2, "db2"));

        db1.merge(&db2);

        assert!((db1.sum_base("dmg.fire") - 80.0).abs() < 0.001);
        assert!((db1.sum_inc("dmg.cold") - 0.2).abs() < 0.001);
    }

    #[test]
    fn test_mod_db_with_condition() {
        let mut db = ModDB::new();

        // 无条件增伤
        db.add(Modifier::inc("dmg.fire", 0.2, "装备"));
        // 移动时生效的增伤
        db.add(Modifier::inc("dmg.fire", 0.3, "移动增伤").with_condition("is_moving == true"));
        // 高战意时生效的增伤（战意 >= 50）
        db.add(Modifier::inc("dmg.fire", 0.5, "战意增伤").with_condition("fighting_will >= 50"));

        // 无条件时只有基础增伤
        let ctx_none = EvalContext::new();
        let sum_none = db.sum_inc_with_ctx("dmg.fire", &ctx_none);
        assert!((sum_none - 0.2).abs() < 0.001);

        // 移动时
        let ctx_moving = EvalContext::new().with_flag("is_moving", true);
        assert!((db.sum_inc_with_ctx("dmg.fire", &ctx_moving) - 0.5).abs() < 0.001); // 0.2 + 0.3

        // 高战意时（60 >= 50）
        let ctx_high_will = EvalContext::new().with_value("fighting_will", 60.0);
        assert!((db.sum_inc_with_ctx("dmg.fire", &ctx_high_will) - 0.7).abs() < 0.001); // 0.2 + 0.5

        // 移动且高战意
        let ctx_both = EvalContext::new()
            .with_flag("is_moving", true)
            .with_value("fighting_will", 100.0);
        assert!((db.sum_inc_with_ctx("dmg.fire", &ctx_both) - 1.0).abs() < 0.001); // 0.2 + 0.3 + 0.5
    }

    #[test]
    fn test_mod_db_with_per_stat() {
        let mut db = ModDB::new();

        // 每 10 点敏捷 +1% 火伤
        db.add(Modifier::inc("dmg.fire", 0.01, "每点敏捷").with_per_stat("dexterity", 10.0));

        // 250 敏捷 → 25 次 → 0.25
        let ctx = EvalContext::new().with_value("dexterity", 250.0);
        assert!((db.sum_inc_with_ctx("dmg.fire", &ctx) - 0.25).abs() < 0.001);

        // 95 敏捷 → 9 次 → 0.09
        let ctx2 = EvalContext::new().with_value("dexterity", 95.0);
        assert!((db.sum_inc_with_ctx("dmg.fire", &ctx2) - 0.09).abs() < 0.001);
    }

    #[test]
    fn test_mod_db_with_mechanic_condition() {
        let mut db = ModDB::new();

        // 有聚能祝福时生效
        db.add(Modifier::more("dmg.cold", 0.2, "祝福加成").with_condition("mechanic_active(\"focus_blessing\")"));
        // 战意 >= 50 时生效
        db.add(Modifier::more("dmg.cold", 0.1, "战意加成").with_condition("mechanic_stacks(\"fighting_will\") >= 50"));

        // 无机制
        let ctx_none = EvalContext::new();
        assert!((db.product_more_with_ctx("dmg.cold", &ctx_none) - 1.0).abs() < 0.001);

        // 有祝福
        let ctx_blessing = EvalContext::new().with_mechanic("focus_blessing", 3);
        assert!((db.product_more_with_ctx("dmg.cold", &ctx_blessing) - 1.2).abs() < 0.001);

        // 战意 100
        let ctx_will = EvalContext::new().with_mechanic("fighting_will", 100);
        assert!((db.product_more_with_ctx("dmg.cold", &ctx_will) - 1.1).abs() < 0.001);

        // 两者都有
        let ctx_both = EvalContext::new()
            .with_mechanic("focus_blessing", 3)
            .with_mechanic("fighting_will", 100);
        assert!((db.product_more_with_ctx("dmg.cold", &ctx_both) - 1.32).abs() < 0.001); // 1.2 * 1.1
    }
}

