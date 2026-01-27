//! Anthropic (Claude) API client with Extended Thinking

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

use super::traits::{
    CompletionRequest, CompletionResponse, LLMProvider, Message, ProviderError, ProviderResult,
};
use crate::runner::rate_limiter::RateLimiter;

const DEFAULT_MODEL: &str = "claude-opus-4-5-20251101";
const API_VERSION: &str = "2023-06-01";
const DEFAULT_THINKING_BUDGET: u32 = 10000;

/// Anthropic API client with Extended Thinking support
pub struct AnthropicClient {
    api_key: String,
    base_url: String,
    http_client: Client,
    rate_limiter: Arc<RateLimiter>,
    default_model: String,
    /// Enable extended thinking (requires compatible model)
    enable_thinking: bool,
    /// Token budget for thinking (default: 10000)
    thinking_budget: u32,
}

impl AnthropicClient {
    /// Create a new Anthropic client with extended thinking enabled
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            base_url: "https://api.anthropic.com/v1".to_string(),
            http_client: Client::new(),
            rate_limiter: Arc::new(RateLimiter::new(60, 100_000)),
            default_model: DEFAULT_MODEL.to_string(),
            enable_thinking: true,
            thinking_budget: DEFAULT_THINKING_BUDGET,
        }
    }

    /// Create from environment variable
    pub fn from_env() -> ProviderResult<Self> {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .map_err(|_| ProviderError::Config("ANTHROPIC_API_KEY not set".to_string()))?;
        Ok(Self::new(api_key))
    }

    /// Set custom base URL
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Set custom rate limits
    pub fn with_rate_limits(mut self, rpm: u32, tpm: u32) -> Self {
        self.rate_limiter = Arc::new(RateLimiter::new(rpm, tpm));
        self
    }

    /// Set default model
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.default_model = model.into();
        self
    }

    /// Enable or disable extended thinking
    pub fn with_thinking(mut self, enabled: bool) -> Self {
        self.enable_thinking = enabled;
        self
    }

    /// Set thinking token budget
    pub fn with_thinking_budget(mut self, budget: u32) -> Self {
        self.thinking_budget = budget;
        self
    }
}

#[derive(Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    /// Extended thinking configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    thinking: Option<ThinkingConfig>,
}

#[derive(Serialize)]
struct ThinkingConfig {
    #[serde(rename = "type")]
    thinking_type: String,
    budget_tokens: u32,
}

#[derive(Serialize, Deserialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

impl From<&Message> for AnthropicMessage {
    fn from(msg: &Message) -> Self {
        Self {
            role: msg.role.clone(),
            content: msg.content.clone(),
        }
    }
}

#[derive(Deserialize)]
struct AnthropicResponse {
    content: Vec<ContentBlock>,
    model: String,
    stop_reason: Option<String>,
    usage: Usage,
}

#[derive(Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    content_type: String,
    /// Text content (for "text" blocks)
    text: Option<String>,
    /// Thinking content (for "thinking" blocks) - kept for deserialization
    #[allow(dead_code)]
    thinking: Option<String>,
}

#[derive(Deserialize)]
struct Usage {
    input_tokens: u32,
    output_tokens: u32,
}

#[derive(Deserialize)]
struct AnthropicError {
    error: ErrorDetail,
}

#[derive(Deserialize)]
struct ErrorDetail {
    message: String,
    #[serde(rename = "type")]
    #[allow(dead_code)]
    error_type: String,
}

#[async_trait]
impl LLMProvider for AnthropicClient {
    fn name(&self) -> &str {
        "anthropic"
    }

    fn default_model(&self) -> &str {
        &self.default_model
    }

    async fn complete(&self, request: &CompletionRequest) -> ProviderResult<CompletionResponse> {
        // Acquire rate limit permission
        let _guard = self.rate_limiter.acquire().await;

        let start = Instant::now();

        // Filter out system messages and extract system prompt
        let system_prompt = request.system_prompt.clone().or_else(|| {
            request
                .messages
                .iter()
                .find(|m| m.role == "system")
                .map(|m| m.content.clone())
        });

        let messages: Vec<AnthropicMessage> = request
            .messages
            .iter()
            .filter(|m| m.role != "system")
            .map(|m| m.into())
            .collect();

        // Build thinking config if enabled
        // Note: Extended thinking requires temperature to be unset or 1.0
        let (thinking, temperature) = if self.enable_thinking {
            (
                Some(ThinkingConfig {
                    thinking_type: "enabled".to_string(),
                    budget_tokens: self.thinking_budget,
                }),
                None, // Temperature must be omitted with extended thinking
            )
        } else {
            (None, request.temperature)
        };

        let body = AnthropicRequest {
            model: request
                .model
                .clone()
                .unwrap_or_else(|| self.default_model.clone()),
            max_tokens: request.max_tokens + self.thinking_budget, // Include thinking budget in max_tokens
            messages,
            system: system_prompt,
            temperature,
            thinking,
        };

        let response = self
            .http_client
            .post(format!("{}/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", API_VERSION)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        let latency_ms = start.elapsed().as_millis() as u64;
        let status = response.status();

        if status == 429 {
            // Rate limited
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(60)
                * 1000;
            return Err(ProviderError::RateLimited {
                retry_after_ms: retry_after,
            });
        }

        if !status.is_success() {
            let error: AnthropicError = response.json().await.map_err(|e| ProviderError::Parse(e.to_string()))?;
            return Err(ProviderError::Api {
                status: status.as_u16(),
                message: error.error.message,
            });
        }

        let api_response: AnthropicResponse = response.json().await?;

        // Record token usage
        self.rate_limiter
            .record_tokens(api_response.usage.input_tokens + api_response.usage.output_tokens)
            .await;

        // Extract text content
        let content = api_response
            .content
            .iter()
            .filter_map(|block| {
                if block.content_type == "text" {
                    block.text.clone()
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("");

        Ok(CompletionResponse {
            content,
            model: api_response.model,
            input_tokens: api_response.usage.input_tokens,
            output_tokens: api_response.usage.output_tokens,
            finish_reason: api_response.stop_reason.unwrap_or_else(|| "unknown".to_string()),
            latency_ms,
        })
    }

    fn rate_limiter(&self) -> &Arc<RateLimiter> {
        &self.rate_limiter
    }

    async fn health_check(&self) -> ProviderResult<bool> {
        // Simple health check by making a minimal request
        let request = CompletionRequest::new(vec![Message::user("Hi")], 10);

        match self.complete(&request).await {
            Ok(_) => Ok(true),
            Err(ProviderError::RateLimited { .. }) => Ok(true), // Still healthy, just rate limited
            Err(_) => Ok(false),
        }
    }
}
