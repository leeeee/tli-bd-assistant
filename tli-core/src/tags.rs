//! 标签系统模块
//!
//! 实现 UTAS (Universal Tag & Attribute System)
//! - 标签整数化 (Interning)
//! - 继承展开
//! - BitSet 集合运算

use fixedbitset::FixedBitSet;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 标签注册表
#[derive(Debug, Clone, Default)]
pub struct TagRegistry {
    /// 标签名到 ID 的映射
    name_to_id: HashMap<String, u32>,
    /// ID 到标签名的映射
    id_to_name: HashMap<u32, String>,
    /// 标签继承关系（ID -> 父级 ID 列表）
    inheritance: HashMap<u32, Vec<u32>>,
    /// 预计算的展开集（包含自身和所有祖先）
    expanded_sets: HashMap<u32, FixedBitSet>,
    /// 最大标签 ID
    max_id: u32,
}

/// 标签定义（用于从 JSON 加载）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagDefinition {
    pub id: u32,
    pub category: String,
    #[serde(default)]
    pub parents: Vec<String>,
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(default)]
    pub condition: Option<String>,
    #[serde(default)]
    pub params: Option<HashMap<String, serde_json::Value>>,
}

impl TagRegistry {
    /// 创建新的空注册表
    pub fn new() -> Self {
        Self::default()
    }

    /// 从 JSON 加载标签注册表
    pub fn from_json(json: &str) -> Result<Self, String> {
        let raw: HashMap<String, serde_json::Value> = serde_json::from_str(json)
            .map_err(|e| format!("Failed to parse JSON: {}", e))?;
        
        let mut registry = Self::new();
        let mut parent_refs: HashMap<String, Vec<String>> = HashMap::new();
        
        // 第一遍：注册所有标签
        for (key, value) in &raw {
            // 跳过元数据
            if key.starts_with('_') {
                continue;
            }
            
            let def: TagDefinition = serde_json::from_value(value.clone())
                .map_err(|e| format!("Failed to parse tag '{}': {}", key, e))?;
            
            registry.register(key.clone(), def.id);
            parent_refs.insert(key.clone(), def.parents);
        }
        
        // 第二遍：建立继承关系
        for (tag_name, parents) in parent_refs {
            if let Some(&tag_id) = registry.name_to_id.get(&tag_name) {
                let parent_ids: Vec<u32> = parents
                    .iter()
                    .filter_map(|p| registry.name_to_id.get(p))
                    .copied()
                    .collect();
                registry.inheritance.insert(tag_id, parent_ids);
            }
        }
        
        // 预计算展开集
        registry.precompute_expanded_sets();
        
        Ok(registry)
    }

    /// 注册单个标签
    pub fn register(&mut self, name: String, id: u32) {
        self.name_to_id.insert(name.clone(), id);
        self.id_to_name.insert(id, name);
        self.max_id = self.max_id.max(id);
    }

    /// 设置标签继承关系
    pub fn set_parents(&mut self, tag_id: u32, parent_ids: Vec<u32>) {
        self.inheritance.insert(tag_id, parent_ids);
    }

    /// 预计算所有标签的展开集
    pub fn precompute_expanded_sets(&mut self) {
        let capacity = (self.max_id + 1) as usize;
        
        for &tag_id in self.id_to_name.keys() {
            let expanded = self.compute_expanded_set(tag_id, capacity);
            self.expanded_sets.insert(tag_id, expanded);
        }
    }

    /// 递归计算单个标签的展开集
    fn compute_expanded_set(&self, tag_id: u32, capacity: usize) -> FixedBitSet {
        let mut result = FixedBitSet::with_capacity(capacity);
        result.insert(tag_id as usize);
        
        if let Some(parents) = self.inheritance.get(&tag_id) {
            for &parent_id in parents {
                result.insert(parent_id as usize);
                // 递归获取祖先
                let parent_expanded = self.compute_expanded_set(parent_id, capacity);
                result.union_with(&parent_expanded);
            }
        }
        
        result
    }

    /// 获取标签 ID
    pub fn get_id(&self, name: &str) -> Option<u32> {
        self.name_to_id.get(name).copied()
    }

    /// 获取标签名
    pub fn get_name(&self, id: u32) -> Option<&str> {
        self.id_to_name.get(&id).map(|s| s.as_str())
    }

