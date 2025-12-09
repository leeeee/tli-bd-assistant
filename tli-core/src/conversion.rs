//! 伤害转化与额外获得模块
//!
//! 实现 Tag Retention（标签记忆）机制
//! - Phase A: Gain as Extra（额外获得）
//! - Phase B: Conversion（伤害转化）

use crate::tags::TagRegistry;
use crate::stats::StatPool;
use fixedbitset::FixedBitSet;
use std::collections::HashMap;

/// 伤害类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DamageType {
    Physical,
    Lightning,
    Cold,
    Fire,
    Chaos,
}

impl DamageType {
    /// 获取所有伤害类型（按转化优先级排序）
    pub fn all_ordered() -> &'static [DamageType] {
        &[
            DamageType::Physical,
            DamageType::Lightning,
            DamageType::Cold,
            DamageType::Fire,
            DamageType::Chaos,
        ]
    }

    /// 转换为字符串键
    pub fn as_key(&self) -> &'static str {
        match self {
            DamageType::Physical => "physical",
            DamageType::Lightning => "lightning",
            DamageType::Cold => "cold",
            DamageType::Fire => "fire",
            DamageType::Chaos => "chaos",
        }
    }

    /// 获取对应的标签名
    pub fn tag_name(&self) -> &'static str {
        match self {
            DamageType::Physical => "Tag_Physical",
            DamageType::Lightning => "Tag_Lightning",
            DamageType::Cold => "Tag_Cold",
            DamageType::Fire => "Tag_Fire",
            DamageType::Chaos => "Tag_Chaos",
        }
    }

    /// 从字符串解析
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "physical" | "phys" => Some(DamageType::Physical),
            "lightning" => Some(DamageType::Lightning),
            "cold" => Some(DamageType::Cold),
            "fire" => Some(DamageType::Fire),
            "chaos" => Some(DamageType::Chaos),
            _ => None,
        }
    }
}

/// 带历史标签的伤害值
#[derive(Debug, Clone)]
pub struct DamageWithTags {
    /// 伤害下限
    pub min: f64,
    /// 伤害上限
    pub max: f64,
    /// 历史标签集合（BitSet）
    pub history_tags: FixedBitSet,
}

impl DamageWithTags {
    pub fn new(min: f64, max: f64, tag_capacity: usize) -> Self {
        Self {
            min,
            max,
            history_tags: FixedBitSet::with_capacity(tag_capacity),
        }
    }

    pub fn zero(tag_capacity: usize) -> Self {
        Self::new(0.0, 0.0, tag_capacity)
    }

    pub fn average(&self) -> f64 {
        (self.min + self.max) / 2.0
    }

    pub fn is_zero(&self) -> bool {
        self.min == 0.0 && self.max == 0.0
    }

    /// 添加历史标签
    pub fn add_tag(&mut self, tag_id: u32) {
        self.history_tags.insert(tag_id as usize);
    }

    /// 合并另一个伤害值（保留所有历史标签）
    pub fn merge(&mut self, other: &DamageWithTags) {
        self.min += other.min;
        self.max += other.max;
        self.history_tags.union_with(&other.history_tags);
    }
}

/// 转化规则
#[derive(Debug, Clone)]
pub struct ConversionRule {
    pub from: DamageType,
    pub to: DamageType,
    pub percent: f64, // 0.0 - 1.0
}

/// 额外获得规则
#[derive(Debug, Clone)]
pub struct ExtraAsRule {
    pub from: DamageType,
    pub to: DamageType,
    pub percent: f64, // 0.0 - 1.0
}

/// 伤害转化引擎
pub struct ConversionEngine {
    tag_capacity: usize,
}

impl ConversionEngine {
    pub fn new(tag_capacity: usize) -> Self {
        Self { tag_capacity }
    }

    /// 执行完整的转化流程
    /// 
    /// 1. 初始化伤害池，添加原始标签
    /// 2. Phase A: Gain as Extra（不扣除原伤害）
    /// 3. Phase B: Conversion（扣除原伤害，按 DAG 顺序）
    pub fn process(
        &self,
        base_damages: &HashMap<DamageType, (f64, f64)>,
        extra_rules: &[ExtraAsRule],
        conversion_rules: &[ConversionRule],
        registry: &TagRegistry,
    ) -> HashMap<DamageType, DamageWithTags> {
        // 1. 初始化伤害池
        let mut pool: HashMap<DamageType, DamageWithTags> = HashMap::new();
        
        for (&dtype, &(min, max)) in base_damages {
            let mut dmg = DamageWithTags::new(min, max, self.tag_capacity);
            // 添加原始伤害类型标签
            if let Some(tag_id) = registry.get_id(dtype.tag_name()) {
                dmg.add_tag(tag_id);
            }
            pool.insert(dtype, dmg);
        }

        // 2. Phase A: Gain as Extra
        self.apply_extra_as(&mut pool, extra_rules, registry);

        // 3. Phase B: Conversion
        self.apply_conversion(&mut pool, conversion_rules, registry);

        pool
    }

