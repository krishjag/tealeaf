//! LLM Provider implementations

pub mod anthropic;
pub mod openai;
pub mod traits;

pub use anthropic::AnthropicClient;
pub use openai::OpenAIClient;
pub use traits::{
    CompletionRequest, CompletionResponse, LLMProvider, Message, ProviderConfig, ProviderError,
    ProviderResult,
};

use crate::config::Config;
use std::sync::Arc;

/// Enum to hold any provider type
pub enum Provider {
    Anthropic(AnthropicClient),
    OpenAI(OpenAIClient),
}

impl Provider {
    /// Get the provider as a trait object
    pub fn as_provider(&self) -> &dyn LLMProvider {
        match self {
            Provider::Anthropic(c) => c,
            Provider::OpenAI(c) => c,
        }
    }

    /// Create provider from name and environment
    pub fn from_name(name: &str) -> ProviderResult<Self> {
        match name.to_lowercase().as_str() {
            "anthropic" | "claude" => Ok(Provider::Anthropic(AnthropicClient::from_env()?)),
            "openai" | "gpt" => Ok(Provider::OpenAI(OpenAIClient::from_env()?)),
            _ => Err(ProviderError::Config(format!("Unknown provider: {}", name))),
        }
    }
}

/// Apply config settings to an Anthropic client
fn configure_anthropic(client: AnthropicClient, config: &Config) -> AnthropicClient {
    if let Some(pc) = config.get_provider("anthropic") {
        let mut c = client
            .with_rate_limits(pc.rpm, pc.tpm)
            .with_model(&pc.default_model);
        if pc.enable_thinking {
            c = c.with_thinking(true).with_thinking_budget(pc.thinking_budget);
        }
        c
    } else {
        client
    }
}

/// Apply config settings to an OpenAI client
fn configure_openai(client: OpenAIClient, config: &Config) -> OpenAIClient {
    if let Some(pc) = config.get_provider("openai") {
        let mut c = client
            .with_rate_limits(pc.rpm, pc.tpm)
            .with_model(&pc.default_model);
        if !pc.reasoning_effort.is_empty() {
            c = c.with_reasoning_effort(&pc.reasoning_effort);
        }
        c
    } else {
        client
    }
}

/// Create all configured providers (legacy — no config applied)
pub fn create_all_providers() -> Vec<Arc<dyn LLMProvider + Send + Sync>> {
    let config = Config::load_or_default();
    create_all_providers_with_config(&config)
}

/// Create all available providers, applying settings from config
pub fn create_all_providers_with_config(config: &Config) -> Vec<Arc<dyn LLMProvider + Send + Sync>> {
    let mut providers: Vec<Arc<dyn LLMProvider + Send + Sync>> = Vec::new();

    if let Ok(client) = AnthropicClient::from_env() {
        providers.push(Arc::new(configure_anthropic(client, config)));
    }

    if let Ok(client) = OpenAIClient::from_env() {
        providers.push(Arc::new(configure_openai(client, config)));
    }

    providers
}

/// Create specific providers by name (legacy — no config applied)
pub fn create_providers(names: &[&str]) -> ProviderResult<Vec<Arc<dyn LLMProvider + Send + Sync>>> {
    let config = Config::load_or_default();
    create_providers_with_config(names, &config)
}

/// Create specific providers by name, applying settings from config
pub fn create_providers_with_config(
    names: &[&str],
    config: &Config,
) -> ProviderResult<Vec<Arc<dyn LLMProvider + Send + Sync>>> {
    let mut providers: Vec<Arc<dyn LLMProvider + Send + Sync>> = Vec::new();

    for name in names {
        match name.to_lowercase().as_str() {
            "anthropic" | "claude" => {
                let client = AnthropicClient::from_env()?;
                providers.push(Arc::new(configure_anthropic(client, config)));
            }
            "openai" | "gpt" => {
                let client = OpenAIClient::from_env()?;
                providers.push(Arc::new(configure_openai(client, config)));
            }
            _ => return Err(ProviderError::Config(format!("Unknown provider: {}", name))),
        }
    }

    Ok(providers)
}
