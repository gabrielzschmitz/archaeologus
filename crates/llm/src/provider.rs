//! Core LLM provider trait and shared types.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// ── Chat types ────────────────────────────────────────────────────────────────

/// A single message in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: Role,
    pub content: String,
}

impl ChatMessage {
    #[must_use]
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: content.into(),
        }
    }

    #[must_use]
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
        }
    }

    #[must_use]
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
        }
    }
}

/// Conversation participant role.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
}

/// Response from a chat completion call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    /// The generated text.
    pub content: String,
    /// Total input tokens used (if reported by the provider).
    pub tokens_used: Option<u32>,
    /// Model identifier used for generation.
    pub model: String,
}

/// A tool/function definition that can be passed to a model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    /// JSON Schema object describing the function parameters.
    pub parameters: serde_json::Value,
}

// ── Provider type ─────────────────────────────────────────────────────────────

/// Which backend to use.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProviderType {
    /// IBM watsonx — PRIMARY (P0).
    Watsonx,
    /// `OpenAI` / `ChatGPT`.
    OpenAI,
    /// Anthropic / Claude.
    Anthropic,
    /// Local Ollama instance.
    Ollama,
    /// In-memory mock (for tests).
    Mock,
}

impl std::fmt::Display for ProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderType::Watsonx => write!(f, "watsonx"),
            ProviderType::OpenAI => write!(f, "openai"),
            ProviderType::Anthropic => write!(f, "anthropic"),
            ProviderType::Ollama => write!(f, "ollama"),
            ProviderType::Mock => write!(f, "mock"),
        }
    }
}

impl std::str::FromStr for ProviderType {
    type Err = LLMError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "watsonx" | "ibm" => Ok(ProviderType::Watsonx),
            "openai" | "chatgpt" => Ok(ProviderType::OpenAI),
            "anthropic" | "claude" => Ok(ProviderType::Anthropic),
            "ollama" | "local" => Ok(ProviderType::Ollama),
            "mock" | "test" => Ok(ProviderType::Mock),
            _ => Err(LLMError::ProviderUnavailable(format!(
                "Unknown provider: {s}"
            ))),
        }
    }
}

// ── Trait ─────────────────────────────────────────────────────────────────────

/// Unified interface over all supported LLM backends.
#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// Human-readable name for this provider instance.
    fn name(&self) -> &str;

    /// Which backend this provider uses.
    fn provider_type(&self) -> ProviderType;

    /// Send a chat completion request.
    async fn chat(
        &self,
        messages: &[ChatMessage],
        temperature: Option<f32>,
        max_tokens: Option<u32>,
    ) -> Result<ChatResponse, LLMError>;

    /// Send a chat completion request with tool definitions.
    async fn chat_with_tools(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolDefinition],
        temperature: Option<f32>,
    ) -> Result<ChatResponse, LLMError>;
}

// ── Errors ────────────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum LLMError {
    #[error("API error: {0}")]
    ApiError(String),

    #[error("Authentication error: {0}")]
    AuthError(String),

    #[error("Rate limit exceeded")]
    RateLimited,

    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Provider not available: {0}")]
    ProviderUnavailable(String),

    #[error("Environment error: {0}")]
    EnvError(String),
}

impl From<std::env::VarError> for LLMError {
    fn from(e: std::env::VarError) -> Self {
        LLMError::EnvError(e.to_string())
    }
}
