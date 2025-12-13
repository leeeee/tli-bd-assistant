//! 计算管线模块
//!
//! 实现完整的 DPS/EHP 计算流程：
//! 1. Sanitization & Slot Conflict
//! 2. Mechanics Processing (祝福、球类等)
//! 3. Stat Pool Aggregation
//! 4. Base Calculation
//! 5. Extra & Conversion
//! 6. Modification (Inc/More)
//! 7. Speed Layer
//! 8. Crit & Luck
//! 9. Mitigation & Output
//!
//! ## 增量计算支持
//!
//! 通过 `PreparedContext` 缓存中间结果，支持悬停预览场景的增量计算：
//! - `prepare_context()`: 准备阶段，生成可复用的中间结果
//! - `calculate_from_prepared()`: 从 PreparedContext 计算最终结果
//! - `calculate_diff_incremental()`: 增量计算预览差异

use crate::conversion::{
    extract_conversion_rules, extract_extra_as_rules, ConversionEngine, DamageType, DamageWithTags,
};
use crate::mechanics::MechanicsProcessor;
use crate::modifiers::ModDB;
use crate::stats::{StatAggregator, StatPool};
use crate::tags::{ContextTags, TagRegistry};
use crate::types::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// 计算错误类型
#[derive(Debug, Error)]
pub enum CalculationError {
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    #[error("Tag registry error: {0}")]
    TagRegistryError(String),
    #[error("Calculation error: {0}")]
    CalculationError(String),
}

/// 预处理上下文（用于缓存中间结果）
///
/// 包含聚合阶段产出的所有中间数据，可被复用于增量计算。
#[derive(Debug, Clone)]
pub struct PreparedContext {
    /// 标签注册表
    pub registry: TagRegistry,
    /// 属性池
    pub stat_pool: StatPool,
    /// 结构化修正存储
    pub mod_db: ModDB,
    /// 基础伤害（按伤害类型分组）
    pub base_damages: HashMap<DamageType, (f64, f64)>,
    /// 技能数据快照
    pub skill_snapshot: SkillSnapshot,
    /// 机制状态快照（层数）
    pub mechanic_stacks: HashMap<String, f64>,
    /// 上下文标志
    pub context_flags: HashMap<String, bool>,
    /// 上下文数值
    pub context_values: HashMap<String, f64>,
    /// 转化规则
    pub conversion_rules: Vec<crate::conversion::ConversionRule>,
    /// Extra-as 规则
    pub extra_as_rules: Vec<crate::conversion::ExtraAsRule>,
    /// 调试追踪
    pub trace: Vec<TraceEntry>,
}

/// 技能数据快照（用于缓存）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillSnapshot {
    pub id: String,
    pub is_attack: bool,
    pub base_time: f64,
    pub effectiveness: f64,
    pub tags: Vec<String>,
}

impl PreparedContext {
    /// 合并另一个 PreparedContext 的 ModDB（用于增量计算）
    ///
    /// 用于悬停预览场景：复用 base 的 PreparedContext，仅合并 preview item 产生的 modifiers
    pub fn merge_modifiers(&mut self, other_mod_db: &ModDB) {
        self.mod_db.merge(other_mod_db);
        // 重新计算 stat_pool（从合并后的 mod_db 重建）
        self.rebuild_stat_pool_from_mod_db();
    }

    /// 从 ModDB 重建 StatPool
    fn rebuild_stat_pool_from_mod_db(&mut self) {
        use crate::modifiers::{ModifierKind, ModifierStore};

        // 清空现有 StatPool
        self.stat_pool = StatPool::new();

        // 遍历 ModDB 中的所有修正，重建 StatPool
        for modifier in self.mod_db.all_modifiers() {
            match modifier.kind {
                ModifierKind::Base => {
                    self.stat_pool.add_base(&modifier.key, modifier.value);
                }
                ModifierKind::Increased => {
                    self.stat_pool.add_increased(&modifier.key, modifier.value);
                }
                ModifierKind::More => {
                    self.stat_pool.add_more(
                        &modifier.key,
                        modifier.value,
                        modifier.bucket_id,
                        &modifier.source,
                    );
                }
                _ => {}
            }
        }

        self.stat_pool.recalculate_all();
    }
}

/// 主计算函数
pub fn calculate_dps(input: &CalculatorInput) -> Result<CalculatorOutput, CalculationError> {
    let mut trace = Vec::new();

    // 0. 初始化标签注册表（实际应从数据库加载）
    let registry = create_default_registry();

    // 1. Sanitization & Slot Conflict
    let sanitized_items = sanitize_items(&input.items, &input.preview_slot)?;
    trace.push(TraceEntry {
        phase: "Sanitization".to_string(),
        description: format!("Processed {} items", sanitized_items.len()),
        values: HashMap::new(),
        matched_tags: vec![],
    });

    // 2. 建立上下文标签
    let mut context = ContextTags::new(registry.clone());
    context.inject_skill_tags(&input.active_skill.tags);
    for support in &input.support_skills {
        context.inject_support_tags(&support.injected_tags);
    }
    context.inject_context_flags(&input.context_flags);

    // 2.5 初始化机制处理器（祝福、球类等）
    let mechanics = MechanicsProcessor::new(
        input.mechanic_definitions.clone(),
        input.mechanic_states.clone(),
    );
    
    // 记录机制状态到 trace
    if !input.mechanic_states.is_empty() {
        let active_mechanics: Vec<String> = input.mechanic_states
            .iter()
            .filter(|s| s.is_active && s.current_stacks > 0)
            .map(|s| format!("{}({}层)", s.id, s.current_stacks))
            .collect();
        
        if !active_mechanics.is_empty() {
            trace.push(TraceEntry {
                phase: "Mechanics".to_string(),
                description: format!("激活机制: {}", active_mechanics.join(", ")),
                values: mechanics.get_all_stacks(),
                matched_tags: vec![],
            });
        }
    }

    // 3. Stat Pool Aggregation（带机制处理器）
    let mut aggregator = StatAggregator::with_mechanics(&context, &mechanics);
    aggregator.aggregate_items(&sanitized_items);
    aggregator.aggregate_skill(&input.active_skill);
    aggregator.aggregate_support_skills(&input.support_skills);
    aggregator.aggregate_overrides(&input.global_overrides);
    
    // 3.5 应用机制基础效果（如聚能祝福每层+4%伤害）
    aggregator.apply_mechanic_base_effects();
    
    // 获取 StatPool 和 ModDB（ModDB 用于溯源，当前暂未使用）
    let (stat_pool, _mod_db) = aggregator.finalize();

    // 4. Base Calculation
    let base_damages = calculate_base_damage(&stat_pool, &input.active_skill);
    trace.push(TraceEntry {
        phase: "Base Damage".to_string(),
        description: "Calculated base damage values".to_string(),
        values: base_damages
            .iter()
            .map(|(k, (min, max))| (k.as_key().to_string(), (*min + *max) / 2.0))
            .collect(),
        matched_tags: vec![],
    });

    // 5. Extra & Conversion (with Tag Retention)
    let extra_rules = extract_extra_as_rules(&stat_pool);
    let conv_rules = extract_conversion_rules(&stat_pool);
    let engine = ConversionEngine::new((registry.max_id() + 1) as usize);
    let damage_pool = engine.process(&base_damages, &extra_rules, &conv_rules, &registry);

    // 6. Modification (Inc/More) - 按标签应用
    let modified_damages = apply_modifications(&damage_pool, &stat_pool, &context);
    
    // Lucky 处理：flag.lucky 或 context_flags.lucky_damage
    let is_lucky = stat_pool.get_base("flag.lucky") > 0.0
        || input.context_flags.get("lucky_damage").copied().unwrap_or(false);
    
    let total_damage: f64 = modified_damages
        .values()
        .map(|d| expected_damage(d.min, d.max, is_lucky))
        .sum();
    trace.push(TraceEntry {
        phase: "Modification".to_string(),
        description: "Applied Inc/More modifiers".to_string(),
        values: modified_damages
            .iter()
            .map(|(k, v)| (k.as_key().to_string(), v.average()))
            .collect(),
        matched_tags: vec![],
    });

    // 7. Speed Layer
    let rate = calculate_rate(&stat_pool, &input.active_skill);
    trace.push(TraceEntry {
        phase: "Speed".to_string(),
        description: format!("Attack/Cast rate: {:.2}/s", rate),
        values: [("rate".to_string(), rate)].into_iter().collect(),
        matched_tags: vec![],
    });

    // 8. Crit & Luck
    let (crit_chance, crit_multiplier) = calculate_crit(&stat_pool, &input.context_flags);
    let crit_factor = calculate_crit_factor(crit_chance, crit_multiplier);
    
    let hit_damage = total_damage * crit_factor;
    trace.push(TraceEntry {
        phase: "Critical".to_string(),
        description: format!("Crit: {:.1}% chance, {:.1}% multi", crit_chance * 100.0, crit_multiplier * 100.0),
        values: [
            ("crit_chance".to_string(), crit_chance),
            ("crit_multiplier".to_string(), crit_multiplier),
            ("crit_factor".to_string(), crit_factor),
        ]
        .into_iter()
        .collect(),
        matched_tags: vec![],
    });

    // 9. Mitigation (Hit Chance & Enemy DR)
    let hit_chance = calculate_hit_chance(&stat_pool, &input.target_config);
    let dps_theoretical = hit_damage * rate;
    let dps_effective = calculate_effective_dps(
        &modified_damages,
        rate,
        crit_factor,
        hit_chance,
        &input.target_config,
    );

    // 10. EHP Calculation
    let ehp_series = calculate_ehp(&stat_pool);

    // 11. Build damage breakdown (带乘区明细，使用 ModDB 提供详细来源)
    let damage_breakdown = build_damage_breakdown(
        &base_damages,
        &modified_damages,
        &stat_pool,
        Some(&_mod_db),
        rate,
        crit_chance,
        crit_multiplier,
        hit_chance,
        &input.target_config,
        is_lucky,
    );

    Ok(CalculatorOutput {
        dps_theoretical,
        dps_effective,
        hit_damage,
        rate,
        crit_chance,
        crit_multiplier,
        hit_chance,
        ehp_series,
        damage_breakdown,
        debug_trace: trace,
    })
}

