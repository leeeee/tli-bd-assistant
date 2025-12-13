//! 条件表达式 AST 模块
//!
//! 对标 POB2 的条件系统，提供可扩展的条件表达式解析和评估。
//!
//! ## 支持的条件类型
//!
//! - **Flag**: 布尔标志检查 (`is_moving`, `cannot_crit`)
//! - **Compare**: 数值比较 (`life_percent <= 0.35`)
//! - **HasTag**: 标签检查 (`has_tag("Tag_Spell")`)
//! - **MechanicActive**: 机制激活检查 (`mechanic_active("focus_blessing")`)
//! - **MechanicStacks**: 机制层数检查 (`mechanic_stacks("fighting_will") >= 50`)
//! - **PerStat**: 每 X 点属性效果 (`per_stat("dexterity", 10)` → 返回倍数)
//! - **And/Or/Not**: 复合条件
//!
//! ## 使用示例
//!
//! ```ignore
//! let ctx = EvalContext::new()
//!     .with_flag("is_moving", true)
//!     .with_value("life_percent", 0.3)
//!     .with_mechanic("focus_blessing", 6);
//!
//! let cond = Condition::parse("is_moving && life_percent <= 0.35").unwrap();
//! assert!(cond.evaluate(&ctx));
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 比较运算符
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompareOp {
    Eq,  // ==
    Ne,  // !=
    Lt,  // <
    Le,  // <=
    Gt,  // >
    Ge,  // >=
}

impl CompareOp {
    /// 从字符串解析运算符
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "==" => Some(CompareOp::Eq),
            "!=" => Some(CompareOp::Ne),
            "<" => Some(CompareOp::Lt),
            "<=" => Some(CompareOp::Le),
            ">" => Some(CompareOp::Gt),
            ">=" => Some(CompareOp::Ge),
            _ => None,
        }
    }

    /// 评估比较
    pub fn evaluate(&self, lhs: f64, rhs: f64) -> bool {
        match self {
            CompareOp::Eq => (lhs - rhs).abs() < f64::EPSILON,
            CompareOp::Ne => (lhs - rhs).abs() >= f64::EPSILON,
            CompareOp::Lt => lhs < rhs,
            CompareOp::Le => lhs <= rhs,
            CompareOp::Gt => lhs > rhs,
            CompareOp::Ge => lhs >= rhs,
        }
    }
}

/// 条件 AST 节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Condition {
    /// 始终为真
    True,
    /// 始终为假
    False,
    /// 布尔标志检查
    Flag { key: String, expected: bool },
    /// 数值比较
    Compare { key: String, op: CompareOp, value: f64 },
    /// 标签检查
    HasTag { tag: String },
    /// 标签集合检查（任一）
    HasAnyTag { tags: Vec<String> },
    /// 标签集合检查（全部）
    HasAllTags { tags: Vec<String> },
    /// 机制激活检查
    MechanicActive { mechanic_id: String },
    /// 机制层数检查
    MechanicStacks { mechanic_id: String, op: CompareOp, value: u32 },
    /// 每 X 点属性效果（返回倍数）
    PerStat { stat: String, per: f64 },
    /// 逻辑与
    And(Box<Condition>, Box<Condition>),
    /// 逻辑或
    Or(Box<Condition>, Box<Condition>),
    /// 逻辑非
    Not(Box<Condition>),
}

impl Default for Condition {
    fn default() -> Self {
        Condition::True
    }
}

