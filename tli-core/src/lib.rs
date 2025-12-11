//! TLI Core - 火炬之光：无限 BD 决策辅助系统计算引擎
//!
//! 本模块提供完整的 DPS/EHP 计算管线，包括：
//! - 标签系统 (UTAS)
//! - 属性聚合
//! - 机制系统 (祝福、球类等)
//! - 伤害转化与标签记忆
//! - 暴击与减伤计算
//! - LRU 缓存优化 (悬停预览加速)

use std::cell::RefCell;
use wasm_bindgen::prelude::*;

pub mod types;
pub mod tags;
pub mod stats;
pub mod mechanics;
pub mod conversion;
pub mod pipeline;
pub mod calculator_cache;
pub mod utils;

pub use types::*;
pub use tags::*;
pub use stats::*;
pub use mechanics::*;
pub use conversion::*;
pub use pipeline::*;
pub use calculator_cache::*;

// WASM 环境中使用 thread_local 维护全局缓存
// 注意：WASM 是单线程的，所以这是安全的
thread_local! {
    static GLOBAL_CACHE: RefCell<CachedCalculator> = RefCell::new(CachedCalculator::new(128));
}

/// WASM 初始化
#[wasm_bindgen(start)]
pub fn init() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

/// 主计算入口点（无缓存）
#[wasm_bindgen]
pub fn calculate(input_json: &str) -> Result<String, JsValue> {
    let input: CalculatorInput = serde_json::from_str(input_json)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse input: {}", e)))?;
    
    let result = pipeline::calculate_dps(&input)
        .map_err(|e| JsValue::from_str(&format!("Calculation error: {}", e)))?;
    
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize result: {}", e)))
}

/// 带缓存的计算入口点
/// 
/// 使用 LRU 缓存优化重复计算场景（如悬停预览）
/// 相同输入会直接返回缓存结果
#[wasm_bindgen]
pub fn calculate_cached(input_json: &str) -> Result<String, JsValue> {
    let input: CalculatorInput = serde_json::from_str(input_json)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse input: {}", e)))?;
    
    let result = GLOBAL_CACHE.with(|cache| {
        cache.borrow_mut().calculate(&input)
    }).map_err(|e| JsValue::from_str(&format!("Calculation error: {}", e)))?;
    
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize result: {}", e)))
}

/// 计算预览差异
/// 
/// 用于悬停预览场景：返回装备更换前后的 DPS/EHP 差异
/// 
/// # Arguments
/// * `base_json` - 当前配置 JSON
/// * `preview_json` - 预览配置 JSON (包含新装备)
/// 
/// # Returns
/// JSON 格式的差异结果
#[wasm_bindgen]
pub fn calculate_diff(base_json: &str, preview_json: &str) -> Result<String, JsValue> {
    let base_input: CalculatorInput = serde_json::from_str(base_json)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse base input: {}", e)))?;
    
    let preview_input: CalculatorInput = serde_json::from_str(preview_json)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse preview input: {}", e)))?;
    
    let diff = GLOBAL_CACHE.with(|cache| {
        cache.borrow_mut().calculate_diff(&base_input, &preview_input)
    }).map_err(|e| JsValue::from_str(&format!("Calculation error: {}", e)))?;
    
    // 构建简化的差异输出
    let output = serde_json::json!({
        "dps_diff": diff.dps_diff,
        "dps_diff_percent": diff.dps_diff_percent,
        "dps_diff_formatted": diff.format_dps_diff(),
        "is_positive": diff.is_positive(),
        "ehp_physical_diff": diff.ehp_physical_diff,
        "crit_chance_diff": diff.crit_chance_diff,
        "base_dps": diff.base.dps_theoretical,
        "preview_dps": diff.preview.dps_theoretical,
    });
    
    serde_json::to_string(&output)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize diff: {}", e)))
}

/// 获取缓存统计信息
#[wasm_bindgen]
pub fn get_cache_stats() -> String {
    GLOBAL_CACHE.with(|cache| {
        let stats = cache.borrow().get_stats();
        serde_json::json!({
            "capacity": stats.capacity,
            "size": stats.size,
            "hits": stats.hits,
            "misses": stats.misses,
            "hit_rate": format!("{:.1}%", stats.hit_rate * 100.0),
        }).to_string()
    })
}

/// 清空计算缓存
#[wasm_bindgen]
pub fn clear_cache() {
    GLOBAL_CACHE.with(|cache| {
        cache.borrow_mut().clear_cache();
    });
}

/// 获取版本信息
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

