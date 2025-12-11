//! 机制系统模块
//!
//! 处理祝福、球类、资源等游戏机制的计算
//!
//! ## 支持的机制类型
//! 
//! - **祝福 (Blessing)**: 聚能祝福、坚韧祝福、灵动祝福
//! - **球类 (Charge)**: 狂乱球、能量球、耐力球
//! - **资源 (Resource)**: 怒火、护体等
//!
//! ## 核心设计
//!
//! 1. 每种机制有基础效果（每层提供的属性）
//! 2. 装备/天赋可以提供 `.per_xxx` 类型的属性，与层数相乘
//! 3. 机制处理器负责计算总效果并应用到属性池

use crate::types::{MechanicDefinition, MechanicState};
use std::collections::HashMap;

/// 机制处理器
/// 
/// 负责管理和计算所有机制的效果
pub struct MechanicsProcessor {
    /// 机制定义 (id -> definition)
    definitions: HashMap<String, MechanicDefinition>,
    /// 机制状态 (id -> state)
    states: HashMap<String, MechanicState>,
}

impl MechanicsProcessor {
    /// 创建新的机制处理器
    pub fn new(definitions: Vec<MechanicDefinition>, states: Vec<MechanicState>) -> Self {
        Self {
            definitions: definitions.into_iter().map(|d| (d.id.clone(), d)).collect(),
            states: states.into_iter().map(|s| (s.id.clone(), s)).collect(),
        }
    }

    /// 创建空的机制处理器（无任何机制）
    pub fn empty() -> Self {
        Self {
            definitions: HashMap::new(),
            states: HashMap::new(),
        }
    }

    /// 获取机制当前层数
    /// 
    /// 如果机制未激活或不存在，返回 0
    pub fn get_stacks(&self, mech_id: &str) -> u32 {
        self.states
            .get(mech_id)
            .filter(|s| s.is_active)
            .map(|s| s.current_stacks)
            .unwrap_or(0)
    }

    /// 检查机制是否激活
    pub fn is_active(&self, mech_id: &str) -> bool {
        self.states
            .get(mech_id)
            .map(|s| s.is_active)
            .unwrap_or(false)
    }

    /// 获取机制定义
    pub fn get_definition(&self, mech_id: &str) -> Option<&MechanicDefinition> {
        self.definitions.get(mech_id)
    }

    /// 获取机制状态
    pub fn get_state(&self, mech_id: &str) -> Option<&MechanicState> {
        self.states.get(mech_id)
    }

    /// 计算所有机制的基础效果
    /// 
    /// 返回 (属性键, 总值) 的映射
    /// 
    /// ## 示例
    /// 
    /// 如果聚能祝福有 4 层，每层 +4% 伤害：
    /// 返回 {"mod.inc.dmg.all": 0.16}
    pub fn calculate_base_effects(&self) -> HashMap<String, f64> {
        let mut effects = HashMap::new();

        for (mech_id, state) in &self.states {
            // 跳过未激活或 0 层的机制
            if !state.is_active || state.current_stacks == 0 {
                continue;
            }

            // 获取机制定义
            if let Some(def) = self.definitions.get(mech_id) {
                // 计算每层基础效果 × 层数
                for (key, value_per_stack) in &def.base_effect_per_stack {
                    let total_value = *value_per_stack * state.current_stacks as f64;
                    *effects.entry(key.clone()).or_insert(0.0) += total_value;
                }
            }
        }

        effects
    }