/// 标签注册表 JSON 内容（编译时内嵌）
/// 
/// 数据来源：src/data/tags_registry.json
/// 注意：如需修改标签定义，请编辑上述 JSON 文件
const TAGS_REGISTRY_JSON: &str = include_str!("data/tags_registry.json");

/// 创建默认的标签注册表
/// 
/// 从内嵌的 JSON 配置加载标签定义，实现数据与代码分离。
/// 如果 JSON 解析失败，将回退到最小硬编码定义。
fn create_default_registry() -> TagRegistry {
    match TagRegistry::from_json(TAGS_REGISTRY_JSON) {
        Ok(registry) => registry,
        Err(_e) => {
            // 解析失败时使用最小回退定义
            // 注意：在生产环境中应记录此错误
            #[cfg(debug_assertions)]
            eprintln!("Warning: Failed to load tags from JSON: {}, using fallback", _e);
            
            create_fallback_registry()
        }
    }
}

/// 创建最小回退标签注册表（仅在 JSON 加载失败时使用）
fn create_fallback_registry() -> TagRegistry {
    let mut registry = TagRegistry::new();

    // 最小必需标签定义
    registry.register("Tag_Damage".to_string(), 1);
    registry.register("Tag_Physical".to_string(), 10);
    registry.register("Tag_Elemental".to_string(), 20);
    registry.register("Tag_Fire".to_string(), 21);
    registry.register("Tag_Cold".to_string(), 22);
    registry.register("Tag_Lightning".to_string(), 23);
    registry.register("Tag_Chaos".to_string(), 30);
    registry.register("Tag_Attack".to_string(), 100);
    registry.register("Tag_Melee".to_string(), 101);
    registry.register("Tag_Ranged".to_string(), 102);
    registry.register("Tag_Spell".to_string(), 110);
    registry.register("Tag_AOE".to_string(), 120);
    registry.register("Tag_Projectile".to_string(), 103);
    registry.register("Tag_DOT".to_string(), 130);

    // 设置继承关系
    registry.set_parents(10, vec![1]);
    registry.set_parents(20, vec![1]);
    registry.set_parents(21, vec![20]);
    registry.set_parents(22, vec![20]);
    registry.set_parents(23, vec![20]);
    registry.set_parents(30, vec![1]);
    registry.set_parents(101, vec![100]);
    registry.set_parents(102, vec![100]);

    registry.precompute_expanded_sets();
    registry
}

/// 准备计算上下文（Phase 1）
///
/// 执行聚合阶段，生成可复用的 PreparedContext。
/// 用于悬停预览场景的增量计算优化。
///
/// # 使用场景
/// ```ignore
/// // 基准计算
/// let base_ctx = prepare_context(&base_input)?;
/// let base_result = calculate_from_prepared(&base_ctx)?;
///
/// // 预览计算（增量）
/// let preview_ctx = prepare_context_incremental(&base_ctx, &preview_item)?;
/// let preview_result = calculate_from_prepared(&preview_ctx)?;
/// ```
pub fn prepare_context(input: &CalculatorInput) -> Result<PreparedContext, CalculationError> {
    let mut trace = Vec::new();

    // 0. 初始化标签注册表
    let registry = create_default_registry();

    // 1. Sanitization & Slot Conflict
    let sanitized_items = sanitize_items(&input.items, &input.preview_slot)?;
    trace.push(TraceEntry {
        phase: "Sanitization".to_string(),
        description: format!("Processed {} items", sanitized_items.len()),
        values: HashMap::new(),
        matched_tags: vec![],
    });

    // 2. 建立上下文标签
    let mut context = ContextTags::new(registry.clone());
    context.inject_skill_tags(&input.active_skill.tags);
    for support in &input.support_skills {
        context.inject_support_tags(&support.injected_tags);
    }
    context.inject_context_flags(&input.context_flags);

    // 2.5 初始化机制处理器
    let mechanics = MechanicsProcessor::new(
        input.mechanic_definitions.clone(),
        input.mechanic_states.clone(),
    );

    // 3. Stat Pool Aggregation
    let mut aggregator = StatAggregator::with_mechanics(&context, &mechanics);
    aggregator.aggregate_items(&sanitized_items);
    aggregator.aggregate_skill(&input.active_skill);
    aggregator.aggregate_support_skills(&input.support_skills);
    aggregator.aggregate_overrides(&input.global_overrides);
    aggregator.apply_mechanic_base_effects();

    let (stat_pool, mod_db) = aggregator.finalize();

    // 4. Base Calculation
    let base_damages = calculate_base_damage(&stat_pool, &input.active_skill);

    // 5. 提取转化规则
    let extra_as_rules = extract_extra_as_rules(&stat_pool);
    let conversion_rules = extract_conversion_rules(&stat_pool);

    // 创建技能快照
    let skill_snapshot = SkillSnapshot {
        id: input.active_skill.id.clone(),
        is_attack: input.active_skill.is_attack,
        base_time: input.active_skill.base_time,
        effectiveness: input.active_skill.effectiveness,
        tags: input.active_skill.tags.clone(),
    };

    Ok(PreparedContext {
        registry,
        stat_pool,
        mod_db,
        base_damages,
        skill_snapshot,
        mechanic_stacks: mechanics.get_all_stacks(),
        context_flags: input.context_flags.clone(),
        context_values: input.context_values.clone(),
        conversion_rules,
        extra_as_rules,
        trace,
    })
}