    /// Phase A: 额外获得
    /// 计算"额外获得"逻辑，不扣除原伤害，产出的新伤害保留原伤害标签
    fn apply_extra_as(
        &self,
        pool: &mut HashMap<DamageType, DamageWithTags>,
        rules: &[ExtraAsRule],
        registry: &TagRegistry,
    ) {
        // 按目标类型收集额外伤害
        let mut extra_damages: HashMap<DamageType, DamageWithTags> = HashMap::new();

        for rule in rules {
            if let Some(source) = pool.get(&rule.from) {
                if source.is_zero() {
                    continue;
                }

                let extra_min = source.min * rule.percent;
                let extra_max = source.max * rule.percent;

                let mut extra_dmg = DamageWithTags::new(extra_min, extra_max, self.tag_capacity);
                // 继承源伤害的历史标签
                extra_dmg.history_tags.union_with(&source.history_tags);
                // 添加目标类型标签
                if let Some(tag_id) = registry.get_id(rule.to.tag_name()) {
                    extra_dmg.add_tag(tag_id);
                }

                extra_damages
                    .entry(rule.to)
                    .or_insert_with(|| DamageWithTags::zero(self.tag_capacity))
                    .merge(&extra_dmg);
            }
        }

        // 将额外伤害合并到主池
        for (dtype, extra) in extra_damages {
            pool.entry(dtype)
                .or_insert_with(|| DamageWithTags::zero(self.tag_capacity))
                .merge(&extra);
        }
    }

    /// Phase B: 伤害转化
    /// 按 DAG 顺序执行转化（Physical -> Lightning -> Cold -> Fire -> Chaos）
    /// 转化会扣除原伤害，产出的新伤害保留历史标签
    fn apply_conversion(
        &self,
        pool: &mut HashMap<DamageType, DamageWithTags>,
        rules: &[ConversionRule],
        registry: &TagRegistry,
    ) {
        // 按源类型分组转化规则
        let mut rules_by_source: HashMap<DamageType, Vec<&ConversionRule>> = HashMap::new();
        for rule in rules {
            rules_by_source
                .entry(rule.from)
                .or_insert_with(Vec::new)
                .push(rule);
        }

        // 按 DAG 顺序处理每种伤害类型
        for &source_type in DamageType::all_ordered() {
            let Some(source_rules) = rules_by_source.get(&source_type) else {
                continue;
            };

            // 计算该类型的总转化率（上限 100%）
            let total_percent: f64 = source_rules
                .iter()
                .map(|r| r.percent)
                .sum::<f64>()
                .min(1.0);

            if total_percent == 0.0 {
                continue;
            }

            // 获取源伤害（需要 clone，因为我们要修改池）
            let source = match pool.get(&source_type) {
                Some(s) if !s.is_zero() => s.clone(),
                _ => continue,
            };

            // 计算转化后的伤害分配
            for rule in source_rules {
                // 如果总转化率超过100%，按比例缩放
                let actual_percent = if total_percent > 1.0 {
                    rule.percent / total_percent
                } else {
                    rule.percent
                };

                let conv_min = source.min * actual_percent;
                let conv_max = source.max * actual_percent;

                let mut converted = DamageWithTags::new(conv_min, conv_max, self.tag_capacity);
                // 继承源伤害的历史标签（Tag Retention）
                converted.history_tags.union_with(&source.history_tags);
                // 添加目标类型标签
                if let Some(tag_id) = registry.get_id(rule.to.tag_name()) {
                    converted.add_tag(tag_id);
                }

                pool.entry(rule.to)
                    .or_insert_with(|| DamageWithTags::zero(self.tag_capacity))
                    .merge(&converted);
            }

            // 扣除源伤害
            if let Some(source_dmg) = pool.get_mut(&source_type) {
                let remaining = 1.0 - total_percent.min(1.0);
                source_dmg.min *= remaining;
                source_dmg.max *= remaining;
            }
        }
    }
}

/// 从属性池提取转化规则
pub fn extract_conversion_rules(pool: &StatPool) -> Vec<ConversionRule> {
    let mut rules = Vec::new();
    
    let conversions = [
        ("conv.phys_to_fire", DamageType::Physical, DamageType::Fire),
        ("conv.phys_to_cold", DamageType::Physical, DamageType::Cold),
        ("conv.phys_to_lightning", DamageType::Physical, DamageType::Lightning),
        ("conv.phys_to_chaos", DamageType::Physical, DamageType::Chaos),
        ("conv.lightning_to_cold", DamageType::Lightning, DamageType::Cold),
        ("conv.lightning_to_fire", DamageType::Lightning, DamageType::Fire),
        ("conv.cold_to_fire", DamageType::Cold, DamageType::Fire),
        ("conv.cold_to_chaos", DamageType::Cold, DamageType::Chaos),
        ("conv.fire_to_chaos", DamageType::Fire, DamageType::Chaos),
    ];
    
    for (key, from, to) in conversions {
        let percent = pool.get_base(key);
        if percent > 0.0 {
            rules.push(ConversionRule {
                from,
                to,
                percent: percent.min(1.0),
            });
        }
    }
    
    rules
}

