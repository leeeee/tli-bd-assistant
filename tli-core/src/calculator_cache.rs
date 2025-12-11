//! 计算缓存模块
//!
//! 借鉴 ZSim 的 MultiplierData 缓存机制，实现 LRU 缓存优化悬停预览性能。
//!
//! 核心思路：
//! - 使用哈希键唯一标识计算输入状态
//! - 缓存 StatPool 和计算结果，避免重复计算
//! - 悬停预览时，仅需增量计算差异部分

use crate::pipeline::{calculate_dps, CalculationError};
use crate::types::{CalculatorInput, CalculatorOutput};
use lru::LruCache;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::num::NonZeroUsize;

/// 缓存键
/// 
/// 基于输入状态生成唯一哈希，用于缓存查找
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct CacheKey {
    /// 装备状态哈希
    items_hash: u64,
    /// 技能配置哈希
    skill_hash: u64,
    /// 机制状态哈希
    mechanics_hash: u64,
    /// 目标配置哈希
    target_hash: u64,
    /// 全局覆盖哈希
    overrides_hash: u64,
}

impl CacheKey {
    /// 从计算输入生成缓存键
    pub fn from_input(input: &CalculatorInput) -> Self {
        let items_hash = Self::hash_items(&input.items);
        let skill_hash = Self::hash_skill(&input.active_skill, &input.support_skills);
        let mechanics_hash = Self::hash_mechanics(&input.mechanic_states);
        let target_hash = Self::hash_target(&input.target_config);
        let overrides_hash = Self::hash_overrides(&input.global_overrides);

        Self {
            items_hash,
            skill_hash,
            mechanics_hash,
            target_hash,
            overrides_hash,
        }
    }

    /// 生成仅排除特定槽位的缓存键（用于预览对比）
    pub fn without_slot(input: &CalculatorInput, slot: &crate::types::SlotType) -> Self {
        let mut hasher = DefaultHasher::new();
        for item in &input.items {
            if &item.slot != slot {
                item.id.hash(&mut hasher);
                // 哈希词缀数据
                for affix in &item.affixes {
                    affix.id.hash(&mut hasher);
                    for (k, v) in &affix.stats {
                        k.hash(&mut hasher);
                        v.to_bits().hash(&mut hasher);
                    }
                }
            }
        }
        let items_hash = hasher.finish();

        Self {
            items_hash,
            skill_hash: Self::hash_skill(&input.active_skill, &input.support_skills),
            mechanics_hash: Self::hash_mechanics(&input.mechanic_states),
            target_hash: Self::hash_target(&input.target_config),
            overrides_hash: Self::hash_overrides(&input.global_overrides),
        }
    }

    fn hash_items(items: &[crate::types::ItemData]) -> u64 {
        let mut hasher = DefaultHasher::new();
        for item in items {
            item.id.hash(&mut hasher);
            item.slot.hash(&mut hasher);
            // 哈希词缀数据
            for affix in &item.affixes {
                affix.id.hash(&mut hasher);
                for (k, v) in &affix.stats {
                    k.hash(&mut hasher);
                    v.to_bits().hash(&mut hasher);
                }
            }
            // 哈希基底属性
            for (k, v) in &item.base_implicit_stats {
                k.hash(&mut hasher);
                v.to_bits().hash(&mut hasher);
            }
            // 哈希暗金词缀属性
            for (k, v) in &item.implicit_stats {
                k.hash(&mut hasher);
                v.to_bits().hash(&mut hasher);
            }
        }
        hasher.finish()
    }

    fn hash_skill(
        active: &crate::types::SkillData,
        supports: &[crate::types::SkillData],
    ) -> u64 {
        let mut hasher = DefaultHasher::new();
        active.id.hash(&mut hasher);
        active.level.hash(&mut hasher);
        active.effectiveness.to_bits().hash(&mut hasher);
        for (k, v) in &active.base_damage {
            k.hash(&mut hasher);
            v.to_bits().hash(&mut hasher);
        }
        for support in supports {
            support.id.hash(&mut hasher);
            support.level.hash(&mut hasher);
        }
        hasher.finish()
    }

    fn hash_mechanics(states: &[crate::types::MechanicState]) -> u64 {
        let mut hasher = DefaultHasher::new();
        for state in states {
            state.id.hash(&mut hasher);
            state.current_stacks.hash(&mut hasher);
            state.is_active.hash(&mut hasher);
        }
        hasher.finish()
    }