    /// 获取标签的展开集（包含自身和所有祖先）
    pub fn get_expanded_set(&self, tag_id: u32) -> Option<&FixedBitSet> {
        self.expanded_sets.get(&tag_id)
    }

    /// 创建空的标签集合
    pub fn create_empty_set(&self) -> FixedBitSet {
        FixedBitSet::with_capacity((self.max_id + 1) as usize)
    }

    /// 从标签名列表创建标签集合
    pub fn create_set_from_names(&self, names: &[String]) -> FixedBitSet {
        let mut set = self.create_empty_set();
        for name in names {
            if let Some(id) = self.get_id(name) {
                set.insert(id as usize);
                // 自动展开继承
                if let Some(expanded) = self.get_expanded_set(id) {
                    set.union_with(expanded);
                }
            }
        }
        set
    }

    /// 从标签 ID 列表创建标签集合
    pub fn create_set_from_ids(&self, ids: &[u32]) -> FixedBitSet {
        let mut set = self.create_empty_set();
        for &id in ids {
            set.insert(id as usize);
            if let Some(expanded) = self.get_expanded_set(id) {
                set.union_with(expanded);
            }
        }
        set
    }

    /// 获取标签数量
    pub fn len(&self) -> usize {
        self.name_to_id.len()
    }

    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        self.name_to_id.is_empty()
    }

    /// 获取最大 ID
    pub fn max_id(&self) -> u32 {
        self.max_id
    }
}

/// 标签集合操作
pub struct TagSet {
    bits: FixedBitSet,
}

impl TagSet {
    /// 从 FixedBitSet 创建
    pub fn from_bitset(bits: FixedBitSet) -> Self {
        Self { bits }
    }

    /// 创建空集合
    pub fn empty(capacity: usize) -> Self {
        Self {
            bits: FixedBitSet::with_capacity(capacity),
        }
    }

    /// 插入标签
    pub fn insert(&mut self, tag_id: u32) {
        self.bits.insert(tag_id as usize);
    }

    /// 移除标签
    pub fn remove(&mut self, tag_id: u32) {
        self.bits.set(tag_id as usize, false);
    }

    /// 检查是否包含标签
    pub fn contains(&self, tag_id: u32) -> bool {
        self.bits.contains(tag_id as usize)
    }

    /// 检查是否包含所有指定标签
    pub fn contains_all(&self, requirements: &FixedBitSet) -> bool {
        requirements.is_subset(&self.bits)
    }

    /// 检查是否包含任一指定标签
    pub fn contains_any(&self, tags: &FixedBitSet) -> bool {
        !self.bits.intersection(tags).next().is_none()
    }

    /// 合并另一个集合
    pub fn union_with(&mut self, other: &TagSet) {
        self.bits.union_with(&other.bits);
    }

    /// 合并 FixedBitSet
    pub fn union_with_bits(&mut self, other: &FixedBitSet) {
        self.bits.union_with(other);
    }

    /// 获取内部 BitSet 引用
    pub fn bits(&self) -> &FixedBitSet {
        &self.bits
    }

    /// 获取内部 BitSet 可变引用
    pub fn bits_mut(&mut self) -> &mut FixedBitSet {
        &mut self.bits
    }

    /// 迭代所有标签 ID
    pub fn iter(&self) -> impl Iterator<Item = u32> + '_ {
        self.bits.ones().map(|i| i as u32)
    }
}

impl Clone for TagSet {
    fn clone(&self) -> Self {
        Self {
            bits: self.bits.clone(),
        }
    }
}

/// 上下文标签管理器
pub struct ContextTags {
    /// 当前活动的标签集合
    active: TagSet,
    /// 标签注册表引用
    registry: TagRegistry,
}

impl ContextTags {
    /// 创建新的上下文标签管理器
    pub fn new(registry: TagRegistry) -> Self {
        let capacity = (registry.max_id() + 1) as usize;
        Self {
            active: TagSet::empty(capacity),
            registry,
        }
    }

    /// 从技能注入静态标签
    pub fn inject_skill_tags(&mut self, tags: &[String]) {
        for tag in tags {
            if let Some(id) = self.registry.get_id(tag) {
                self.active.insert(id);
                // 展开继承
                if let Some(expanded) = self.registry.get_expanded_set(id) {
                    self.active.union_with_bits(expanded);
                }
            }
        }
    }