impl Condition {
    /// 解析条件表达式字符串
    ///
    /// 支持的格式:
    /// - `true` / `false`
    /// - `key == true` / `key == false`
    /// - `key <= 0.35` / `key >= 100`
    /// - `has_tag("Tag_Spell")`
    /// - `mechanic_active("focus_blessing")`
    /// - `mechanic_stacks("fighting_will") >= 50`
    /// - `per_stat("dexterity", 10)`
    /// - `cond1 && cond2` / `cond1 || cond2`
    /// - `!cond`
    pub fn parse(expr: &str) -> Result<Self, String> {
        let expr = expr.trim();

        // 空字符串或 "true" 返回 True
        if expr.is_empty() || expr == "true" {
            return Ok(Condition::True);
        }
        if expr == "false" {
            return Ok(Condition::False);
        }

        // 处理逻辑运算符（优先级：NOT > AND > OR）
        // 先处理 OR（最低优先级）
        if let Some(idx) = Self::find_logical_op(expr, "||") {
            let left = Condition::parse(&expr[..idx])?;
            let right = Condition::parse(&expr[idx + 2..])?;
            return Ok(Condition::Or(Box::new(left), Box::new(right)));
        }

        // 处理 AND
        if let Some(idx) = Self::find_logical_op(expr, "&&") {
            let left = Condition::parse(&expr[..idx])?;
            let right = Condition::parse(&expr[idx + 2..])?;
            return Ok(Condition::And(Box::new(left), Box::new(right)));
        }

        // 处理 NOT
        if expr.starts_with('!') {
            let inner = Condition::parse(&expr[1..])?;
            return Ok(Condition::Not(Box::new(inner)));
        }

        // 处理括号
        if expr.starts_with('(') && expr.ends_with(')') {
            return Condition::parse(&expr[1..expr.len() - 1]);
        }

        // 处理函数调用
        if expr.starts_with("has_tag(") {
            return Self::parse_has_tag(expr);
        }
        if expr.starts_with("has_any_tag(") {
            return Self::parse_has_any_tag(expr);
        }
        if expr.starts_with("has_all_tags(") {
            return Self::parse_has_all_tags(expr);
        }
        if expr.starts_with("mechanic_active(") {
            return Self::parse_mechanic_active(expr);
        }
        if expr.starts_with("mechanic_stacks(") {
            return Self::parse_mechanic_stacks(expr);
        }
        if expr.starts_with("per_stat(") {
            return Self::parse_per_stat(expr);
        }

        // 处理简单比较
        Self::parse_comparison(expr)
    }

    /// 查找逻辑运算符位置（跳过括号内的内容）
    fn find_logical_op(expr: &str, op: &str) -> Option<usize> {
        let mut depth = 0;
        let bytes = expr.as_bytes();
        let op_bytes = op.as_bytes();
        
        for i in 0..bytes.len() {
            match bytes[i] {
                b'(' => depth += 1,
                b')' => depth -= 1,
                _ if depth == 0 && i + op.len() <= bytes.len() => {
                    if &bytes[i..i + op.len()] == op_bytes {
                        return Some(i);
                    }
                }
                _ => {}
            }
        }
        None
    }

    /// 解析 has_tag("Tag_Spell")
    fn parse_has_tag(expr: &str) -> Result<Self, String> {
        let inner = expr
            .strip_prefix("has_tag(")
            .and_then(|s| s.strip_suffix(')'))
            .ok_or_else(|| "Invalid has_tag syntax".to_string())?;
        
        let tag = inner.trim().trim_matches('"').trim_matches('\'').to_string();
        Ok(Condition::HasTag { tag })
    }

    /// 解析 has_any_tag("Tag1", "Tag2")
    fn parse_has_any_tag(expr: &str) -> Result<Self, String> {
        let inner = expr
            .strip_prefix("has_any_tag(")
            .and_then(|s| s.strip_suffix(')'))
            .ok_or_else(|| "Invalid has_any_tag syntax".to_string())?;
        
        let tags: Vec<String> = inner
            .split(',')
            .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
            .collect();
        
        Ok(Condition::HasAnyTag { tags })
    }

    /// 解析 has_all_tags("Tag1", "Tag2")
    fn parse_has_all_tags(expr: &str) -> Result<Self, String> {
        let inner = expr
            .strip_prefix("has_all_tags(")
            .and_then(|s| s.strip_suffix(')'))
            .ok_or_else(|| "Invalid has_all_tags syntax".to_string())?;
        
        let tags: Vec<String> = inner
            .split(',')
            .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
            .collect();
        
        Ok(Condition::HasAllTags { tags })
    }

    /// 解析 mechanic_active("focus_blessing")
    fn parse_mechanic_active(expr: &str) -> Result<Self, String> {
        let inner = expr
            .strip_prefix("mechanic_active(")
            .and_then(|s| s.strip_suffix(')'))
            .ok_or_else(|| "Invalid mechanic_active syntax".to_string())?;
        
        let mechanic_id = inner.trim().trim_matches('"').trim_matches('\'').to_string();
        Ok(Condition::MechanicActive { mechanic_id })
    }