    fn hash_target(target: &crate::types::TargetConfig) -> u64 {
        let mut hasher = DefaultHasher::new();
        target.level.hash(&mut hasher);
        target.armor.hash(&mut hasher);
        target.generic_dr.to_bits().hash(&mut hasher);
        for (k, v) in &target.resistances {
            k.hash(&mut hasher);
            v.to_bits().hash(&mut hasher);
        }
        hasher.finish()
    }

    fn hash_overrides(overrides: &std::collections::HashMap<String, f64>) -> u64 {
        let mut hasher = DefaultHasher::new();
        // 排序以确保一致性
        let mut pairs: Vec<_> = overrides.iter().collect();
        pairs.sort_by_key(|(k, _)| *k);
        for (k, v) in pairs {
            k.hash(&mut hasher);
            v.to_bits().hash(&mut hasher);
        }
        hasher.finish()
    }
}

/// 带缓存的计算器
///
/// 实现 LRU 缓存策略，优化悬停预览等高频计算场景
pub struct CachedCalculator {
    /// 计算结果缓存 (LRU, 默认最多 128 个)
    result_cache: LruCache<CacheKey, CalculatorOutput>,
    /// 缓存命中统计
    cache_hits: u64,
    /// 缓存未命中统计
    cache_misses: u64,
}

impl CachedCalculator {
    /// 创建新的缓存计算器
    ///
    /// # Arguments
    /// * `capacity` - 缓存容量，默认 128
    pub fn new(capacity: usize) -> Self {
        let cap = NonZeroUsize::new(capacity).unwrap_or(NonZeroUsize::new(128).unwrap());
        Self {
            result_cache: LruCache::new(cap),
            cache_hits: 0,
            cache_misses: 0,
        }
    }

    /// 带缓存的计算
    ///
    /// 如果缓存命中，直接返回缓存结果；否则执行完整计算并缓存
    pub fn calculate(&mut self, input: &CalculatorInput) -> Result<CalculatorOutput, CalculationError> {
        let cache_key = CacheKey::from_input(input);

        // 尝试从缓存获取
        if let Some(cached) = self.result_cache.get(&cache_key) {
            self.cache_hits += 1;
            return Ok(cached.clone());
        }

        // 缓存未命中，执行计算
        self.cache_misses += 1;
        let result = calculate_dps(input)?;

        // 存入缓存
        self.result_cache.put(cache_key, result.clone());

        Ok(result)
    }

    /// 计算预览差异
    ///
    /// 优化悬停预览场景：计算基准结果和预览结果，返回差异
    pub fn calculate_diff(
        &mut self,
        base_input: &CalculatorInput,
        preview_input: &CalculatorInput,
    ) -> Result<CalculationDiff, CalculationError> {
        let base_result = self.calculate(base_input)?;
        let preview_result = self.calculate(preview_input)?;

        Ok(CalculationDiff {
            base: base_result.clone(),
            preview: preview_result.clone(),
            dps_diff: preview_result.dps_theoretical - base_result.dps_theoretical,
            dps_diff_percent: if base_result.dps_theoretical > 0.0 {
                (preview_result.dps_theoretical - base_result.dps_theoretical) / base_result.dps_theoretical * 100.0
            } else {
                0.0
            },
            ehp_physical_diff: preview_result.ehp_series.physical - base_result.ehp_series.physical,
            crit_chance_diff: preview_result.crit_chance - base_result.crit_chance,
        })
    }

    /// 清空缓存
    pub fn clear_cache(&mut self) {
        self.result_cache.clear();
    }

    /// 获取缓存统计信息
    pub fn get_stats(&self) -> CacheStats {
        CacheStats {
            capacity: self.result_cache.cap().get(),
            size: self.result_cache.len(),
            hits: self.cache_hits,
            misses: self.cache_misses,
            hit_rate: if self.cache_hits + self.cache_misses > 0 {
                self.cache_hits as f64 / (self.cache_hits + self.cache_misses) as f64
            } else {
                0.0
            },
        }
    }

    /// 预热缓存
    ///
    /// 预先计算常见配置，填充缓存
    pub fn warmup(&mut self, inputs: &[CalculatorInput]) -> Result<(), CalculationError> {
        for input in inputs {
            self.calculate(input)?;
        }
        Ok(())
    }
}

impl Default for CachedCalculator {
    fn default() -> Self {
        Self::new(128)
    }
}

/// 计算差异结果
///
/// 用于悬停预览时显示装备更换的影响
#[derive(Debug, Clone)]
pub struct CalculationDiff {
    /// 基准计算结果
    pub base: CalculatorOutput,
    /// 预览计算结果
    pub preview: CalculatorOutput,
    /// DPS 差值
    pub dps_diff: f64,
    /// DPS 差值百分比
    pub dps_diff_percent: f64,
    /// 物理 EHP 差值
    pub ehp_physical_diff: f64,
    /// 暴击率差值
    pub crit_chance_diff: f64,
}

