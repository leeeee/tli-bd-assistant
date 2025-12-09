//! 工具函数模块

use wasm_bindgen::prelude::*;

/// 设置 panic hook（用于调试）
pub fn set_panic_hook() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

/// 浮点数近似比较
pub fn approx_eq(a: f64, b: f64, epsilon: f64) -> bool {
    (a - b).abs() < epsilon
}

/// 百分比格式化
pub fn format_percent(value: f64) -> String {
    format!("{:.1}%", value * 100.0)
}

/// 数值格式化（大数使用 K/M 后缀）
pub fn format_number(value: f64) -> String {
    if value >= 1_000_000.0 {
        format!("{:.2}M", value / 1_000_000.0)
    } else if value >= 1_000.0 {
        format!("{:.2}K", value / 1_000.0)
    } else {
        format!("{:.1}", value)
    }
}

/// 限制数值范围
pub fn clamp(value: f64, min: f64, max: f64) -> f64 {
    value.max(min).min(max)
}

/// 线性插值
pub fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

/// 日志输出到控制台
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    pub fn log(s: &str);
    
    #[wasm_bindgen(js_namespace = console)]
    pub fn warn(s: &str);
    
    #[wasm_bindgen(js_namespace = console)]
    pub fn error(s: &str);
}

/// 调试日志宏
#[macro_export]
macro_rules! console_log {
    ($($arg:tt)*) => {
        $crate::utils::log(&format!($($arg)*))
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_approx_eq() {
        assert!(approx_eq(1.0, 1.0000001, 0.001));
        assert!(!approx_eq(1.0, 1.1, 0.001));
    }

    #[test]
    fn test_format_number() {
        assert_eq!(format_number(500.0), "500.0");
        assert_eq!(format_number(1500.0), "1.50K");
        assert_eq!(format_number(1500000.0), "1.50M");
    }

    #[test]
    fn test_clamp() {
        assert_eq!(clamp(5.0, 0.0, 10.0), 5.0);
        assert_eq!(clamp(-5.0, 0.0, 10.0), 0.0);
        assert_eq!(clamp(15.0, 0.0, 10.0), 10.0);
    }
}