/// 从预处理上下文计算最终结果（Phase 2）
///
/// 复用 PreparedContext 中的中间数据进行后续计算阶段。
pub fn calculate_from_prepared(
    ctx: &PreparedContext,
    target_config: &TargetConfig,
) -> Result<CalculatorOutput, CalculationError> {
    let mut trace = ctx.trace.clone();

    // 5. Extra & Conversion (with Tag Retention)
    let engine = ConversionEngine::new((ctx.registry.max_id() + 1) as usize);
    let damage_pool = engine.process(
        &ctx.base_damages,
        &ctx.extra_as_rules,
        &ctx.conversion_rules,
        &ctx.registry,
    );

    // 创建临时 ContextTags 用于 apply_modifications
    let mut context = ContextTags::new(ctx.registry.clone());
    context.inject_skill_tags(&ctx.skill_snapshot.tags);
    context.inject_context_flags(&ctx.context_flags);

    // 6. Modification (Inc/More)
    let modified_damages = apply_modifications(&damage_pool, &ctx.stat_pool, &context);

    // Lucky 处理
    let is_lucky = ctx.stat_pool.get_base("flag.lucky") > 0.0
        || ctx.context_flags.get("lucky_damage").copied().unwrap_or(false);

    let total_damage: f64 = modified_damages
        .values()
        .map(|d| expected_damage(d.min, d.max, is_lucky))
        .sum();

    trace.push(TraceEntry {
        phase: "Modification".to_string(),
        description: "Applied Inc/More modifiers".to_string(),
        values: modified_damages
            .iter()
            .map(|(k, v)| (k.as_key().to_string(), v.average()))
            .collect(),
        matched_tags: vec![],
    });

    // 7. Speed Layer
    let rate = calculate_rate_from_pool(&ctx.stat_pool, &ctx.skill_snapshot);
    trace.push(TraceEntry {
        phase: "Speed".to_string(),
        description: format!("Attack/Cast rate: {:.2}/s", rate),
        values: [("rate".to_string(), rate)].into_iter().collect(),
        matched_tags: vec![],
    });

    // 8. Crit & Luck
    let (crit_chance, crit_multiplier) = calculate_crit(&ctx.stat_pool, &ctx.context_flags);
    let crit_factor = calculate_crit_factor(crit_chance, crit_multiplier);

    let hit_damage = total_damage * crit_factor;
    trace.push(TraceEntry {
        phase: "Critical".to_string(),
        description: format!(
            "Crit: {:.1}% chance, {:.1}% multi",
            crit_chance * 100.0,
            crit_multiplier * 100.0
        ),
        values: [
            ("crit_chance".to_string(), crit_chance),
            ("crit_multiplier".to_string(), crit_multiplier),
            ("crit_factor".to_string(), crit_factor),
        ]
        .into_iter()
        .collect(),
        matched_tags: vec![],
    });

    // 9. Mitigation
    let hit_chance = calculate_hit_chance(&ctx.stat_pool, target_config);
    let dps_theoretical = hit_damage * rate;
    let dps_effective = calculate_effective_dps(
        &modified_damages,
        rate,
        crit_factor,
        hit_chance,
        target_config,
    );

    // 10. EHP Calculation
    let ehp_series = calculate_ehp(&ctx.stat_pool);

    // 构建输出（使用 ModDB 提供详细来源）
    let damage_breakdown = build_damage_breakdown(
        &ctx.base_damages,
        &modified_damages,
        &ctx.stat_pool,
        Some(&ctx.mod_db),
        rate,
        crit_chance,
        crit_multiplier,
        hit_chance,
        target_config,
        is_lucky,
    );

    Ok(CalculatorOutput {
        dps_theoretical,
        dps_effective,
        hit_damage,
        rate,
        crit_chance,
        crit_multiplier,
        hit_chance,
        ehp_series,
        damage_breakdown,
        debug_trace: trace,
    })
}

/// 从 SkillSnapshot 计算速率（用于 PreparedContext）
fn calculate_rate_from_pool(pool: &StatPool, skill: &SkillSnapshot) -> f64 {
    let base_time = skill.base_time;
    if base_time <= 0.0 {
        return 1.0;
    }

    let base_rate = 1.0 / base_time;
    let speed_key = if skill.is_attack {
        "speed.attack"
    } else {
        "speed.cast"
    };
    let speed_inc = pool.get_increased(speed_key);
    let speed_more = pool.get_more_multiplier(speed_key);

    base_rate * (1.0 + speed_inc) * speed_more
}

/// 为预览装备创建增量 ModDB
///
/// 用于悬停预览场景：只聚合 preview item 的属性，返回增量 ModDB
pub fn prepare_item_modifiers(
    item: &ItemData,
    registry: &TagRegistry,
    mechanics: Option<&MechanicsProcessor>,
) -> ModDB {
    let context = ContextTags::new(registry.clone());
    let mut aggregator = if let Some(m) = mechanics {
        StatAggregator::with_mechanics(&context, m)
    } else {
        StatAggregator::new(&context)
    };

    aggregator.aggregate_single_item(item);
    let (_pool, mod_db) = aggregator.finalize();
    mod_db
}

/// 1. Sanitization & Slot Conflict
fn sanitize_items(
    items: &[ItemData],
    preview_slot: &Option<PreviewSlot>,
) -> Result<Vec<ItemData>, CalculationError> {
    let mut result: Vec<ItemData> = Vec::new();
    let mut slots_used: HashMap<SlotType, bool> = HashMap::new();
    let mut has_two_handed = false;

    // 如果有预览槽位，先检查是否为双手武器
    if let Some(preview) = preview_slot {
        if preview.item.is_two_handed {
            has_two_handed = true;
        }
    }

    // 处理现有装备
    for item in items {
        // 检查是否被预览槽位替换
        if let Some(preview) = preview_slot {
            if item.slot == preview.slot_type {
                continue; // 跳过，后面会添加预览装备
            }
        }

        // 双手武器互斥检查
        if item.is_two_handed {
            has_two_handed = true;
        }

        // 如果已有双手武器，忽略副手
        if has_two_handed && item.slot == SlotType::WeaponOff {
            continue;
        }

        // 检查槽位冲突
        if slots_used.contains_key(&item.slot) && !matches!(item.slot, SlotType::Ring1 | SlotType::Ring2) {
            // 允许两个戒指槽位
            continue;
        }

        slots_used.insert(item.slot, true);
        result.push(item.clone());
    }

    // 添加预览装备
    if let Some(preview) = preview_slot {
        // 双手武器检查
        if preview.item.is_two_handed {
            // 移除副手
            result.retain(|i| i.slot != SlotType::WeaponOff);
        }
        result.push(preview.item.clone());
    }

    Ok(result)
}

