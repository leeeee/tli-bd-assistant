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

use crate::conversion::{
    extract_conversion_rules, extract_extra_as_rules, ConversionEngine, DamageType, DamageWithTags,
};
use crate::mechanics::MechanicsProcessor;
use crate::stats::{StatAggregator, StatPool};
use crate::tags::{ContextTags, TagRegistry};
use crate::types::*;
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
    
    let stat_pool = aggregator.finalize();

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
    
    let total_damage: f64 = modified_damages.values().map(|d| d.average()).sum();
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

    // 11. Build damage breakdown (带乘区明细)
    let damage_breakdown = build_damage_breakdown(
        &base_damages,
        &modified_damages,
        &stat_pool,
        rate,
        crit_chance,
        crit_multiplier,
        hit_chance,
        &input.target_config,
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

/// 创建默认的标签注册表
fn create_default_registry() -> TagRegistry {
    let mut registry = TagRegistry::new();

    // 伤害类型标签
    registry.register("Tag_Damage".to_string(), 1);
    registry.register("Tag_Physical".to_string(), 10);
    registry.register("Tag_Elemental".to_string(), 20);
    registry.register("Tag_Fire".to_string(), 21);
    registry.register("Tag_Cold".to_string(), 22);
    registry.register("Tag_Lightning".to_string(), 23);
    registry.register("Tag_Chaos".to_string(), 30);

    // 技能类型标签
    registry.register("Tag_Attack".to_string(), 100);
    registry.register("Tag_Melee".to_string(), 101);
    registry.register("Tag_Ranged".to_string(), 102);
    registry.register("Tag_Spell".to_string(), 110);
    registry.register("Tag_AOE".to_string(), 120);
    registry.register("Tag_Projectile".to_string(), 103);
    registry.register("Tag_DOT".to_string(), 130);

    // 设置继承关系
    registry.set_parents(10, vec![1]);  // Physical -> Damage
    registry.set_parents(20, vec![1]);  // Elemental -> Damage
    registry.set_parents(21, vec![20]); // Fire -> Elemental
    registry.set_parents(22, vec![20]); // Cold -> Elemental
    registry.set_parents(23, vec![20]); // Lightning -> Elemental
    registry.set_parents(30, vec![1]);  // Chaos -> Damage
    registry.set_parents(101, vec![100]); // Melee -> Attack
    registry.set_parents(102, vec![100]); // Ranged -> Attack

    registry.precompute_expanded_sets();
    registry
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

    // 应用 Damage Effectiveness
    for (_, (min, max)) in base.iter_mut() {
        *min *= effectiveness;
        *max *= effectiveness;
    }
    
    // 应用等级缩放乘数 (21级及以上的 More 乘数)
    if level_multiplier > 1.0 {
        for (_, (min, max)) in base.iter_mut() {
            *min *= level_multiplier;
            *max *= level_multiplier;
        }
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
        
        // 收集 More 修正
        let more_multiplier = stat_pool.get_more_multiplier("dmg.all");
        
        // 应用所有修正
        modified.min *= inc_multiplier * more_multiplier;
        modified.max *= inc_multiplier * more_multiplier;
        
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
    rate: f64,
    crit_chance: f64,
    crit_multiplier: f64,
    hit_chance: f64,
    target: &TargetConfig,
) -> DamageBreakdown {
    let mut by_type = HashMap::new();
    let mut after_conversion = HashMap::new();

    for (dtype, dmg) in modified_damages {
        by_type.insert(dtype.as_key().to_string(), dmg.average());
        after_conversion.insert(
            dtype.as_key().to_string(),
            DamageWithHistory {
                damage: dmg.average(),
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

    // 计算各乘区明细
    let multipliers = build_multiplier_breakdown(
        base_damage,
        pool,
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
    rate: f64,
    crit_chance: f64,
    crit_multiplier: f64,
    hit_chance: f64,
    target: &TargetConfig,
) -> MultiplierBreakdown {
    let mut zone_sources: HashMap<String, Vec<ZoneSource>> = HashMap::new();

    // 1. 基础伤害区
    let base_damage_zone = base_damage;
    zone_sources.insert("base_damage".to_string(), vec![ZoneSource {
        source: "技能基础".to_string(),
        value: base_damage,
        stat_key: "dmg.base".to_string(),
    }]);

    // 2. 增伤区 (收集所有 increased 来源)
    let inc_dmg_all = pool.get_increased("mod.inc.dmg.all");
    let inc_dmg_phys = pool.get_increased("mod.inc.dmg.phys");
    let inc_dmg_fire = pool.get_increased("mod.inc.dmg.fire");
    let inc_dmg_cold = pool.get_increased("mod.inc.dmg.cold");
    let inc_dmg_lightning = pool.get_increased("mod.inc.dmg.lightning");
    let inc_dmg_elemental = pool.get_increased("mod.inc.dmg.elemental");
    let inc_dmg_chaos = pool.get_increased("mod.inc.dmg.chaos");
    let inc_dmg_spell = pool.get_increased("mod.inc.dmg.spell");
    let inc_dmg_attack = pool.get_increased("mod.inc.dmg.attack");
    
    // 综合增伤区 = 1 + sum(各类增伤)
    let total_increased = inc_dmg_all + inc_dmg_phys + inc_dmg_fire + inc_dmg_cold 
        + inc_dmg_lightning + inc_dmg_elemental + inc_dmg_chaos + inc_dmg_spell + inc_dmg_attack;
    let increased_zone = 1.0 + total_increased;
    
    let mut inc_sources = Vec::new();
    if inc_dmg_all > 0.0 {
        inc_sources.push(ZoneSource {
            source: "全伤害增加".to_string(),
            value: inc_dmg_all,
            stat_key: "mod.inc.dmg.all".to_string(),
        });
    }
    if inc_dmg_phys > 0.0 {
        inc_sources.push(ZoneSource {
            source: "物理增伤".to_string(),
            value: inc_dmg_phys,
            stat_key: "mod.inc.dmg.phys".to_string(),
        });
    }
    if inc_dmg_fire > 0.0 {
        inc_sources.push(ZoneSource {
            source: "火焰增伤".to_string(),
            value: inc_dmg_fire,
            stat_key: "mod.inc.dmg.fire".to_string(),
        });
    }
    if inc_dmg_cold > 0.0 {
        inc_sources.push(ZoneSource {
            source: "冰冷增伤".to_string(),
            value: inc_dmg_cold,
            stat_key: "mod.inc.dmg.cold".to_string(),
        });
    }
    if inc_dmg_lightning > 0.0 {
        inc_sources.push(ZoneSource {
            source: "闪电增伤".to_string(),
            value: inc_dmg_lightning,
            stat_key: "mod.inc.dmg.lightning".to_string(),
        });
    }
    if inc_dmg_elemental > 0.0 {
        inc_sources.push(ZoneSource {
            source: "元素增伤".to_string(),
            value: inc_dmg_elemental,
            stat_key: "mod.inc.dmg.elemental".to_string(),
        });
    }
    if inc_dmg_chaos > 0.0 {
        inc_sources.push(ZoneSource {
            source: "混沌增伤".to_string(),
            value: inc_dmg_chaos,
            stat_key: "mod.inc.dmg.chaos".to_string(),
        });
    }
    if inc_dmg_spell > 0.0 {
        inc_sources.push(ZoneSource {
            source: "法术增伤".to_string(),
            value: inc_dmg_spell,
            stat_key: "mod.inc.dmg.spell".to_string(),
        });
    }
    if inc_dmg_attack > 0.0 {
        inc_sources.push(ZoneSource {
            source: "攻击增伤".to_string(),
            value: inc_dmg_attack,
            stat_key: "mod.inc.dmg.attack".to_string(),
        });
    }
    zone_sources.insert("increased".to_string(), inc_sources);

    // 3. More 乘区
    let more_dmg_all = pool.get_more_multiplier("mod.more.dmg.all");
    let more_dmg_phys = pool.get_more_multiplier("mod.more.dmg.phys");
    let more_dmg_fire = pool.get_more_multiplier("mod.more.dmg.fire");
    let more_dmg_cold = pool.get_more_multiplier("mod.more.dmg.cold");
    let more_dmg_lightning = pool.get_more_multiplier("mod.more.dmg.lightning");
    let more_dmg_elemental = pool.get_more_multiplier("mod.more.dmg.elemental");
    let more_dmg_spell = pool.get_more_multiplier("mod.more.dmg.spell");
    let more_dmg_attack = pool.get_more_multiplier("mod.more.dmg.attack");
    
    // More 区 = product(各类 more)
    let more_zone = more_dmg_all * more_dmg_phys * more_dmg_fire * more_dmg_cold 
        * more_dmg_lightning * more_dmg_elemental * more_dmg_spell * more_dmg_attack;
    
    let mut more_sources = Vec::new();
    if more_dmg_all != 1.0 {
        more_sources.push(ZoneSource {
            source: "全伤害提高".to_string(),
            value: more_dmg_all,
            stat_key: "mod.more.dmg.all".to_string(),
        });
    }
    // ... 其他 more 来源同理 (简化)
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
}