impl CalculationDiff {
    /// 是否为正收益
    pub fn is_positive(&self) -> bool {
        self.dps_diff > 0.0
    }

    /// 获取格式化的差异显示
    pub fn format_dps_diff(&self) -> String {
        if self.dps_diff > 0.0 {
            format!("+{:.0} ({:+.1}%)", self.dps_diff, self.dps_diff_percent)
        } else {
            format!("{:.0} ({:+.1}%)", self.dps_diff, self.dps_diff_percent)
        }
    }
}

/// 缓存统计信息
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// 缓存容量
    pub capacity: usize,
    /// 当前缓存大小
    pub size: usize,
    /// 缓存命中次数
    pub hits: u64,
    /// 缓存未命中次数
    pub misses: u64,
    /// 命中率
    pub hit_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;
    use std::collections::HashMap;

    fn create_test_input() -> CalculatorInput {
        CalculatorInput {
            context_flags: HashMap::new(),
            context_values: HashMap::new(),
            target_config: TargetConfig::default(),
            items: vec![],
            active_skill: SkillData {
                id: "test_skill".to_string(),
                skill_type: SkillType::Active,
                damage_type: Some("fire".to_string()),
                is_attack: false,
                level: 1,
                base_damage: [
                    ("dmg.fire.min".to_string(), 50.0),
                    ("dmg.fire.max".to_string(), 100.0),
                ]
                .into_iter()
                .collect(),
                base_time: 0.8,
                cooldown: None,
                mana_cost: 10,
                effectiveness: 1.0,
                tags: vec!["Tag_Spell".to_string(), "Tag_Fire".to_string()],
                stats: HashMap::new(),
                injected_tags: vec![],
                mana_multiplier: 1.0,
                level_data: None,
                scaling_rules: vec![],
            },
            support_skills: vec![],
            global_overrides: HashMap::new(),
            preview_slot: None,
            mechanic_states: vec![],
            mechanic_definitions: vec![],
        }
    }

    #[test]
    fn test_cache_hit() {
        let mut calculator = CachedCalculator::new(16);
        let input = create_test_input();

        // 第一次计算，应该是 miss
        let result1 = calculator.calculate(&input).unwrap();
        assert_eq!(calculator.cache_misses, 1);
        assert_eq!(calculator.cache_hits, 0);

        // 第二次相同输入，应该是 hit
        let result2 = calculator.calculate(&input).unwrap();
        assert_eq!(calculator.cache_misses, 1);
        assert_eq!(calculator.cache_hits, 1);

        // 结果应该相同
        assert_eq!(result1.dps_theoretical, result2.dps_theoretical);
    }

    #[test]
    fn test_cache_key_different_skill_level() {
        let mut calculator = CachedCalculator::new(16);
        let mut input1 = create_test_input();
        let mut input2 = create_test_input();

        input1.active_skill.level = 1;
        input2.active_skill.level = 10;

        // 不同等级应该产生不同的缓存键
        calculator.calculate(&input1).unwrap();
        calculator.calculate(&input2).unwrap();

        assert_eq!(calculator.cache_misses, 2);
        assert_eq!(calculator.cache_hits, 0);
    }

    #[test]
    fn test_calculate_diff() {
        let mut calculator = CachedCalculator::new(16);
        let base_input = create_test_input();
        
        let mut preview_input = create_test_input();
        preview_input.global_overrides.insert("mod.inc.dmg.fire".to_string(), 0.5); // +50% 火焰增伤

        let diff = calculator.calculate_diff(&base_input, &preview_input).unwrap();

        // 预览应该有更高的 DPS
        assert!(diff.dps_diff > 0.0);
        assert!(diff.is_positive());
    }

    #[test]
    fn test_cache_stats() {
        let mut calculator = CachedCalculator::new(16);
        let input = create_test_input();

        calculator.calculate(&input).unwrap();
        calculator.calculate(&input).unwrap();
        calculator.calculate(&input).unwrap();

        let stats = calculator.get_stats();
        assert_eq!(stats.hits, 2);
        assert_eq!(stats.misses, 1);
        assert!(stats.hit_rate > 0.6); // 2/3 ≈ 0.67
    }

    #[test]
    fn test_cache_clear() {
        let mut calculator = CachedCalculator::new(16);
        let input = create_test_input();

        calculator.calculate(&input).unwrap();
        assert_eq!(calculator.get_stats().size, 1);

        calculator.clear_cache();
        assert_eq!(calculator.get_stats().size, 0);
    }
}