/// 获取技能在指定等级的有效数据
/// 
/// 逻辑：
/// - 1-20级：使用 level_data 中的具体数据
/// - 21-30级：使用20级数据 + 每级叠乘 1.10 (默认)
/// - 31级及以上：使用30级数据 + 每级叠乘 1.08 (默认)
fn get_skill_effective_data(skill: &SkillData) -> (HashMap<String, f64>, f64, f64) {
    let level = skill.level;
    
    // 如果有等级数据，使用它
    if let Some(level_data) = &skill.level_data {
        let base_damage = level_data.base_damage.clone();
        let effectiveness = level_data.effectiveness;
        
        // 计算等级缩放乘数
        let level_multiplier = calculate_level_scaling(level, &skill.scaling_rules);
        
        return (base_damage, effectiveness, level_multiplier);
    }
    
    // 否则使用默认数据
    let base_damage = skill.base_damage.clone();
    let effectiveness = skill.effectiveness;
    let level_multiplier = calculate_level_scaling(level, &skill.scaling_rules);
    
    (base_damage, effectiveness, level_multiplier)
}

/// 计算技能等级缩放乘数
/// 
/// 默认规则：
/// - 1-20级：无额外缩放 (multiplier = 1.0)
/// - 21-30级：每级 +10% (叠乘)
/// - 31级及以上：每级 +8% (叠乘)
fn calculate_level_scaling(level: u32, rules: &[SkillScalingRule]) -> f64 {
    if level <= 20 {
        return 1.0;
    }
    
    let mut multiplier = 1.0;
    
    // 使用自定义规则
    if !rules.is_empty() {
        for rule in rules {
            if level >= rule.level_start {
                let end = rule.level_end.unwrap_or(u32::MAX);
                if level <= end {
                    let levels_in_range = (level - rule.level_start + 1).min(
                        end.saturating_sub(rule.level_start) + 1
                    );
                    multiplier *= rule.multiplier_per_level.powi(levels_in_range as i32);
                } else {
                    // 完整应用此区间
                    let levels_in_range = end - rule.level_start + 1;
                    multiplier *= rule.multiplier_per_level.powi(levels_in_range as i32);
                }
            }
        }
    } else {
        // 使用默认规则
        // 21-30级：每级 +10%
        if level > 20 {
            let levels_21_30 = (level.min(30) - 20) as i32;
            multiplier *= 1.10_f64.powi(levels_21_30);
        }
        // 31级及以上：每级 +8%
        if level > 30 {
            let levels_31_plus = (level - 30) as i32;
            multiplier *= 1.08_f64.powi(levels_31_plus);
        }
    }
    
    multiplier
}

/// 3. 计算基础伤害
fn calculate_base_damage(
    pool: &StatPool,
    skill: &SkillData,
) -> HashMap<DamageType, (f64, f64)> {
    let mut base = HashMap::new();
    
    // 获取等级有效数据
    let (base_damage_map, effectiveness, level_multiplier) = get_skill_effective_data(skill);

    // 从技能获取基础伤害
    for (key, value) in &base_damage_map {
        if key.contains("phys") {
            let entry = base.entry(DamageType::Physical).or_insert((0.0, 0.0));
            if key.contains("min") {
                entry.0 += value;
            } else if key.contains("max") {
                entry.1 += value;
            }
        } else if key.contains("fire") {
            let entry = base.entry(DamageType::Fire).or_insert((0.0, 0.0));
            if key.contains("min") {
                entry.0 += value;
            } else if key.contains("max") {
                entry.1 += value;
            }
        } else if key.contains("cold") {
            let entry = base.entry(DamageType::Cold).or_insert((0.0, 0.0));
            if key.contains("min") {
                entry.0 += value;
            } else if key.contains("max") {
                entry.1 += value;
            }
        } else if key.contains("lightning") {
            let entry = base.entry(DamageType::Lightning).or_insert((0.0, 0.0));
            if key.contains("min") {
                entry.0 += value;
            } else if key.contains("max") {
                entry.1 += value;
            }
        } else if key.contains("chaos") {
            let entry = base.entry(DamageType::Chaos).or_insert((0.0, 0.0));
            if key.contains("min") {
                entry.0 += value;
            } else if key.contains("max") {
                entry.1 += value;
            }
        }
    }

    // 对于攻击技能，使用武器伤害
    if skill.is_attack {
        let phys_min = pool.get_base("dmg.phys.min");
        let phys_max = pool.get_base("dmg.phys.max");
        if phys_min > 0.0 || phys_max > 0.0 {
            let entry = base.entry(DamageType::Physical).or_insert((0.0, 0.0));
            entry.0 += phys_min;
            entry.1 += phys_max;
        }
    }

    // 应用等级缩放乘数 (21级及以上的 More 乘数)
    if level_multiplier > 1.0 {
    for (_, (min, max)) in base.iter_mut() {
            *min *= level_multiplier;
            *max *= level_multiplier;
        }
    }

    // 将“世事无常”一类的最小/最大伤害拉伸提前到点伤阶段
    // 仅作用于已有的 min/max 基础伤害桶，后续 Inc/More 不再二次放大这些拉伸
    let stretch_min_global = pool.get_more_multiplier("dmg.min");
    let stretch_max_global = pool.get_more_multiplier("dmg.max");
    let stretch_min_phys = pool.get_more_multiplier("dmg.phys.min");
    let stretch_max_phys = pool.get_more_multiplier("dmg.phys.max");

    for (dtype, (min, max)) in base.iter_mut() {
        let (smin, smax) = match dtype {
            DamageType::Physical => (
                stretch_min_global * stretch_min_phys,
                stretch_max_global * stretch_max_phys,
            ),
            _ => (stretch_min_global, stretch_max_global),
        };
        *min *= smin;
        *max *= smax;
    }

    base
}