    /// 计算带层数乘算的属性值
    /// 
    /// 用于处理 `.per_xxx` 类型的属性
    /// 
    /// ## 参数
    /// 
    /// - `key`: 属性键，如 "mod.inc.dmg.cold.per_focus_blessing"
    /// - `value_per_stack`: 每层提供的值
    /// 
    /// ## 返回
    /// 
    /// - `Some((base_key, total_value))`: 如果机制激活且有层数
    /// - `None`: 如果机制未激活或层数为 0
    pub fn calculate_per_stack_value(&self, key: &str, value_per_stack: f64) -> Option<(String, f64)> {
        // 提取机制 ID
        let mech_id = extract_mechanic_id(key)?;
        
        // 获取层数
        let stacks = self.get_stacks(&mech_id);
        if stacks == 0 {
            return None;
        }

        // 计算实际值
        let total_value = value_per_stack * stacks as f64;

        // 提取不带 per_xxx 后缀的基础键
        let base_key = key.replace(&format!(".per_{}", mech_id), "");

        Some((base_key, total_value))
    }

    /// 获取所有激活机制的层数映射
    /// 
    /// 用于注入 context_values
    pub fn get_all_stacks(&self) -> HashMap<String, f64> {
        self.states
            .iter()
            .filter(|(_, s)| s.is_active)
            .map(|(id, s)| (format!("{}_stacks", id), s.current_stacks as f64))
            .collect()
    }

    /// 获取所有机制 ID
    pub fn all_mechanic_ids(&self) -> impl Iterator<Item = &String> {
        self.definitions.keys()
    }
}

/// 从属性键中提取机制 ID
/// 
/// ## 示例
/// 
/// - `"mod.inc.dmg.cold.per_focus_blessing"` -> `Some("focus_blessing")`
/// - `"mod.inc.dmg.cold"` -> `None`
pub fn extract_mechanic_id(key: &str) -> Option<String> {
    if let Some(idx) = key.find(".per_") {
        Some(key[idx + 5..].to_string())
    } else {
        None
    }
}

