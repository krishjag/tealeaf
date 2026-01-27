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

/// Create all configured providers
pub fn create_all_providers() -> Vec<Arc<dyn LLMProvider + Send + Sync>> {
    let mut providers: Vec<Arc<dyn LLMProvider + Send + Sync>> = Vec::new();

    if let Ok(client) = AnthropicClient::from_env() {
        providers.push(Arc::new(client));
    }

    if let Ok(client) = OpenAIClient::from_env() {
        providers.push(Arc::new(client));
    }

    providers
}

/// Create specific providers by name
pub fn create_providers(names: &[&str]) -> ProviderResult<Vec<Arc<dyn LLMProvider + Send + Sync>>> {
    let mut providers: Vec<Arc<dyn LLMProvider + Send + Sync>> = Vec::new();

    for name in names {
        match Provider::from_name(name)? {
            Provider::Anthropic(c) => providers.push(Arc::new(c)),
            Provider::OpenAI(c) => providers.push(Arc::new(c)),
        }
    }

    Ok(providers)
}