/// 6. 应用 Inc/More 修正（带标签匹配）
fn apply_modifications(
    damage_pool: &HashMap<DamageType, DamageWithTags>,
    stat_pool: &StatPool,
    context: &ContextTags,
) -> HashMap<DamageType, DamageWithTags> {
    let mut result = HashMap::new();
    let registry = context.registry();

    for (dtype, dmg) in damage_pool {
        if dmg.is_zero() {
            continue;
        }

        let mut modified = dmg.clone();
        
        // 收集所有适用的 Inc 修正
        let mut total_inc = 0.0;
        
        // 全局伤害增加
        total_inc += stat_pool.get_increased("dmg.all");
        
        // 根据历史标签应用对应的 Inc
        // Physical Inc
        if dmg.history_tags.contains(registry.get_id("Tag_Physical").unwrap_or(0) as usize) {
            total_inc += stat_pool.get_increased("dmg.phys");
        }
        
        // Fire Inc
        if dmg.history_tags.contains(registry.get_id("Tag_Fire").unwrap_or(0) as usize) {
            total_inc += stat_pool.get_increased("dmg.fire");
        }
        
        // Cold Inc
        if dmg.history_tags.contains(registry.get_id("Tag_Cold").unwrap_or(0) as usize) {
            total_inc += stat_pool.get_increased("dmg.cold");
        }
        
        // Lightning Inc
        if dmg.history_tags.contains(registry.get_id("Tag_Lightning").unwrap_or(0) as usize) {
            total_inc += stat_pool.get_increased("dmg.lightning");
        }
        
        // Chaos Inc
        if dmg.history_tags.contains(registry.get_id("Tag_Chaos").unwrap_or(0) as usize) {
            total_inc += stat_pool.get_increased("dmg.chaos");
        }
        
        // Elemental Inc (如果有任何元素标签)
        let has_elemental = dmg.history_tags.contains(registry.get_id("Tag_Fire").unwrap_or(0) as usize)
            || dmg.history_tags.contains(registry.get_id("Tag_Cold").unwrap_or(0) as usize)
            || dmg.history_tags.contains(registry.get_id("Tag_Lightning").unwrap_or(0) as usize);
        if has_elemental {
            total_inc += stat_pool.get_increased("dmg.elemental");
        }

        // 技能类型 Inc
        if context.active_set().contains(registry.get_id("Tag_Spell").unwrap_or(0)) {
            total_inc += stat_pool.get_increased("dmg.spell");
        }
        if context.active_set().contains(registry.get_id("Tag_Attack").unwrap_or(0)) {
            total_inc += stat_pool.get_increased("dmg.attack");
        }
        if context.active_set().contains(registry.get_id("Tag_Melee").unwrap_or(0)) {
            total_inc += stat_pool.get_increased("dmg.melee");
        }
        if context.active_set().contains(registry.get_id("Tag_AOE").unwrap_or(0)) {
            total_inc += stat_pool.get_increased("dmg.aoe");
        }
        if context.active_set().contains(registry.get_id("Tag_Projectile").unwrap_or(0)) {
            total_inc += stat_pool.get_increased("dmg.projectile");
        }

        // 应用 Inc
        let inc_multiplier = 1.0 + total_inc;
        
        // 收集 More 修正（支持按类型/全局/最小值/最大值拆分，并按历史标签叠加）
        let more_all = stat_pool.get_more_multiplier("dmg.all");
        let more_type = match dtype {
            DamageType::Physical => stat_pool.get_more_multiplier("dmg.phys"),
            DamageType::Fire => stat_pool.get_more_multiplier("dmg.fire"),
            DamageType::Cold => stat_pool.get_more_multiplier("dmg.cold"),
            DamageType::Lightning => stat_pool.get_more_multiplier("dmg.lightning"),
            DamageType::Chaos => stat_pool.get_more_multiplier("dmg.chaos"),
        };
        // 法术专属 more（积聚等效果）：作为独立乘区参与
        let more_spell = if context.active_set().contains(registry.get_id("Tag_Spell").unwrap_or(0)) {
            stat_pool.get_more_multiplier("dmg.spell")
        } else {
            1.0
        };
        // 基于历史标签的 more（转化后仍享受源类型 more），避免与当前类型重复叠乘
        let mut more_history = 1.0;
        let current_tag = match dtype {
            DamageType::Physical => registry.get_id("Tag_Physical"),
            DamageType::Fire => registry.get_id("Tag_Fire"),
            DamageType::Cold => registry.get_id("Tag_Cold"),
            DamageType::Lightning => registry.get_id("Tag_Lightning"),
            DamageType::Chaos => registry.get_id("Tag_Chaos"),
        };
        let apply_history = |hist: &fixedbitset::FixedBitSet, tag_id: Option<u32>, key: &str, acc: &mut f64| {
            if let Some(id) = tag_id {
                if hist.contains(id as usize) {
                    *acc *= stat_pool.get_more_multiplier(key);
                }
            }
        };
        // 仅当历史标签与当前类型不同才叠乘
        let hist = &dmg.history_tags;
        if current_tag != registry.get_id("Tag_Lightning") {
            apply_history(hist, registry.get_id("Tag_Lightning"), "dmg.lightning", &mut more_history);
        }
        if current_tag != registry.get_id("Tag_Cold") {
            apply_history(hist, registry.get_id("Tag_Cold"), "dmg.cold", &mut more_history);
        }
        if current_tag != registry.get_id("Tag_Fire") {
            apply_history(hist, registry.get_id("Tag_Fire"), "dmg.fire", &mut more_history);
        }
        if current_tag != registry.get_id("Tag_Physical") {
            apply_history(hist, registry.get_id("Tag_Physical"), "dmg.phys", &mut more_history);
        }
        if current_tag != registry.get_id("Tag_Chaos") {
            apply_history(hist, registry.get_id("Tag_Chaos"), "dmg.chaos", &mut more_history);
        }
        // 最小/最大拉伸已在基础伤害阶段应用，这里置为 1 以避免重复放大
        let more_min_generic = 1.0;
        let more_max_generic = 1.0;
        let more_min_type = match dtype {
            DamageType::Physical => 1.0,
            _ => stat_pool.get_more_multiplier(&format!("dmg.{}.min", dtype.as_key())),
        };
        let more_max_type = match dtype {
            DamageType::Physical => 1.0,
            _ => stat_pool.get_more_multiplier(&format!("dmg.{}.max", dtype.as_key())),
        };
        
        let more_multiplier_min = more_all * more_type * more_spell * more_history * more_min_generic * more_min_type;
        let more_multiplier_max = more_all * more_type * more_spell * more_history * more_max_generic * more_max_type;
        
        // 应用所有修正
        modified.min *= inc_multiplier * more_multiplier_min;
        modified.max *= inc_multiplier * more_multiplier_max;
        
        result.insert(*dtype, modified);
    }

    result
}

/// 7. 计算攻击/施法速率
fn calculate_rate(pool: &StatPool, skill: &SkillData) -> f64 {
    let base_time = skill.base_time;
    if base_time <= 0.0 {
        return 1.0;
    }

    let base_rate = 1.0 / base_time;

    // 选择攻速还是施法速度
    let speed_key = if skill.is_attack {
        "speed.attack"
    } else {
        "speed.cast"
    };

    let speed_inc = pool.get_increased(speed_key);
    let speed_more = pool.get_more_multiplier(speed_key);
    
    // 武器基础攻速（如果是攻击）
    // 默认武器攻速为 1.0，只有明确设置时才使用设置值
    let weapon_speed = if skill.is_attack {
        let base_speed = pool.get_base("weapon.base_speed");
        if base_speed > 0.0 { base_speed } else { 1.0 }
    } else {
        1.0
    };

    let rate = base_rate * weapon_speed * (1.0 + speed_inc) * speed_more;

    // 处理冷却限制
    if let Some(cd) = skill.cooldown {
        if cd > 0.0 {
            let cd_rate = 1.0 / cd;
            return rate.min(cd_rate);
        }
    }

    rate
}

/// 8. 计算暴击
fn calculate_crit(pool: &StatPool, context_flags: &HashMap<String, bool>) -> (f64, f64) {
    // 基础暴击率
    let base_crit = pool.get_base("crit.chance");
    let crit_inc = pool.get_increased("crit.chance");
    let crit_chance = (base_crit * (1.0 + crit_inc)).min(1.0).max(0.0);

    // 暴击伤害
    let base_multi = 1.5; // 基础暴击伤害 150%
    let crit_dmg_inc = pool.get_increased("crit.dmg");
    let crit_multiplier = base_multi + crit_dmg_inc;

    // 检查是否无法暴击
    if context_flags.get("cannot_crit").copied().unwrap_or(false) {
        return (0.0, 1.0);
    }

    (crit_chance, crit_multiplier)
}