/// 从属性池提取额外获得规则
pub fn extract_extra_as_rules(pool: &StatPool) -> Vec<ExtraAsRule> {
    let mut rules = Vec::new();
    
    let extras = [
        ("extra.phys_as_fire", DamageType::Physical, DamageType::Fire),
        ("extra.phys_as_cold", DamageType::Physical, DamageType::Cold),
        ("extra.phys_as_lightning", DamageType::Physical, DamageType::Lightning),
        ("extra.phys_as_chaos", DamageType::Physical, DamageType::Chaos),
        ("extra.lightning_as_cold", DamageType::Lightning, DamageType::Cold),
        ("extra.lightning_as_fire", DamageType::Lightning, DamageType::Fire),
        ("extra.cold_as_fire", DamageType::Cold, DamageType::Fire),
        ("extra.fire_as_chaos", DamageType::Fire, DamageType::Chaos),
    ];
    
    for (key, from, to) in extras {
        let percent = pool.get_base(key);
        if percent > 0.0 {
            rules.push(ExtraAsRule {
                from,
                to,
                percent,
            });
        }
    }
    
    rules
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_registry() -> TagRegistry {
        let mut registry = TagRegistry::new();
        registry.register("Tag_Physical".to_string(), 10);
        registry.register("Tag_Fire".to_string(), 21);
        registry.register("Tag_Cold".to_string(), 22);
        registry.register("Tag_Lightning".to_string(), 23);
        registry.register("Tag_Chaos".to_string(), 30);
        registry.precompute_expanded_sets();
        registry
    }

    #[test]
    fn test_extra_as() {
        let registry = create_test_registry();
        let engine = ConversionEngine::new(64);
        
        let mut base = HashMap::new();
        base.insert(DamageType::Physical, (100.0, 100.0));
        
        let extra_rules = vec![ExtraAsRule {
            from: DamageType::Physical,
            to: DamageType::Fire,
            percent: 0.20, // 20% 物理额外获得为火焰
        }];
        
        let result = engine.process(&base, &extra_rules, &[], &registry);
        
        // 物理不变：100
        let phys = result.get(&DamageType::Physical).unwrap();
        assert!((phys.average() - 100.0).abs() < 0.01);
        
        // 火焰：100 * 0.2 = 20
        let fire = result.get(&DamageType::Fire).unwrap();
        assert!((fire.average() - 20.0).abs() < 0.01);
        
        // 火焰伤害应该同时有物理和火焰标签
        assert!(fire.history_tags.contains(10)); // Physical
        assert!(fire.history_tags.contains(21)); // Fire
    }

    #[test]
    fn test_conversion_with_tag_retention() {
        let registry = create_test_registry();
        let engine = ConversionEngine::new(64);
        
        let mut base = HashMap::new();
        base.insert(DamageType::Physical, (100.0, 100.0));
        
        let conv_rules = vec![ConversionRule {
            from: DamageType::Physical,
            to: DamageType::Fire,
            percent: 0.50, // 50% 物理转火焰
        }];
        
        let result = engine.process(&base, &[], &conv_rules, &registry);
        
        // 物理剩余：100 * 0.5 = 50
        let phys = result.get(&DamageType::Physical).unwrap();
        assert!((phys.average() - 50.0).abs() < 0.01);
        
        // 火焰：100 * 0.5 = 50
        let fire = result.get(&DamageType::Fire).unwrap();
        assert!((fire.average() - 50.0).abs() < 0.01);
        
        // 火焰伤害应该保留物理标签（Tag Retention）
        assert!(fire.history_tags.contains(10)); // Physical
        assert!(fire.history_tags.contains(21)); // Fire
    }

    #[test]
    fn test_full_conversion_flow() {
        // 测试文档中的例子：
        // 100 物理基础伤 + 50% 转火焰 + 10% 物理Inc + 10% 火焰Inc
        let registry = create_test_registry();
        let engine = ConversionEngine::new(64);
        
        let mut base = HashMap::new();
        base.insert(DamageType::Physical, (100.0, 100.0));
        
        let conv_rules = vec![ConversionRule {
            from: DamageType::Physical,
            to: DamageType::Fire,
            percent: 0.50,
        }];
        
        let result = engine.process(&base, &[], &conv_rules, &registry);
        
        // 此时：
        // - 物理 50（历史标签：Physical）
        // - 火焰 50（历史标签：Physical + Fire）
        
        // 物理部分吃到 10% Physical Inc: 50 * 1.1 = 55
        // 火焰部分吃到 10% Physical Inc + 10% Fire Inc: 50 * 1.1 * 1.1 = 60.5
        // 总 DPS = 55 + 60.5 = 115.5
        
        let phys = result.get(&DamageType::Physical).unwrap();
        let fire = result.get(&DamageType::Fire).unwrap();
        
        assert!((phys.average() - 50.0).abs() < 0.01);
        assert!((fire.average() - 50.0).abs() < 0.01);
        
        // 验证标签保留
        assert!(fire.history_tags.contains(10)); // Physical 标签被保留
    }
}

