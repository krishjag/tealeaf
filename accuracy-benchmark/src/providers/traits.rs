//! Provider trait definitions for LLM API clients

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::runner::rate_limiter::RateLimiter;

/// A message in a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

impl Message {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.into(),
        }
    }

    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".to_string(),
            content: content.into(),
        }
    }
}

/// Request for a completion from an LLM provider
#[derive(Debug, Clone)]
pub struct CompletionRequest {
    pub model: Option<String>,
    pub messages: Vec<Message>,
    pub max_tokens: u32,
    pub temperature: Option<f32>,
    pub system_prompt: Option<String>,
}

impl CompletionRequest {
    pub fn new(messages: Vec<Message>, max_tokens: u32) -> Self {
        Self {
            model: None,
            messages,
            max_tokens,
            temperature: None,
            system_prompt: None,
        }
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    pub fn with_temperature(mut self, temp: f32) -> Self {
        self.temperature = Some(temp);
        self
    }

    pub fn with_system(mut self, system: impl Into<String>) -> Self {
        self.system_prompt = Some(system.into());
        self
    }
}

/// Response from an LLM provider
#[derive(Debug, Clone)]
pub struct CompletionResponse {
    pub content: String,
    pub model: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub finish_reason: String,
    pub latency_ms: u64,
    pub http_status: u16,
}

/// Error types for provider operations
#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("API error: {status} - {message}")]
    Api { status: u16, message: String },

    #[error("Rate limited: retry after {retry_after_ms}ms")]
    RateLimited { retry_after_ms: u64 },

    #[error("Timeout after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Configuration error: {0}")]
    Config(String),
}

pub type ProviderResult<T> = Result<T, ProviderError>;

/// Trait for LLM providers
#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// Get the provider name (e.g., "anthropic", "openai", "grok")
    fn name(&self) -> &str;

    /// Get the default model for this provider
    fn default_model(&self) -> &str;

    /// Send a completion request
    async fn complete(&self, request: &CompletionRequest) -> ProviderResult<CompletionResponse>;

    /// Get the rate limiter for this provider
    fn rate_limiter(&self) -> &Arc<RateLimiter>;

    /// Check if the provider is healthy/accessible
    async fn health_check(&self) -> ProviderResult<bool>;
}

/// Configuration for a provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub name: String,
    pub api_key_env: String,
    pub base_url: String,
    pub default_model: String,
    pub rate_limit_rpm: u32,
    pub rate_limit_tpm: u32,
    pub timeout_ms: u64,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            api_key_env: String::new(),
            base_url: String::new(),
            default_model: String::new(),
            rate_limit_rpm: 60,
            rate_limit_tpm: 100_000,
            timeout_ms: 60_000,
        }
    }
}