    /// 解析 mechanic_stacks("fighting_will") >= 50
    fn parse_mechanic_stacks(expr: &str) -> Result<Self, String> {
        // 找到闭括号
        let close_paren = expr.find(')').ok_or("Missing closing paren")?;
        let func_part = &expr[..=close_paren];
        let compare_part = expr[close_paren + 1..].trim();
        
        let inner = func_part
            .strip_prefix("mechanic_stacks(")
            .and_then(|s| s.strip_suffix(')'))
            .ok_or_else(|| "Invalid mechanic_stacks syntax".to_string())?;
        
        let mechanic_id = inner.trim().trim_matches('"').trim_matches('\'').to_string();
        
        // 解析比较部分
        let operators = ["<=", ">=", "!=", "==", "<", ">"];
        for op_str in operators {
            if let Some(idx) = compare_part.find(op_str) {
                let op = CompareOp::from_str(op_str).ok_or("Invalid operator")?;
                let value_str = compare_part[idx + op_str.len()..].trim();
                let value: u32 = value_str
                    .parse()
                    .map_err(|_| format!("Invalid number: {}", value_str))?;
                
                return Ok(Condition::MechanicStacks { mechanic_id, op, value });
            }
        }
        
        // 默认检查是否 > 0
        Ok(Condition::MechanicStacks {
            mechanic_id,
            op: CompareOp::Gt,
            value: 0,
        })
    }

    /// 解析 per_stat("dexterity", 10)
    fn parse_per_stat(expr: &str) -> Result<Self, String> {
        let inner = expr
            .strip_prefix("per_stat(")
            .and_then(|s| s.strip_suffix(')'))
            .ok_or_else(|| "Invalid per_stat syntax".to_string())?;
        
        let parts: Vec<&str> = inner.split(',').collect();
        if parts.len() != 2 {
            return Err("per_stat requires 2 arguments".to_string());
        }
        
        let stat = parts[0].trim().trim_matches('"').trim_matches('\'').to_string();
        let per: f64 = parts[1]
            .trim()
            .parse()
            .map_err(|_| format!("Invalid number: {}", parts[1]))?;
        
        Ok(Condition::PerStat { stat, per })
    }

    /// 解析比较表达式
    fn parse_comparison(expr: &str) -> Result<Self, String> {
        let operators = ["<=", ">=", "!=", "==", "<", ">"];
        
        for op_str in operators {
            if let Some(idx) = expr.find(op_str) {
                let key = expr[..idx].trim().to_string();
                let value_str = expr[idx + op_str.len()..].trim();
                
                // 尝试解析为布尔值
                if value_str == "true" || value_str == "false" {
                    let expected = value_str == "true";
                    if op_str == "==" {
                        return Ok(Condition::Flag { key, expected });
                    } else if op_str == "!=" {
                        return Ok(Condition::Flag { key, expected: !expected });
                    }
                }
                
                // 尝试解析为数值
                if let Ok(value) = value_str.parse::<f64>() {
                    let op = CompareOp::from_str(op_str).ok_or("Invalid operator")?;
                    return Ok(Condition::Compare { key, op, value });
                }
                
                return Err(format!("Cannot parse value: {}", value_str));
            }
        }
        
        // 单独的标识符视为 Flag 检查（true）
        Ok(Condition::Flag {
            key: expr.to_string(),
            expected: true,
        })
    }

    /// 评估条件
    pub fn evaluate(&self, ctx: &EvalContext) -> bool {
        match self {
            Condition::True => true,
            Condition::False => false,
            Condition::Flag { key, expected } => {
                ctx.flags.get(key).copied().unwrap_or(false) == *expected
            }
            Condition::Compare { key, op, value } => {
                let actual = ctx.values.get(key).copied().unwrap_or(0.0);
                op.evaluate(actual, *value)
            }
            Condition::HasTag { tag } => ctx.tags.contains(tag),
            Condition::HasAnyTag { tags } => tags.iter().any(|t| ctx.tags.contains(t)),
            Condition::HasAllTags { tags } => tags.iter().all(|t| ctx.tags.contains(t)),
            Condition::MechanicActive { mechanic_id } => {
                ctx.mechanic_stacks.get(mechanic_id).copied().unwrap_or(0) > 0
            }
            Condition::MechanicStacks { mechanic_id, op, value } => {
                let stacks = ctx.mechanic_stacks.get(mechanic_id).copied().unwrap_or(0);
                op.evaluate(stacks as f64, *value as f64)
            }
            Condition::PerStat { stat, per } => {
                // PerStat 不用于布尔评估，用于计算倍数
                // 这里返回 true 如果有任意层数
                let value = ctx.values.get(stat).copied().unwrap_or(0.0);
                value >= *per
            }
            Condition::And(left, right) => left.evaluate(ctx) && right.evaluate(ctx),
            Condition::Or(left, right) => left.evaluate(ctx) || right.evaluate(ctx),
            Condition::Not(inner) => !inner.evaluate(ctx),
        }
    }

