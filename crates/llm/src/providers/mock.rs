//! In-memory mock provider for tests.

use async_trait::async_trait;

use crate::provider::{
    ChatMessage, ChatResponse, LLMError, LLMProvider, ProviderType, ToolDefinition,
};

/// Mock LLM provider that returns a fixed response.  Use in tests to avoid
/// network calls.
#[derive(Debug, Clone, Default)]
pub struct MockProvider {
    /// The response text returned by all chat calls.
    pub response: String,
}

impl MockProvider {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_response(response: impl Into<String>) -> Self {
        Self { response: response.into() }
    }
}

#[async_trait]
impl LLMProvider for MockProvider {
    fn name(&self) -> &'static str {
        "mock"
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::Mock
    }

    async fn chat(
        &self,
        _messages: &[ChatMessage],
        _temperature: Option<f32>,
        _max_tokens: Option<u32>,
    ) -> Result<ChatResponse, LLMError> {
        Ok(ChatResponse {
            content: self.response.clone(),
            tokens_used: Some(0),
            model: "mock".to_string(),
        })
    }

    async fn chat_with_tools(
        &self,
        _messages: &[ChatMessage],
        _tools: &[ToolDefinition],
        _temperature: Option<f32>,
    ) -> Result<ChatResponse, LLMError> {
        Ok(ChatResponse {
            content: self.response.clone(),
            tokens_used: Some(0),
            model: "mock".to_string(),
        })
    }
}
