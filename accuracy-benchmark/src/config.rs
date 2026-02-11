//! Configuration management for accuracy benchmark suite
//!
//! Loads model configurations from TOML files and provides runtime access.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Root configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub providers: HashMap<String, ProviderConfig>,
    #[serde(default)]
    pub benchmark: BenchmarkConfig,
}

/// Provider-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub name: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub default_model: String,
    /// Requests per minute
    #[serde(default = "default_rpm")]
    pub rpm: u32,
    /// Tokens per minute
    #[serde(default = "default_tpm")]
    pub tpm: u32,
    /// Model definitions
    #[serde(default)]
    pub models: HashMap<String, ModelConfig>,
    /// Extended thinking (Anthropic)
    #[serde(default)]
    pub enable_thinking: bool,
    #[serde(default = "default_thinking_budget")]
    pub thinking_budget: u32,
    /// Reasoning effort for o3 models (OpenAI)
    #[serde(default = "default_reasoning_effort")]
    pub reasoning_effort: String,
}

/// Model-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub id: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default = "default_max_output_tokens")]
    pub max_output_tokens: u32,
    /// Whether this model uses max_completion_tokens instead of max_tokens
    #[serde(default)]
    pub uses_completion_tokens: bool,
    /// Whether this model supports temperature parameter
    #[serde(default = "default_true")]
    pub supports_temperature: bool,
    /// Whether this model supports extended thinking
    #[serde(default)]
    pub supports_thinking: bool,
    /// Whether this is a reasoning model (o1, o3)
    #[serde(default)]
    pub is_reasoning_model: bool,
}

/// Benchmark execution settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkConfig {
    #[serde(default = "default_parallel_requests")]
    pub parallel_requests: usize,
    #[serde(default = "default_retry_count")]
    pub retry_count: u32,
    #[serde(default = "default_retry_delay_ms")]
    pub retry_delay_ms: u64,
    #[serde(default = "default_max_retry_delay_ms")]
    pub max_retry_delay_ms: u64,
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
    #[serde(default)]
    pub format_comparison: FormatComparisonConfig,
    #[serde(default)]
    pub output: OutputConfig,
}

/// Format comparison settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatComparisonConfig {
    /// Enable format comparison mode
    #[serde(default)]
    pub enabled: bool,
    /// Compare TeaLeaf vs JSON formats
    #[serde(default)]
    pub compare_formats: bool,
}

impl Default for FormatComparisonConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            compare_formats: false,
        }
    }
}

/// Output configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    #[serde(default = "default_output_dir")]
    pub output_dir: String,
    #[serde(default = "default_true")]
    pub save_responses: bool,
    #[serde(default = "default_true")]
    pub generate_tl: bool,
    #[serde(default = "default_true")]
    pub generate_json: bool,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            output_dir: default_output_dir(),
            save_responses: true,
            generate_tl: true,
            generate_json: true,
        }
    }
}

// Default value functions
fn default_true() -> bool { true }
fn default_rpm() -> u32 { 60 }
fn default_tpm() -> u32 { 100_000 }
fn default_thinking_budget() -> u32 { 10_000 }
fn default_reasoning_effort() -> String { "medium".to_string() }
fn default_max_output_tokens() -> u32 { 4096 }
fn default_parallel_requests() -> usize { 3 }
fn default_retry_count() -> u32 { 3 }
fn default_retry_delay_ms() -> u64 { 1000 }
fn default_max_retry_delay_ms() -> u64 { 60_000 }
fn default_timeout_ms() -> u64 { 120_000 }
fn default_output_dir() -> String { "results/runs".to_string() }

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            parallel_requests: default_parallel_requests(),
            retry_count: default_retry_count(),
            retry_delay_ms: default_retry_delay_ms(),
            max_retry_delay_ms: default_max_retry_delay_ms(),
            timeout_ms: default_timeout_ms(),
            format_comparison: FormatComparisonConfig::default(),
            output: OutputConfig::default(),
        }
    }
}

impl Config {
    /// Load configuration from a TOML file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let content = fs::read_to_string(path.as_ref())
            .map_err(|e| ConfigError::Io(e.to_string()))?;
        Self::from_toml(&content)
    }

    /// Parse configuration from a TOML string
    pub fn from_toml(content: &str) -> Result<Self, ConfigError> {
        toml::from_str(content)
            .map_err(|e| ConfigError::Parse(e.to_string()))
    }

    /// Parse configuration from a JSON string (legacy support)
    pub fn from_json(content: &str) -> Result<Self, ConfigError> {
        serde_json::from_str(content)
            .map_err(|e| ConfigError::Parse(e.to_string()))
    }

    /// Load from default config location or return defaults
    pub fn load_or_default() -> Self {
        // Try loading from config/models.toml relative to current dir
        let config_paths = [
            "config/models.toml",
            "../config/models.toml",
            "accuracy-benchmark/config/models.toml",
        ];

        for path in &config_paths {
            if let Ok(config) = Self::from_file(path) {
                tracing::info!("Loaded configuration from {}", path);
                return config;
            }
        }

        tracing::info!("Using default configuration");
        Self::default()
    }

    /// Save configuration to a TOML file
    pub fn save_toml<P: AsRef<Path>>(&self, path: P) -> Result<(), ConfigError> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| ConfigError::Parse(e.to_string()))?;
        fs::write(path, content)
            .map_err(|e| ConfigError::Io(e.to_string()))?;
        Ok(())
    }

    /// Get enabled providers
    pub fn enabled_providers(&self) -> Vec<&ProviderConfig> {
        self.providers
            .values()
            .filter(|p| p.enabled)
            .collect()
    }

    /// Get a specific provider config
    pub fn get_provider(&self, name: &str) -> Option<&ProviderConfig> {
        self.providers.get(name)
    }

    /// Check if format comparison is enabled
    pub fn format_comparison_enabled(&self) -> bool {
        self.benchmark.format_comparison.enabled && self.benchmark.format_comparison.compare_formats
    }
}