    /// 计算 PerStat 倍数
    pub fn evaluate_multiplier(&self, ctx: &EvalContext) -> f64 {
        match self {
            Condition::PerStat { stat, per } => {
                let value = ctx.values.get(stat).copied().unwrap_or(0.0);
                (value / per).floor()
            }
            _ => 1.0,
        }
    }
}

/// 条件评估上下文
#[derive(Debug, Clone, Default)]
pub struct EvalContext {
    /// 布尔标志
    pub flags: HashMap<String, bool>,
    /// 数值属性
    pub values: HashMap<String, f64>,
    /// 当前标签集合
    pub tags: Vec<String>,
    /// 机制层数
    pub mechanic_stacks: HashMap<String, u32>,
}

impl EvalContext {
    /// 创建新的评估上下文
    pub fn new() -> Self {
        Self::default()
    }

    /// 添加标志
    pub fn with_flag(mut self, key: &str, value: bool) -> Self {
        self.flags.insert(key.to_string(), value);
        self
    }

    /// 添加数值
    pub fn with_value(mut self, key: &str, value: f64) -> Self {
        self.values.insert(key.to_string(), value);
        self
    }

    /// 添加标签
    pub fn with_tag(mut self, tag: &str) -> Self {
        self.tags.push(tag.to_string());
        self
    }

    /// 添加多个标签
    pub fn with_tags(mut self, tags: &[String]) -> Self {
        self.tags.extend(tags.iter().cloned());
        self
    }

    /// 添加机制层数
    pub fn with_mechanic(mut self, mechanic_id: &str, stacks: u32) -> Self {
        self.mechanic_stacks.insert(mechanic_id.to_string(), stacks);
        self
    }