/// 计算暴击因子
fn calculate_crit_factor(crit_chance: f64, crit_multiplier: f64) -> f64 {
    // 平均伤害 = (1 - crit_chance) * 1.0 + crit_chance * crit_multiplier
    1.0 + crit_chance * (crit_multiplier - 1.0)
}

/// 计算期望伤害，支持 Lucky 机制
/// Lucky: 取两次掷骰较高值，等价于区间 [min, max] 的期望从 0.5 提升到 2/3
fn expected_damage(min: f64, max: f64, is_lucky: bool) -> f64 {
    if !is_lucky || max <= min {
        return (min + max) / 2.0;
    }

    // 期望 = min + (max - min) * 2/3
    min + (max - min) * (2.0 / 3.0)
}

/// 9. 计算命中率
fn calculate_hit_chance(pool: &StatPool, _target: &TargetConfig) -> f64 {
    let base_acc = pool.get_base("acc.rating");
    let acc_chance = pool.get_base("acc.chance");

    // 简化的命中计算
    if acc_chance > 0.0 {
        acc_chance.min(1.0)
    } else if base_acc > 0.0 {
        // 基于命中值计算（简化公式）
        (base_acc / (base_acc + 100.0)).min(0.95)
    } else {
        0.95 // 默认95%命中
    }
}

/// 计算有效 DPS（考虑目标抗性）
fn calculate_effective_dps(
    damages: &HashMap<DamageType, DamageWithTags>,
    rate: f64,
    crit_factor: f64,
    hit_chance: f64,
    target: &TargetConfig,
) -> f64 {
    let mut total = 0.0;

    for (dtype, dmg) in damages {
        let avg = dmg.average() * crit_factor;
        
        // 获取目标抗性
        let resistance = target
            .resistances
            .get(dtype.as_key())
            .copied()
            .unwrap_or(0.0);
        
        // 简化的减伤计算
        let damage_taken = avg * (1.0 - resistance) * (1.0 - target.generic_dr);
        total += damage_taken;
    }

    total * rate * hit_chance
}

/// 10. 计算 EHP
fn calculate_ehp(pool: &StatPool) -> EhpSeries {
    let base_life = pool.get_base("base.life").max(1.0);
    let armor = pool.get_base("def.armor");
    
    // 物理 EHP = Life / (1 - phys_reduction)
    // 简化：phys_reduction = armor / (armor + 1000)
    let phys_reduction = armor / (armor + 1000.0);
    let phys_ehp = base_life / (1.0 - phys_reduction).max(0.01);

    // 元素 EHP = Life / (1 - res)
    let fire_res = pool.get_base("res.fire").min(0.75);
    let cold_res = pool.get_base("res.cold").min(0.75);
    let lightning_res = pool.get_base("res.lightning").min(0.75);
    let chaos_res = pool.get_base("res.chaos").min(0.75);

    EhpSeries {
        physical: phys_ehp,
        fire: base_life / (1.0 - fire_res).max(0.01),
        cold: base_life / (1.0 - cold_res).max(0.01),
        lightning: base_life / (1.0 - lightning_res).max(0.01),
        chaos: base_life / (1.0 - chaos_res).max(0.01),
    }
}

/// 构建伤害明细
/// 构建伤害分解明细，包含各乘区详情
/// 
/// 借鉴 ZSim 的设计，将伤害拆分为独立乘区：
/// - 基础伤害区、增伤区、More区、暴击区、速度区、命中区、防御区、抗性区、易伤区
fn build_damage_breakdown(
    base_damages: &HashMap<DamageType, (f64, f64)>,
    modified_damages: &HashMap<DamageType, DamageWithTags>,
    pool: &StatPool,
    mod_db: Option<&ModDB>,
    rate: f64,
    crit_chance: f64,
    crit_multiplier: f64,
    hit_chance: f64,
    target: &TargetConfig,
    is_lucky: bool,
) -> DamageBreakdown {
    let mut by_type = HashMap::new();
    let mut after_conversion = HashMap::new();

    for (dtype, dmg) in modified_damages {
        by_type.insert(dtype.as_key().to_string(), expected_damage(dmg.min, dmg.max, is_lucky));
        after_conversion.insert(
            dtype.as_key().to_string(),
            DamageWithHistory {
                damage: expected_damage(dmg.min, dmg.max, is_lucky),
                history_tags: dmg
                    .history_tags
                    .ones()
                    .map(|i| format!("tag_{}", i))
                    .collect(),
            },
        );
    }

    let base_damage: f64 = base_damages
        .values()
        .map(|(min, max)| (min + max) / 2.0)
        .sum();

    // 计算各乘区明细（传入 ModDB 以获取详细来源）
    let multipliers = build_multiplier_breakdown(
        base_damage,
        pool,
        mod_db,
        rate,
        crit_chance,
        crit_multiplier,
        hit_chance,
        target,
    );

    DamageBreakdown {
        by_type,
        base_damage,
        total_increased: pool.get_increased("dmg.all"),
        total_more: pool.get_more_multiplier("dmg.all"),
        after_conversion,
        multipliers,
    }
}

