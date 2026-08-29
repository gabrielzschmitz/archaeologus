//! Anthropic / Claude provider.

use async_trait::async_trait;

use crate::provider::{
    ChatMessage, ChatResponse, LLMError, LLMProvider, ProviderType, Role, ToolDefinition,
};

pub struct AnthropicProvider {
    client: reqwest::Client,
    api_key: String,
    model: String,
}

impl AnthropicProvider {
    #[must_use]
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            model,
        }
    }

    /// Create from environment variables.
    ///
    /// # Errors
    /// Returns [`LLMError::AuthError`] if `ANTHROPIC_API_KEY` is not set.
    pub fn from_env() -> Result<Self, LLMError> {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .map_err(|_| LLMError::AuthError("ANTHROPIC_API_KEY not set".into()))?;
        let model = std::env::var("ANTHROPIC_MODEL")
            .unwrap_or_else(|_| "claude-sonnet-4-20250514".to_string());
        Ok(Self::new(api_key, model))
    }

    fn messages_json(messages: &[ChatMessage]) -> (Option<String>, serde_json::Value) {
        let mut system_prompt: Option<String> = None;
        let chat_messages: Vec<serde_json::Value> = messages
            .iter()
            .filter_map(|m| match m.role {
                Role::System => {
                    system_prompt = Some(m.content.clone());
                    None
                }
                Role::User => Some(serde_json::json!({
                    "role": "user",
                    "content": m.content,
                })),
                Role::Assistant => Some(serde_json::json!({
                    "role": "assistant",
                    "content": m.content,
                })),
            })
            .collect();
        (system_prompt, serde_json::Value::Array(chat_messages))
    }
}

#[async_trait]
impl LLMProvider for AnthropicProvider {
    fn name(&self) -> &'static str {
        "anthropic"
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::Anthropic
    }

    async fn chat(
        &self,
        messages: &[ChatMessage],
        temperature: Option<f32>,
        max_tokens: Option<u32>,
    ) -> Result<ChatResponse, LLMError> {
        let (system, msgs) = Self::messages_json(messages);

        let mut body = serde_json::json!({
            "model": self.model,
            "messages": msgs,
            "max_tokens": max_tokens.unwrap_or(4096),
            "temperature": temperature.unwrap_or(0.7),
        });
        if let Some(sys) = system {
            body["system"] = serde_json::Value::String(sys);
        }

        let resp = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(LLMError::ApiError(format!(
                "Anthropic error ({status}): {body}"
            )));
        }

        let data: serde_json::Value = resp.json().await?;
        let content = data["content"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let tokens_used = data["usage"]["input_tokens"]
            .as_u64()
            .zip(data["usage"]["output_tokens"].as_u64())
            .and_then(|(i, o)| u32::try_from(i + o).ok());

        Ok(ChatResponse {
            content,
            tokens_used,
            model: self.model.clone(),
        })
    }

    async fn chat_with_tools(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolDefinition],
        temperature: Option<f32>,
    ) -> Result<ChatResponse, LLMError> {
        let (system, msgs) = Self::messages_json(messages);

        let tools_json: serde_json::Value = serde_json::Value::Array(
            tools
                .iter()
                .map(|t| {
                    serde_json::json!({
                        "name": t.name,
                        "description": t.description,
                        "input_schema": t.parameters,
                    })
                })
                .collect(),
        );

        let mut body = serde_json::json!({
            "model": self.model,
            "messages": msgs,
            "tools": tools_json,
            "max_tokens": 4096,
            "temperature": temperature.unwrap_or(0.7),
        });
        if let Some(sys) = system {
            body["system"] = serde_json::Value::String(sys);
        }

        let resp = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(LLMError::ApiError(format!(
                "Anthropic error ({status}): {body}"
            )));
        }

        let data: serde_json::Value = resp.json().await?;
        let content = data["content"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let tokens_used = data["usage"]["input_tokens"]
            .as_u64()
            .zip(data["usage"]["output_tokens"].as_u64())
            .and_then(|(i, o)| u32::try_from(i + o).ok());

        Ok(ChatResponse {
            content,
            tokens_used,
            model: self.model.clone(),
        })
    }
}