    /// 从辅助技能注入标签
    pub fn inject_support_tags(&mut self, injected_tags: &[String]) {
        self.inject_skill_tags(injected_tags);
    }

    /// 根据上下文标志注入状态标签
    pub fn inject_context_flags(&mut self, flags: &HashMap<String, bool>) {
        // 状态标签映射
        let state_mappings = [
            ("is_moving", "Tag_State_Moving", "Tag_State_Stationary"),
            ("low_life", "Tag_State_Low_Life", ""),
            ("full_life", "Tag_State_Full_Life", ""),
            ("recently_crit", "Tag_State_Recently_Crit", ""),
            ("recently_killed", "Tag_State_Recently_Killed", ""),
            ("enemy_chilled", "Tag_State_Enemy_Chilled", ""),
            ("enemy_frozen", "Tag_State_Enemy_Frozen", ""),
            ("enemy_shocked", "Tag_State_Enemy_Shocked", ""),
            ("enemy_ignited", "Tag_State_Enemy_Ignited", ""),
            ("enemy_controlled", "Tag_State_Enemy_Controlled", ""),
        ];

        for (flag_key, true_tag, false_tag) in state_mappings {
            if let Some(&value) = flags.get(flag_key) {
                let tag_to_add = if value { true_tag } else { false_tag };
                if !tag_to_add.is_empty() {
                    if let Some(id) = self.registry.get_id(tag_to_add) {
                        self.active.insert(id);
                    }
                }
            }
        }
    }

    /// 检查修正是否满足条件
    pub fn matches_requirements(&self, requirements: &[u32]) -> bool {
        if requirements.is_empty() {
            return true;
        }
        
        // 创建需求集合
        let mut req_set = self.registry.create_empty_set();
        for &id in requirements {
            req_set.insert(id as usize);
        }
        
        self.active.contains_all(&req_set)
    }

    /// 获取当前活动标签集合
    pub fn active_set(&self) -> &TagSet {
        &self.active
    }

    /// 获取注册表引用
    pub fn registry(&self) -> &TagRegistry {
        &self.registry
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_registry() -> TagRegistry {
        let mut registry = TagRegistry::new();
        
        // 注册基础标签
        registry.register("Tag_Damage".to_string(), 1);
        registry.register("Tag_Elemental".to_string(), 20);
        registry.register("Tag_Fire".to_string(), 21);
        registry.register("Tag_Physical".to_string(), 10);
        
        // 设置继承
        registry.set_parents(20, vec![1]); // Elemental -> Damage
        registry.set_parents(21, vec![20]); // Fire -> Elemental
        registry.set_parents(10, vec![1]); // Physical -> Damage
        
        // 预计算
        registry.precompute_expanded_sets();
        
        registry
    }

    #[test]
    fn test_tag_inheritance() {
        let registry = create_test_registry();
        
        // Fire 应该展开为 [Fire, Elemental, Damage]
        let fire_expanded = registry.get_expanded_set(21).unwrap();
        assert!(fire_expanded.contains(21)); // Fire
        assert!(fire_expanded.contains(20)); // Elemental
        assert!(fire_expanded.contains(1));  // Damage
    }

    #[test]
    fn test_tag_set_operations() {
        let registry = create_test_registry();
        
        let set = registry.create_set_from_names(&[
            "Tag_Fire".to_string(),
        ]);
        
        let tag_set = TagSet::from_bitset(set);
        
        // 应该包含 Fire 及其所有祖先
        assert!(tag_set.contains(21)); // Fire
        assert!(tag_set.contains(20)); // Elemental
        assert!(tag_set.contains(1));  // Damage
    }

    #[test]
    fn test_requirements_matching() {
        let registry = create_test_registry();
        let mut ctx = ContextTags::new(registry);
        
        ctx.inject_skill_tags(&["Tag_Fire".to_string()]);
        
        // Fire 技能应该满足 Elemental 需求
        assert!(ctx.matches_requirements(&[20])); // Elemental
        // 也应该满足 Damage 需求
        assert!(ctx.matches_requirements(&[1])); // Damage
        // 但不应该满足 Physical 需求
        assert!(!ctx.matches_requirements(&[10])); // Physical
    }
}

