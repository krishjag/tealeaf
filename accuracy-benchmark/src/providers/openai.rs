//! OpenAI API client with GPT-5.x and reasoning model support

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

use super::traits::{
    CompletionRequest, CompletionResponse, LLMProvider, Message, ProviderError, ProviderResult,
};
use crate::runner::rate_limiter::RateLimiter;

const DEFAULT_MODEL: &str = "gpt-5.2";
const DEFAULT_REASONING_EFFORT: &str = "medium";

/// OpenAI API client with o3-mini reasoning model support
pub struct OpenAIClient {
    api_key: String,
    base_url: String,
    http_client: Client,
    rate_limiter: Arc<RateLimiter>,
    default_model: String,
    /// Reasoning effort for o3-mini (low, medium, high)
    reasoning_effort: String,
}

impl OpenAIClient {
    /// Create a new OpenAI client with o3-mini
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            base_url: "https://api.openai.com/v1".to_string(),
            http_client: Client::new(),
            rate_limiter: Arc::new(RateLimiter::new(500, 200_000)),
            default_model: DEFAULT_MODEL.to_string(),
            reasoning_effort: DEFAULT_REASONING_EFFORT.to_string(),
        }
    }

    /// Create from environment variable
    pub fn from_env() -> ProviderResult<Self> {
        let api_key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| ProviderError::Config("OPENAI_API_KEY not set".to_string()))?;
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

    /// Set reasoning effort (low, medium, high) for o3-mini
    pub fn with_reasoning_effort(mut self, effort: impl Into<String>) -> Self {
        self.reasoning_effort = effort.into();
        self
    }

    /// Check if current model is a reasoning model (o1, o3-mini, etc.)
    #[allow(dead_code)]
    fn is_reasoning_model(&self) -> bool {
        self.default_model.starts_with("o1") || self.default_model.starts_with("o3")
    }
}

#[derive(Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    /// For standard models
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    /// For reasoning models (o1, o3-mini)
    #[serde(skip_serializing_if = "Option::is_none")]
    max_completion_tokens: Option<u32>,
    /// Temperature (not supported by reasoning models)
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    /// Reasoning effort for o3-mini (low, medium, high)
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning_effort: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct OpenAIMessage {
    role: String,
    content: String,
}

impl From<&Message> for OpenAIMessage {
    fn from(msg: &Message) -> Self {
        Self {
            role: msg.role.clone(),
            content: msg.content.clone(),
        }
    }
}

#[derive(Deserialize)]
struct OpenAIResponse {
    choices: Vec<Choice>,
    model: String,
    usage: OpenAIUsage,
}

#[derive(Deserialize)]
struct Choice {
    message: OpenAIMessage,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct OpenAIUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
}

#[derive(Deserialize)]
struct OpenAIError {
    error: OpenAIErrorDetail,
}

#[derive(Deserialize)]
struct OpenAIErrorDetail {
    message: String,
    #[serde(rename = "type")]
    #[allow(dead_code)]
    error_type: Option<String>,
}

#[async_trait]
impl LLMProvider for OpenAIClient {
    fn name(&self) -> &str {
        "openai"
    }

    fn default_model(&self) -> &str {
        &self.default_model
    }

    async fn complete(&self, request: &CompletionRequest) -> ProviderResult<CompletionResponse> {
        // Acquire rate limit permission
        let _guard = self.rate_limiter.acquire().await;

        let start = Instant::now();

        // Build messages, including system prompt if provided
        let mut messages: Vec<OpenAIMessage> = Vec::new();

        if let Some(system) = &request.system_prompt {
            messages.push(OpenAIMessage {
                role: "system".to_string(),
                content: system.clone(),
            });
        }

        for msg in &request.messages {
            messages.push(msg.into());
        }

        let model = request
            .model
            .clone()
            .unwrap_or_else(|| self.default_model.clone());

        // Check if using a reasoning model or newer GPT-5.x models
        let is_reasoning = model.starts_with("o1") || model.starts_with("o3");
        let is_gpt5 = model.starts_with("gpt-5");
        let uses_completion_tokens = is_reasoning || is_gpt5;
        let is_o3 = model.starts_with("o3");

        let body = if uses_completion_tokens {
            // Reasoning models and GPT-5.x use max_completion_tokens
            OpenAIRequest {
                model,
                messages,
                max_tokens: None,
                max_completion_tokens: Some(request.max_tokens),
                temperature: if is_reasoning { None } else { request.temperature },
                reasoning_effort: if is_o3 {
                    Some(self.reasoning_effort.clone())
                } else {
                    None
                },
            }
        } else {
            // Legacy models use max_tokens
            OpenAIRequest {
                model,
                messages,
                max_tokens: Some(request.max_tokens),
                max_completion_tokens: None,
                temperature: request.temperature,
                reasoning_effort: None,
            }
        };

        let response = self
            .http_client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        let latency_ms = start.elapsed().as_millis() as u64;
        let status = response.status();

        if status == 429 {
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(60)
                * 1000;

            // Parse the response body to distinguish rate_limit_error from
            // insufficient_quota. OpenAI returns 429 for both, but only the
            // former is worth retrying.
            let body = response.text().await.unwrap_or_default();
            if let Ok(error) = serde_json::from_str::<OpenAIError>(&body) {
                let error_type = error.error.error_type.as_deref().unwrap_or("");
                if error_type == "insufficient_quota" || error.error.message.contains("exceeded your current quota") {
                    return Err(ProviderError::Config(format!(
                        "OpenAI quota exceeded: {}",
                        error.error.message
                    )));
                }
                tracing::debug!("Rate limited (type={}): {}", error_type, error.error.message);
            }

            return Err(ProviderError::RateLimited {
                retry_after_ms: retry_after,
            });
        }

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            let message = match serde_json::from_str::<OpenAIError>(&body) {
                Ok(error) => error.error.message,
                Err(_) => format!("HTTP {}: {}", status.as_u16(), body),
            };

            // 401/403 are auth errors — don't waste retries
            if status == 401 || status == 403 {
                return Err(ProviderError::Config(format!(
                    "OpenAI auth error ({}): {}",
                    status.as_u16(),
                    message
                )));
            }

            return Err(ProviderError::Api {
                status: status.as_u16(),
                message,
            });
        }

        let api_response: OpenAIResponse = response.json().await?;

        // Record token usage
        self.rate_limiter
            .record_tokens(api_response.usage.prompt_tokens + api_response.usage.completion_tokens)
            .await;

        let choice = api_response
            .choices
            .first()
            .ok_or_else(|| ProviderError::Parse("No choices in response".to_string()))?;

        Ok(CompletionResponse {
            content: choice.message.content.clone(),
            model: api_response.model,
            input_tokens: api_response.usage.prompt_tokens,
            output_tokens: api_response.usage.completion_tokens,
            finish_reason: choice.finish_reason.clone().unwrap_or_else(|| "unknown".to_string()),
            latency_ms,
        })
    }

    fn rate_limiter(&self) -> &Arc<RateLimiter> {
        &self.rate_limiter
    }

    async fn health_check(&self) -> ProviderResult<bool> {
        let request = CompletionRequest::new(vec![Message::user("Hi")], 10);

        match self.complete(&request).await {
            Ok(_) => Ok(true),
            Err(ProviderError::RateLimited { .. }) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}