    /// 从现有上下文数据构建
    pub fn from_context(
        flags: &HashMap<String, bool>,
        values: &HashMap<String, f64>,
    ) -> Self {
        Self {
            flags: flags.clone(),
            values: values.clone(),
            tags: vec![],
            mechanic_stacks: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_true_false() {
        assert!(matches!(Condition::parse("true").unwrap(), Condition::True));
        assert!(matches!(Condition::parse("false").unwrap(), Condition::False));
    }

    #[test]
    fn test_parse_flag() {
        let cond = Condition::parse("is_moving == true").unwrap();
        if let Condition::Flag { key, expected } = cond {
            assert_eq!(key, "is_moving");
            assert!(expected);
        } else {
            panic!("Expected Flag condition");
        }
    }

    #[test]
    fn test_parse_compare() {
        let cond = Condition::parse("life_percent <= 0.35").unwrap();
        if let Condition::Compare { key, op, value } = cond {
            assert_eq!(key, "life_percent");
            assert_eq!(op, CompareOp::Le);
            assert!((value - 0.35).abs() < 0.001);
        } else {
            panic!("Expected Compare condition");
        }
    }

    #[test]
    fn test_parse_has_tag() {
        let cond = Condition::parse("has_tag(\"Tag_Spell\")").unwrap();
        if let Condition::HasTag { tag } = cond {
            assert_eq!(tag, "Tag_Spell");
        } else {
            panic!("Expected HasTag condition");
        }
    }

    #[test]
    fn test_parse_mechanic_active() {
        let cond = Condition::parse("mechanic_active(\"focus_blessing\")").unwrap();
        if let Condition::MechanicActive { mechanic_id } = cond {
            assert_eq!(mechanic_id, "focus_blessing");
        } else {
            panic!("Expected MechanicActive condition");
        }
    }

    #[test]
    fn test_parse_mechanic_stacks() {
        let cond = Condition::parse("mechanic_stacks(\"fighting_will\") >= 50").unwrap();
        if let Condition::MechanicStacks { mechanic_id, op, value } = cond {
            assert_eq!(mechanic_id, "fighting_will");
            assert_eq!(op, CompareOp::Ge);
            assert_eq!(value, 50);
        } else {
            panic!("Expected MechanicStacks condition");
        }
    }

    #[test]
    fn test_parse_per_stat() {
        let cond = Condition::parse("per_stat(\"dexterity\", 10)").unwrap();
        if let Condition::PerStat { stat, per } = cond {
            assert_eq!(stat, "dexterity");
            assert!((per - 10.0).abs() < 0.001);
        } else {
            panic!("Expected PerStat condition");
        }
    }

    #[test]
    fn test_parse_and() {
        let cond = Condition::parse("is_moving == true && life_percent <= 0.35").unwrap();
        assert!(matches!(cond, Condition::And(_, _)));
    }

    #[test]
    fn test_parse_or() {
        let cond = Condition::parse("is_moving == true || is_stationary == true").unwrap();
        assert!(matches!(cond, Condition::Or(_, _)));
    }

    #[test]
    fn test_parse_not() {
        let cond = Condition::parse("!cannot_crit").unwrap();
        assert!(matches!(cond, Condition::Not(_)));
    }

    #[test]
    fn test_evaluate_flag() {
        let ctx = EvalContext::new().with_flag("is_moving", true);
        
        let cond = Condition::parse("is_moving == true").unwrap();
        assert!(cond.evaluate(&ctx));
        
        let cond = Condition::parse("is_moving == false").unwrap();
        assert!(!cond.evaluate(&ctx));
    }

    #[test]
    fn test_evaluate_compare() {
        let ctx = EvalContext::new().with_value("life_percent", 0.3);
        
        let cond = Condition::parse("life_percent <= 0.35").unwrap();
        assert!(cond.evaluate(&ctx));
        
        let cond = Condition::parse("life_percent >= 0.5").unwrap();
        assert!(!cond.evaluate(&ctx));
    }

    #[test]
    fn test_evaluate_has_tag() {
        let ctx = EvalContext::new().with_tag("Tag_Spell").with_tag("Tag_Lightning");
        
        let cond = Condition::parse("has_tag(\"Tag_Spell\")").unwrap();
        assert!(cond.evaluate(&ctx));
        
        let cond = Condition::parse("has_tag(\"Tag_Attack\")").unwrap();
        assert!(!cond.evaluate(&ctx));
    }

    #[test]
    fn test_evaluate_mechanic() {
        let ctx = EvalContext::new()
            .with_mechanic("focus_blessing", 6)
            .with_mechanic("fighting_will", 100);
        
        let cond = Condition::parse("mechanic_active(\"focus_blessing\")").unwrap();
        assert!(cond.evaluate(&ctx));
        
        let cond = Condition::parse("mechanic_stacks(\"fighting_will\") >= 50").unwrap();
        assert!(cond.evaluate(&ctx));
        
        let cond = Condition::parse("mechanic_stacks(\"fighting_will\") >= 150").unwrap();
        assert!(!cond.evaluate(&ctx));
    }

    #[test]
    fn test_evaluate_per_stat() {
        let ctx = EvalContext::new().with_value("dexterity", 250.0);
        
        let cond = Condition::parse("per_stat(\"dexterity\", 10)").unwrap();
        let multiplier = cond.evaluate_multiplier(&ctx);
        assert!((multiplier - 25.0).abs() < 0.001); // 250 / 10 = 25
    }

    #[test]
    fn test_evaluate_complex() {
        let ctx = EvalContext::new()
            .with_flag("is_moving", true)
            .with_value("life_percent", 0.3)
            .with_tag("Tag_Spell");
        
        // 移动中且低血
        let cond = Condition::parse("is_moving == true && life_percent <= 0.35").unwrap();
        assert!(cond.evaluate(&ctx));
        
        // 移动中或满血
        let cond = Condition::parse("is_moving == true || life_percent >= 1.0").unwrap();
        assert!(cond.evaluate(&ctx));
        
        // 不是静止状态
        let cond = Condition::parse("!is_stationary").unwrap();
        assert!(cond.evaluate(&ctx)); // is_stationary 未设置，默认 false，所以 !false = true
    }
}