impl Default for Config {
    fn default() -> Self {
        let mut providers = HashMap::new();

        // Default Anthropic config
        let mut anthropic_models = HashMap::new();
        anthropic_models.insert("sonnet-4-5".to_string(), ModelConfig {
            id: "claude-sonnet-4-5-20250929".to_string(),
            display_name: "Claude Sonnet 4.5".to_string(),
            max_output_tokens: 8192,
            uses_completion_tokens: false,
            supports_temperature: true,
            supports_thinking: true,
            is_reasoning_model: false,
        });
        anthropic_models.insert("opus-4-5".to_string(), ModelConfig {
            id: "claude-opus-4-5-20251101".to_string(),
            display_name: "Claude Opus 4.5".to_string(),
            max_output_tokens: 8192,
            uses_completion_tokens: false,
            supports_temperature: true,
            supports_thinking: true,
            is_reasoning_model: false,
        });
        anthropic_models.insert("sonnet-4".to_string(), ModelConfig {
            id: "claude-sonnet-4-20250514".to_string(),
            display_name: "Claude Sonnet 4".to_string(),
            max_output_tokens: 8192,
            uses_completion_tokens: false,
            supports_temperature: true,
            supports_thinking: true,
            is_reasoning_model: false,
        });

        providers.insert("anthropic".to_string(), ProviderConfig {
            name: "anthropic".to_string(),
            enabled: true,
            default_model: "claude-sonnet-4-5-20250929".to_string(),
            rpm: 1_000,
            tpm: 450_000,
            models: anthropic_models,
            enable_thinking: true,
            thinking_budget: 10_000,
            reasoning_effort: String::new(),
        });

        // Default OpenAI config
        let mut openai_models = HashMap::new();
        openai_models.insert("gpt-5-2".to_string(), ModelConfig {
            id: "gpt-5.2".to_string(),
            display_name: "GPT-5.2".to_string(),
            max_output_tokens: 16384,
            uses_completion_tokens: true,
            supports_temperature: true,
            supports_thinking: false,
            is_reasoning_model: false,
        });
        openai_models.insert("gpt-4-turbo".to_string(), ModelConfig {
            id: "gpt-4-turbo".to_string(),
            display_name: "GPT-4 Turbo".to_string(),
            max_output_tokens: 4096,
            uses_completion_tokens: false,
            supports_temperature: true,
            supports_thinking: false,
            is_reasoning_model: false,
        });
        openai_models.insert("o3-mini".to_string(), ModelConfig {
            id: "o3-mini-2025-01-31".to_string(),
            display_name: "o3-mini".to_string(),
            max_output_tokens: 16384,
            uses_completion_tokens: true,
            supports_temperature: false,
            supports_thinking: false,
            is_reasoning_model: true,
        });

        providers.insert("openai".to_string(), ProviderConfig {
            name: "openai".to_string(),
            enabled: true,
            default_model: "gpt-5.2".to_string(),
            rpm: 500,
            tpm: 200_000,
            models: openai_models,
            enable_thinking: false,
            thinking_budget: 0,
            reasoning_effort: "medium".to_string(),
        });

        Self {
            providers,
            benchmark: BenchmarkConfig::default(),
        }
    }
}

impl ProviderConfig {
    /// Get model config by ID
    pub fn get_model(&self, model_id: &str) -> Option<&ModelConfig> {
        self.models.values().find(|m| m.id == model_id)
    }

    /// Get the default model config
    pub fn default_model_config(&self) -> Option<&ModelConfig> {
        self.get_model(&self.default_model)
    }
}

/// Configuration errors
#[derive(Debug, Clone)]
pub enum ConfigError {
    Io(String),
    Parse(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::Io(e) => write!(f, "IO error: {}", e),
            ConfigError::Parse(e) => write!(f, "Parse error: {}", e),
        }
    }
}

impl std::error::Error for ConfigError {}

/// Data format for benchmark tasks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DataFormat {
    /// TeaLeaf format (structured, schema-aware)
    TL,
    /// JSON format (raw)
    Json,
    /// TOON format (Token-Oriented Object Notation)
    Toon,
}

impl DataFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            DataFormat::TL => "tl",
            DataFormat::Json => "json",
            DataFormat::Toon => "toon",
        }
    }

    /// Human-readable name for use in prompt templates (e.g., "TeaLeaf", "JSON", "TOON")
    pub fn display_name(&self) -> &'static str {
        match self {
            DataFormat::TL => "TeaLeaf",
            DataFormat::Json => "JSON",
            DataFormat::Toon => "TOON",
        }
    }

    pub fn all() -> Vec<DataFormat> {
        vec![DataFormat::TL, DataFormat::Json, DataFormat::Toon]
    }
}

impl std::fmt::Display for DataFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert!(config.providers.contains_key("anthropic"));
        assert!(config.providers.contains_key("openai"));
    }

    #[test]
    fn test_parse_toml_config() {
        let toml = r#"
[providers.test]
name = "test"
enabled = true
default_model = "test-model"
rpm = 100
tpm = 50000

[providers.test.models.default]
id = "test-model"
display_name = "Test Model"
max_output_tokens = 4096
"#;
        let config = Config::from_toml(toml).unwrap();
        assert!(config.providers.contains_key("test"));
        assert_eq!(config.providers["test"].default_model, "test-model");
    }

    #[test]
    fn test_format_comparison() {
        let config = Config::default();
        assert!(!config.format_comparison_enabled());
    }
}
