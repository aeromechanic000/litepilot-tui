use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ModelSize {
    Small,
    Medium,
    Large,
}

impl fmt::Display for ModelSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModelSize::Small => write!(f, "Small (≤5B)"),
            ModelSize::Medium => write!(f, "Medium (6-14B)"),
            ModelSize::Large => write!(f, "Large (≥30B)"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ModelInfo {
    pub name: String,
    pub size: u64,
    pub parameter_count: u64,
    pub quantization_level: String,
    pub family: String,
    pub size_class: ModelSize,
    pub context_window: u64,
}

/// Extract approximate parameter count from model name (e.g., "qwen3:4b" → 4)
#[allow(dead_code)]
pub fn estimate_parameters(name: &str) -> u64 {
    let lower = name.to_lowercase();
    // Try patterns like ":4b", ":14b", ":32b", ":0.5b"
    if let Some(rest) = lower.split(':').nth(1) {
        let digits: String = rest
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '.')
            .collect();
        if let Ok(val) = digits.parse::<f64>() {
            return val as u64;
        }
    }
    // Fallback: try size-based heuristic (roughly 1 byte per parameter in Q4)
    0
}

#[allow(dead_code)]
pub fn classify_model(parameter_count: u64) -> ModelSize {
    match parameter_count {
        0..=5 => ModelSize::Small,
        6..=14 => ModelSize::Medium,
        _ => ModelSize::Large,
    }
}

#[allow(dead_code)]
pub fn estimate_context_window(name: &str) -> u64 {
    let params = estimate_parameters(name);
    match params {
        0..=1 => 2048,
        2..=5 => 4096,
        6..=14 => 8192,
        _ => 16384,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimate_params_from_name() {
        assert_eq!(estimate_parameters("qwen3:4b"), 4);
        assert_eq!(estimate_parameters("llama3.2:1b"), 1);
        assert_eq!(estimate_parameters("deepseek-coder:14b"), 14);
        assert_eq!(estimate_parameters("qwen3:32b"), 32);
    }

    #[test]
    fn estimate_params_unknown_format() {
        assert_eq!(estimate_parameters("custom-model"), 0);
    }

    #[test]
    fn classify_small() {
        assert_eq!(classify_model(1), ModelSize::Small);
        assert_eq!(classify_model(3), ModelSize::Small);
        assert_eq!(classify_model(5), ModelSize::Small);
    }

    #[test]
    fn classify_medium() {
        assert_eq!(classify_model(7), ModelSize::Medium);
        assert_eq!(classify_model(14), ModelSize::Medium);
    }

    #[test]
    fn classify_large() {
        assert_eq!(classify_model(30), ModelSize::Large);
        assert_eq!(classify_model(70), ModelSize::Large);
    }

    #[test]
    fn context_window_estimates() {
        assert_eq!(estimate_context_window("model:1b"), 2048);
        assert_eq!(estimate_context_window("model:3b"), 4096);
        assert_eq!(estimate_context_window("model:7b"), 8192);
        assert_eq!(estimate_context_window("model:70b"), 16384);
    }

    #[test]
    fn model_size_display() {
        assert!(format!("{}", ModelSize::Small).contains("Small"));
        assert!(format!("{}", ModelSize::Medium).contains("Medium"));
        assert!(format!("{}", ModelSize::Large).contains("Large"));
    }
}