/// 检查属性键是否是 per_xxx 类型
pub fn is_per_stack_stat(key: &str) -> bool {
    key.contains(".per_")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_definitions() -> Vec<MechanicDefinition> {
        vec![
            MechanicDefinition {
                id: "focus_blessing".to_string(),
                display_name: "聚能祝福".to_string(),
                category: "blessing".to_string(),
                tag_key: "Mech_Blessing".to_string(),
                default_max_stacks: 4,
                base_effect_per_stack: [
                    ("mod.inc.dmg.all".to_string(), 0.04),
                ].into_iter().collect(),
                description: String::new(),
            },
            MechanicDefinition {
                id: "tenacity_blessing".to_string(),
                display_name: "坚韧祝福".to_string(),
                category: "blessing".to_string(),
                tag_key: "Mech_Blessing".to_string(),
                default_max_stacks: 4,
                base_effect_per_stack: [
                    ("def.damage_taken_reduction".to_string(), 0.04),
                ].into_iter().collect(),
                description: String::new(),
            },
            MechanicDefinition {
                id: "agility_blessing".to_string(),
                display_name: "灵动祝福".to_string(),
                category: "blessing".to_string(),
                tag_key: "Mech_Blessing".to_string(),
                default_max_stacks: 4,
                base_effect_per_stack: [
                    ("speed.attack".to_string(), 0.04),
                    ("speed.cast".to_string(), 0.04),
                    ("mod.inc.dmg.all".to_string(), 0.02),
                ].into_iter().collect(),
                description: String::new(),
            },
            MechanicDefinition {
                id: "fighting_will".to_string(),
                display_name: "战意".to_string(),
                category: "resource".to_string(),
                tag_key: "Mech_FightingWill".to_string(),
                default_max_stacks: 100,
                base_effect_per_stack: [
                    ("crit.chance.attack".to_string(), 0.02),
                    ("crit.chance.spell".to_string(), 0.02),
                ].into_iter().collect(),
                description: "每点战意值提供2%攻击和法术暴击值".to_string(),
            },
        ]
    }

    #[test]
    fn test_focus_blessing_base_effect() {
        // 聚能祝福 4 层：每层 +4% 伤害 = 总计 +16%
        let definitions = create_test_definitions();
        let states = vec![
            MechanicState {
                id: "focus_blessing".to_string(),
                current_stacks: 4,
                max_stacks: 4,
                is_active: true,
            },
        ];

        let processor = MechanicsProcessor::new(definitions, states);
        let effects = processor.calculate_base_effects();

        assert!((effects.get("mod.inc.dmg.all").copied().unwrap_or(0.0) - 0.16).abs() < 0.001);
    }

    #[test]
    fn test_tenacity_blessing_base_effect() {
        // 坚韧祝福 3 层：每层 -4% 受到伤害 = 总计 -12%
        let definitions = create_test_definitions();
        let states = vec![
            MechanicState {
                id: "tenacity_blessing".to_string(),
                current_stacks: 3,
                max_stacks: 4,
                is_active: true,
            },
        ];

        let processor = MechanicsProcessor::new(definitions, states);
        let effects = processor.calculate_base_effects();

        assert!((effects.get("def.damage_taken_reduction").copied().unwrap_or(0.0) - 0.12).abs() < 0.001);
    }

    #[test]
    fn test_agility_blessing_base_effect() {
        // 灵动祝福 4 层：每层 +4% 攻击/施法速度, +2% 伤害
        // 总计：+16% 攻速, +16% 施法速度, +8% 伤害
        let definitions = create_test_definitions();
        let states = vec![
            MechanicState {
                id: "agility_blessing".to_string(),
                current_stacks: 4,
                max_stacks: 4,
                is_active: true,
            },
        ];

        let processor = MechanicsProcessor::new(definitions, states);
        let effects = processor.calculate_base_effects();

        assert!((effects.get("speed.attack").copied().unwrap_or(0.0) - 0.16).abs() < 0.001);
        assert!((effects.get("speed.cast").copied().unwrap_or(0.0) - 0.16).abs() < 0.001);
        assert!((effects.get("mod.inc.dmg.all").copied().unwrap_or(0.0) - 0.08).abs() < 0.001);
    }
    
    #[test]
    fn test_fighting_will_base_effect() {
        // 战意 50 点：每点 +2% 攻击/法术暴击值
        // 总计：+100% 攻击暴击值, +100% 法术暴击值
        let definitions = create_test_definitions();
        let states = vec![
            MechanicState {
                id: "fighting_will".to_string(),
                current_stacks: 50,
                max_stacks: 100,
                is_active: true,
            },
        ];

        let processor = MechanicsProcessor::new(definitions, states);
        let effects = processor.calculate_base_effects();

        assert!((effects.get("crit.chance.attack").copied().unwrap_or(0.0) - 1.0).abs() < 0.001);
        assert!((effects.get("crit.chance.spell").copied().unwrap_or(0.0) - 1.0).abs() < 0.001);
    }
    
    #[test]
    fn test_fighting_will_max_stacks() {
        // 战意满层 100 点：每点 +2% 攻击/法术暴击值
        // 总计：+200% 攻击暴击值, +200% 法术暴击值
        let definitions = create_test_definitions();
        let states = vec![
            MechanicState {
                id: "fighting_will".to_string(),
                current_stacks: 100,
                max_stacks: 100,
                is_active: true,
            },
        ];

        let processor = MechanicsProcessor::new(definitions, states);
        let effects = processor.calculate_base_effects();

        assert!((effects.get("crit.chance.attack").copied().unwrap_or(0.0) - 2.0).abs() < 0.001);
        assert!((effects.get("crit.chance.spell").copied().unwrap_or(0.0) - 2.0).abs() < 0.001);
    }

    #[test]
    fn test_inactive_mechanic() {
        // 未激活的机制不应产生效果
        let definitions = create_test_definitions();
        let states = vec![
            MechanicState {
                id: "focus_blessing".to_string(),
                current_stacks: 4,
                max_stacks: 4,
                is_active: false, // 未激活
            },
        ];

        let processor = MechanicsProcessor::new(definitions, states);
        let effects = processor.calculate_base_effects();

        assert!(effects.is_empty());
    }

    #[test]
    fn test_zero_stacks() {
        // 0 层机制不应产生效果
        let definitions = create_test_definitions();
        let states = vec![
            MechanicState {
                id: "focus_blessing".to_string(),
                current_stacks: 0, // 0 层
                max_stacks: 4,
                is_active: true,
            },
        ];

        let processor = MechanicsProcessor::new(definitions, states);
        let effects = processor.calculate_base_effects();

        assert!(effects.is_empty());
    }

    #[test]
    fn test_per_stack_attribute() {
        // 测试 .per_xxx 属性计算
        // 伊斯拉菲尔的旧律：每层聚能祝福 +14% 冰冷伤害
        let definitions = create_test_definitions();
        let states = vec![
            MechanicState {
                id: "focus_blessing".to_string(),
                current_stacks: 4,
                max_stacks: 4,
                is_active: true,
            },
        ];

        let processor = MechanicsProcessor::new(definitions, states);
        
        let result = processor.calculate_per_stack_value(
            "mod.inc.dmg.cold.per_focus_blessing",
            0.14
        );

        assert!(result.is_some());
        let (base_key, total_value) = result.unwrap();
        assert_eq!(base_key, "mod.inc.dmg.cold");
        assert!((total_value - 0.56).abs() < 0.001); // 4 层 × 14% = 56%
    }

    #[test]
    fn test_extract_mechanic_id() {
        assert_eq!(
            extract_mechanic_id("mod.inc.dmg.cold.per_focus_blessing"),
            Some("focus_blessing".to_string())
        );
        assert_eq!(
            extract_mechanic_id("crit.dmg.per_tenacity_blessing"),
            Some("tenacity_blessing".to_string())
        );
        assert_eq!(
            extract_mechanic_id("crit.chance.per_fighting_will"),
            Some("fighting_will".to_string())
        );
        assert_eq!(
            extract_mechanic_id("mod.inc.dmg.cold"),
            None
        );
    }

    #[test]
    fn test_multiple_blessings() {
        // 测试多个祝福同时激活
        let definitions = create_test_definitions();
        let states = vec![
            MechanicState {
                id: "focus_blessing".to_string(),
                current_stacks: 4,
                max_stacks: 4,
                is_active: true,
            },
            MechanicState {
                id: "agility_blessing".to_string(),
                current_stacks: 4,
                max_stacks: 4,
                is_active: true,
            },
        ];

        let processor = MechanicsProcessor::new(definitions, states);
        let effects = processor.calculate_base_effects();

        // 聚能 4层 × 4% + 灵动 4层 × 2% = 16% + 8% = 24%
        assert!((effects.get("mod.inc.dmg.all").copied().unwrap_or(0.0) - 0.24).abs() < 0.001);
        // 灵动 4层 × 4% = 16%
        assert!((effects.get("speed.attack").copied().unwrap_or(0.0) - 0.16).abs() < 0.001);
    }
    
    #[test]
    fn test_fighting_will_with_blessing() {
        // 测试战意与祝福同时激活
        let definitions = create_test_definitions();
        let states = vec![
            MechanicState {
                id: "focus_blessing".to_string(),
                current_stacks: 4,
                max_stacks: 4,
                is_active: true,
            },
            MechanicState {
                id: "fighting_will".to_string(),
                current_stacks: 25,
                max_stacks: 100,
                is_active: true,
            },
        ];

        let processor = MechanicsProcessor::new(definitions, states);
        let effects = processor.calculate_base_effects();

        // 聚能 4层 × 4% = 16% 伤害
        assert!((effects.get("mod.inc.dmg.all").copied().unwrap_or(0.0) - 0.16).abs() < 0.001);
        // 战意 25点 × 2% = 50% 暴击值
        assert!((effects.get("crit.chance.attack").copied().unwrap_or(0.0) - 0.50).abs() < 0.001);
        assert!((effects.get("crit.chance.spell").copied().unwrap_or(0.0) - 0.50).abs() < 0.001);
    }
}

