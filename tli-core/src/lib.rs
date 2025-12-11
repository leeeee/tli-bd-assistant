//! TLI Core - 火炬之光：无限 BD 决策辅助系统计算引擎
//!
//! 本模块提供完整的 DPS/EHP 计算管线，包括：
//! - 标签系统 (UTAS)
//! - 属性聚合
//! - 机制系统 (祝福、球类等)
//! - 伤害转化与标签记忆
//! - 暴击与减伤计算

use wasm_bindgen::prelude::*;

pub mod types;
pub mod tags;
pub mod stats;
pub mod mechanics;
pub mod conversion;
pub mod pipeline;
pub mod utils;

pub use types::*;
pub use tags::*;
pub use stats::*;
pub use mechanics::*;
pub use conversion::*;
pub use pipeline::*;

/// WASM 初始化
#[wasm_bindgen(start)]
pub fn init() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

/// 主计算入口点
#[wasm_bindgen]
pub fn calculate(input_json: &str) -> Result<String, JsValue> {
    let input: CalculatorInput = serde_json::from_str(input_json)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse input: {}", e)))?;
    
    let result = pipeline::calculate_dps(&input)
        .map_err(|e| JsValue::from_str(&format!("Calculation error: {}", e)))?;
    
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize result: {}", e)))
}

/// 获取版本信息
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

