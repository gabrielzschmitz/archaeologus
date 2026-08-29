//! Provider factory and configuration.

use crate::provider::{LLMError, LLMProvider, ProviderType};
use crate::providers::{
    anthropic::AnthropicProvider, mock::MockProvider, ollama::OllamaProvider,
    openai::OpenAIProvider, watsonx::WatsonxProvider,
};

/// Configuration for selecting and constructing an LLM provider.
#[derive(Debug, Clone)]
pub struct LLMConfig {
    /// Which backend to use.  Reads `LLM_PROVIDER` env var; defaults to `watsonx`.
    pub provider_type: ProviderType,
    /// Override for the model identifier.  Reads `LLM_MODEL` env var.
    pub model: Option<String>,
    /// Override for generation temperature.  Reads `LLM_TEMPERATURE` env var.
    pub temperature: Option<f32>,
    /// Override for max output tokens.  Reads `LLM_MAX_TOKENS` env var.
    pub max_tokens: Option<u32>,
}

impl LLMConfig {
    /// Build configuration from environment variables.
    ///
    /// # Errors
    /// Returns an error if `LLM_PROVIDER` contains an unrecognised value.
    pub fn from_env() -> Result<Self, LLMError> {
        let provider_type = std::env::var("LLM_PROVIDER")
            .unwrap_or_else(|_| "watsonx".to_string()) // Default: IBM watsonx (P0)
            .parse()?;

        Ok(Self {
            provider_type,
            model: std::env::var("LLM_MODEL").ok(),
            temperature: std::env::var("LLM_TEMPERATURE")
                .ok()
                .and_then(|v| v.parse().ok()),
            max_tokens: std::env::var("LLM_MAX_TOKENS")
                .ok()
                .and_then(|v| v.parse().ok()),
        })
    }
}

/// Construct a boxed [`LLMProvider`] from the given config.
///
/// # Errors
/// Returns an error if required env vars for the selected provider are missing.
pub fn create_provider(config: &LLMConfig) -> Result<Box<dyn LLMProvider>, LLMError> {
    match config.provider_type {
        ProviderType::Watsonx => Ok(Box::new(WatsonxProvider::from_env()?)),
        ProviderType::OpenAI => Ok(Box::new(OpenAIProvider::from_env()?)),
        ProviderType::Anthropic => Ok(Box::new(AnthropicProvider::from_env()?)),
        ProviderType::Ollama => Ok(Box::new(OllamaProvider::from_env()?)),
        ProviderType::Mock => Ok(Box::new(MockProvider::new())),
    }
}