/// 构建乘区明细
/// 
/// 各乘区计算公式：
/// - 基础伤害区: 技能基础伤害值
/// - 增伤区: 1 + sum(所有 increased)
/// - More区: product(所有 more)
/// - 暴击期望区: 1 + crit_chance * crit_damage
/// - 速度区: 攻击/施法速率
/// - 命中区: 命中率
/// - 防御区: level_constant / (enemy_armor + level_constant)
/// - 抗性区: 1 - enemy_res + res_reduction + res_penetration
/// - 易伤区: 1 + enemy_increased_damage_taken
fn build_multiplier_breakdown(
    base_damage: f64,
    pool: &StatPool,
    mod_db: Option<&ModDB>,
    rate: f64,
    crit_chance: f64,
    crit_multiplier: f64,
    hit_chance: f64,
    target: &TargetConfig,
) -> MultiplierBreakdown {
    use crate::modifiers::{ModifierKind, ModifierStore};

    let mut zone_sources: HashMap<String, Vec<ZoneSource>> = HashMap::new();

    // 1. 基础伤害区
    let base_damage_zone = base_damage;
    zone_sources.insert("base_damage".to_string(), vec![ZoneSource {
        source: "技能基础".to_string(),
        value: base_damage,
        stat_key: "dmg.base".to_string(),
    }]);

    // 2. 增伤区 (收集所有 increased 来源)
    let inc_keys = ["dmg.all", "dmg.phys", "dmg.fire", "dmg.cold", 
                    "dmg.lightning", "dmg.elemental", "dmg.chaos", "dmg.spell", "dmg.attack"];
    let inc_names = ["全伤害增加", "物理增伤", "火焰增伤", "冰冷增伤",
                     "闪电增伤", "元素增伤", "混沌增伤", "法术增伤", "攻击增伤"];
    
    let mut total_increased = 0.0;
    let mut inc_sources = Vec::new();
    
    for (key, name) in inc_keys.iter().zip(inc_names.iter()) {
        let value = pool.get_increased(key);
        if value > 0.0 {
            total_increased += value;
            
            // 如果有 ModDB，获取详细来源
            if let Some(db) = mod_db {
                let sources = db.get_sources(key);
                for src in sources.iter().filter(|s| s.kind == ModifierKind::Increased) {
                    inc_sources.push(ZoneSource {
                        source: format!("{} ({})", src.source, name),
                        value: src.value,
                        stat_key: key.to_string(),
                    });
                }
            } else {
                inc_sources.push(ZoneSource {
                    source: name.to_string(),
                    value,
                    stat_key: key.to_string(),
                });
            }
        }
    }
    
    let increased_zone = 1.0 + total_increased;
    zone_sources.insert("increased".to_string(), inc_sources);

    // 3. More 乘区
    let more_keys = ["dmg.all", "dmg.phys", "dmg.fire", "dmg.cold",
                     "dmg.lightning", "dmg.elemental", "dmg.spell", "dmg.attack"];
    let more_names = ["全伤害提高", "物理伤害提高", "火焰伤害提高", "冰冷伤害提高",
                      "闪电伤害提高", "元素伤害提高", "法术伤害提高", "攻击伤害提高"];
    
    let mut more_zone = 1.0;
    let mut more_sources = Vec::new();
    
    for (key, name) in more_keys.iter().zip(more_names.iter()) {
        let value = pool.get_more_multiplier(key);
        if value != 1.0 {
            more_zone *= value;
            
            // 如果有 ModDB，获取详细来源
            if let Some(db) = mod_db {
                let sources = db.get_sources(key);
                for src in sources.iter().filter(|s| s.kind == ModifierKind::More) {
                    more_sources.push(ZoneSource {
                        source: format!("{} ({})", src.source, name),
                        value: 1.0 + src.value, // More 值显示为乘数形式
                        stat_key: key.to_string(),
                    });
                }
            } else {
                more_sources.push(ZoneSource {
                    source: name.to_string(),
                    value,
                    stat_key: key.to_string(),
                });
            }
        }
    }
    
    zone_sources.insert("more".to_string(), more_sources);

    // 4. 暴击期望区
    // 公式: 1 + crit_chance * (crit_multiplier - 1)
    // crit_multiplier 语义: 1.5 = 150% 总暴击伤害 (非暴击时为 100%)
    // 例: 50% 暴击率, 150% 暴击伤害 → 1 + 0.5 * 0.5 = 1.25 倍期望伤害
    let effective_crit_chance = crit_chance.min(1.0).max(0.0);
    let crit_zone = 1.0 + effective_crit_chance * (crit_multiplier - 1.0);
    zone_sources.insert("crit".to_string(), vec![
        ZoneSource {
            source: "暴击率".to_string(),
            value: crit_chance,
            stat_key: "crit.chance".to_string(),
        },
        ZoneSource {
            source: "暴击伤害".to_string(),
            value: crit_multiplier,
            stat_key: "crit.multiplier".to_string(),
        },
    ]);

    // 5. 速度区
    let speed_zone = rate;
    zone_sources.insert("speed".to_string(), vec![ZoneSource {
        source: "攻击/施法速率".to_string(),
        value: rate,
        stat_key: "rate".to_string(),
    }]);

    // 6. 命中区
    let hit_zone = hit_chance;
    zone_sources.insert("hit".to_string(), vec![ZoneSource {
        source: "命中率".to_string(),
        value: hit_chance,
        stat_key: "hit.chance".to_string(),
    }]);

    // 7. 防御区 (敌人护甲)
    // 公式: level_constant / (enemy_armor + level_constant)
    let level_constant = 1000.0; // 等级常数，后续可参数化
    let enemy_armor = target.armor as f64;
    let defense_zone = if enemy_armor > 0.0 {
        level_constant / (enemy_armor + level_constant)
    } else {
        1.0
    };
    zone_sources.insert("defense".to_string(), vec![ZoneSource {
        source: format!("敌人护甲: {}", enemy_armor),
        value: defense_zone,
        stat_key: "target.armor".to_string(),
    }]);

    // 8. 抗性区
    // 公式: 1 - enemy_res + res_reduction + res_penetration
    // 取平均抗性作为示例
    let avg_resistance = (target.resistances.get("fire").unwrap_or(&0.0)
        + target.resistances.get("cold").unwrap_or(&0.0)
        + target.resistances.get("lightning").unwrap_or(&0.0)
        + target.resistances.get("chaos").unwrap_or(&0.0)) / 4.0;
    let res_penetration = pool.get_base("mod.penetration.res.all");
    let resistance_zone = (1.0 - avg_resistance + res_penetration).max(0.0);
    zone_sources.insert("resistance".to_string(), vec![ZoneSource {
        source: format!("平均抗性: {:.1}%", avg_resistance * 100.0),
        value: resistance_zone,
        stat_key: "target.resistance".to_string(),
    }]);

    // 9. 易伤区
    let vulnerability = pool.get_base("target.increased_damage_taken");
    let vulnerability_zone = 1.0 + vulnerability;
    zone_sources.insert("vulnerability".to_string(), vec![ZoneSource {
        source: "敌人受到伤害增加".to_string(),
        value: vulnerability,
        stat_key: "target.increased_damage_taken".to_string(),
    }]);

    // 10. 机制特殊区 (祝福、球类等提供的额外乘区)
    let mechanics_more = pool.get_base("mechanics.more.dmg");
    let mechanics_zone = if mechanics_more > 0.0 { 1.0 + mechanics_more } else { 1.0 };
    zone_sources.insert("mechanics".to_string(), vec![ZoneSource {
        source: "机制加成".to_string(),
        value: mechanics_more,
        stat_key: "mechanics.more.dmg".to_string(),
    }]);

    MultiplierBreakdown {
        base_damage_zone,
        increased_zone,
        more_zone,
        crit_zone,
        speed_zone,
        hit_zone,
        defense_zone,
        resistance_zone,
        vulnerability_zone,
        mechanics_zone,
        zone_sources,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::MechanicDefinition;

    fn create_test_input() -> CalculatorInput {
        CalculatorInput {
            context_flags: HashMap::new(),
            context_values: HashMap::new(),
            target_config: TargetConfig::default(),
            items: vec![],
            active_skill: SkillData {
                id: "test_fireball".to_string(),
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
    fn test_basic_calculation() {
        let input = create_test_input();
        let result = calculate_dps(&input).unwrap();

        // 基础伤害 75 (平均)
        // 速率 1.25/s
        // 理论 DPS ≈ 75 * 1.25 * crit_factor
        assert!(result.dps_theoretical > 0.0);
        assert!(result.hit_damage > 0.0);
        assert!(result.rate > 0.0);
    }

    #[test]
    fn test_with_increased_damage() {
        let mut input = create_test_input();
        input.global_overrides.insert("mod.inc.dmg.fire".to_string(), 1.0); // +100% fire damage

        let result = calculate_dps(&input).unwrap();

        // 伤害应该翻倍
        let base_result = calculate_dps(&create_test_input()).unwrap();
        assert!(result.hit_damage > base_result.hit_damage * 1.5);
    }

    #[test]
    fn test_conversion_with_tag_retention() {
        // 测试物理转火焰，确保火焰部分也能吃到物理增伤
        let mut input = create_test_input();
        input.active_skill.is_attack = true;
        input.active_skill.base_damage.clear();
        input.active_skill.tags = vec!["Tag_Attack".to_string(), "Tag_Melee".to_string()];

        // 添加武器物理伤害
        input.items.push(ItemData {
            id: "test_sword".to_string(),
            base_type: "sword".to_string(),
            slot: SlotType::WeaponMain,
            is_two_handed: false,
            base_implicit_stats: HashMap::new(), // 武器基底属性（无）
            implicit_stats: [
                ("dmg.phys.min".to_string(), 50.0),
                ("dmg.phys.max".to_string(), 100.0),
            ]
            .into_iter()
            .collect(),
            affixes: vec![],
            tags: vec![],
            is_unique: false,
            is_corrupted: false,
        });

        // 50% 物理转火焰
        input.global_overrides.insert("conv.phys_to_fire".to_string(), 0.5);
        // +100% 物理增伤
        input.global_overrides.insert("mod.inc.dmg.phys".to_string(), 1.0);
        // +100% 火焰增伤
        input.global_overrides.insert("mod.inc.dmg.fire".to_string(), 1.0);

        let result = calculate_dps(&input).unwrap();

        // 确保计算正常完成
        assert!(result.dps_theoretical > 0.0);
        
        // 检查伤害构成
        assert!(result.damage_breakdown.by_type.contains_key("physical"));
        assert!(result.damage_breakdown.by_type.contains_key("fire"));
    }

    #[test]
    fn test_chain_lightning_with_supports_and_blessings() {
        // 战意 100 层，聚能祝福 6 层，侵蚀版旧律最大值
        let input = CalculatorInput {
            context_flags: HashMap::from([
                ("lucky_damage".to_string(), false),
                ("cannot_crit".to_string(), false),
            ]),
            context_values: HashMap::new(),
            target_config: TargetConfig::default(),
            items: vec![ItemData {
                id: "equip_legend_116".to_string(),
                base_type: "gloves_all_magic_grip".to_string(),
                slot: SlotType::Gloves,
                is_two_handed: false,
                base_implicit_stats: HashMap::from([("base.es".to_string(), 527.0)]),
                implicit_stats: HashMap::from([
                    ("mod.more.dmg.cold.per_focus_blessing".to_string(), 0.19),
                    ("mod.inc.crit.dmg.per_focus_blessing".to_string(), 0.04),
                    ("blessing.duration".to_string(), 0.40),
                ]),
                affixes: vec![],
                tags: vec!["Tag_Armor".to_string(), "Tag_Gloves".to_string(), "Tag_Cold".to_string()],
                is_unique: true,
                is_corrupted: true,
            }],
            active_skill: SkillData {
                id: "skill_chain_lightning".to_string(),
                skill_type: SkillType::Active,
                damage_type: Some("lightning".to_string()),
                is_attack: false,
                level: 21,
                base_damage: HashMap::from([
                    ("dmg.lightning.min".to_string(), 95.0),
                    ("dmg.lightning.max".to_string(), 1811.0),
                ]),
                base_time: 0.65,
                cooldown: None,
                mana_cost: 8,
                effectiveness: 1.0, // 避免重复乘效用
                tags: vec![
                    "Tag_Spell".to_string(),
                    "Tag_Lightning".to_string(),
                    "Tag_Chain".to_string(),
                    "Tag_Burst".to_string(),
                ],
                stats: HashMap::new(),
                injected_tags: vec![],
                mana_multiplier: 1.0,
                level_data: None,
                scaling_rules: vec![
                    SkillScalingRule { level_start: 21, level_end: Some(30), multiplier_per_level: 1.10 },
                    SkillScalingRule { level_start: 31, level_end: None, multiplier_per_level: 1.08 },
                ],
            },
            support_skills: vec![
                SkillData {
                    id: "support_lightning_to_cold".to_string(),
                    skill_type: SkillType::Support,
                    damage_type: None,
                    is_attack: false,
                    level: 20,
                    base_damage: HashMap::new(),
                    base_time: 0.0,
                    cooldown: None,
                    mana_cost: 0,
                    effectiveness: 1.0,
                    tags: vec!["Tag_Support".to_string(), "Tag_Lightning".to_string(), "Tag_Cold".to_string()],
                    stats: HashMap::from([
                        ("conv.lightning_to_cold".to_string(), 1.0),
                        ("mod.more.dmg.lightning".to_string(), 0.25),
                    ]),
                    injected_tags: vec![],
                    mana_multiplier: 1.0,
                    level_data: None,
                    scaling_rules: vec![],
                },
                SkillData {
                    id: "support_psychic_burst".to_string(),
                    skill_type: SkillType::Support,
                    damage_type: None,
                    is_attack: false,
                    level: 20,
                    base_damage: HashMap::new(),
                    base_time: 0.0,
                    cooldown: None,
                    mana_cost: 0,
                    effectiveness: 1.0,
                    tags: vec!["Tag_Support".to_string(), "Tag_Spell".to_string()],
                    stats: HashMap::from([
                        ("mod.more.dmg.all".to_string(), 0.45),
                        ("speed.cast".to_string(), 0.16),
                    ]),
                    injected_tags: vec![],
                    mana_multiplier: 1.0,
                    level_data: None,
                    scaling_rules: vec![],
                },
            ],
            global_overrides: HashMap::from([
                // 战意换算后的基础暴击率示例
                ("crit.chance".to_string(), 0.10),
                // 世事无常：拉伸最小/最大伤害范围（全局 + 物理）
                ("mod.more.dmg.phys.min".to_string(), -0.90),
                ("mod.more.dmg.phys.max".to_string(), 0.80),
                ("mod.more.dmg.min".to_string(), -0.40),
                ("mod.more.dmg.max".to_string(), 0.40),
            ]),
            preview_slot: None,
            mechanic_states: vec![
                MechanicState { id: "focus_blessing".to_string(), current_stacks: 6, max_stacks: 6, is_active: true },
                MechanicState { id: "fighting_will".to_string(), current_stacks: 100, max_stacks: 100, is_active: true },
            ],
            mechanic_definitions: vec![
                MechanicDefinition {
                    id: "focus_blessing".to_string(),
                    display_name: "聚能祝福".to_string(),
                    category: "blessing".to_string(),
                    tag_key: "Mech_Blessing".to_string(),
                    default_max_stacks: 6,
                    base_effect_per_stack: HashMap::from([
                        ("mod.more.dmg.all".to_string(), 0.04),
                        ("mod.more.dmg.spell".to_string(), 0.03),
                    ]),
                    description: "聚能祝福每层提供额外伤害".to_string(),
                },
                MechanicDefinition {
                    id: "fighting_will".to_string(),
                    display_name: "战意".to_string(),
                    category: "resource".to_string(),
                    tag_key: "Mech_FightingWill".to_string(),
                    default_max_stacks: 100,
                    base_effect_per_stack: HashMap::from([
                        ("crit.chance.rating".to_string(), 2.0),
                    ]),
                    description: "战意每层提供 2 点暴击值".to_string(),
                },
            ],
        };

        let result = calculate_dps(&input).expect("calc ok");
        println!(
            "dps_theoretical={:.2}, hit_damage={:.2}, rate={:.2}, crit={:.2}%/{:.2}x",
            result.dps_theoretical,
            result.hit_damage,
            result.rate,
            result.crit_chance * 100.0,
            result.crit_multiplier
        );

        assert!(result.dps_theoretical > 0.0);
        assert!(result.hit_damage > 0.0);
    }
}
