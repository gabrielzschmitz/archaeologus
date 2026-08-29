//! `OpenAI` / `ChatGPT` provider.

use async_trait::async_trait;

use crate::provider::{
    ChatMessage, ChatResponse, LLMError, LLMProvider, ProviderType, Role, ToolDefinition,
};

pub struct OpenAIProvider {
    client: reqwest::Client,
    api_key: String,
    model: String,
    base_url: String,
}

impl OpenAIProvider {
    #[must_use]
    pub fn new(api_key: String, model: String, base_url: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            model,
            base_url,
        }
    }

    /// Create from environment variables.
    ///
    /// # Errors
    /// Returns [`LLMError::AuthError`] if `OPENAI_API_KEY` is not set.
    pub fn from_env() -> Result<Self, LLMError> {
        let api_key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| LLMError::AuthError("OPENAI_API_KEY not set".into()))?;
        let model = std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4o".to_string());
        let base_url = std::env::var("OPENAI_BASE_URL")
            .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
        Ok(Self::new(api_key, model, base_url))
    }

    fn messages_json(messages: &[ChatMessage]) -> serde_json::Value {
        serde_json::Value::Array(
            messages
                .iter()
                .map(|m| {
                    serde_json::json!({
                        "role": match m.role {
                            Role::System => "system",
                            Role::User => "user",
                            Role::Assistant => "assistant",
                        },
                        "content": m.content,
                    })
                })
                .collect(),
        )
    }
}

#[async_trait]
impl LLMProvider for OpenAIProvider {
    fn name(&self) -> &'static str {
        "openai"
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::OpenAI
    }

    async fn chat(
        &self,
        messages: &[ChatMessage],
        temperature: Option<f32>,
        max_tokens: Option<u32>,
    ) -> Result<ChatResponse, LLMError> {
        let body = serde_json::json!({
            "model": self.model,
            "messages": Self::messages_json(messages),
            "temperature": temperature.unwrap_or(0.7),
            "max_tokens": max_tokens.unwrap_or(4096),
        });

        let resp = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(LLMError::ApiError(format!(
                "OpenAI error ({status}): {body}"
            )));
        }

        let data: serde_json::Value = resp.json().await?;
        let content = data["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let tokens_used = data["usage"]["total_tokens"]
            .as_u64()
            .and_then(|v| u32::try_from(v).ok());

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
        let tools_json: serde_json::Value = serde_json::Value::Array(
            tools
                .iter()
                .map(|t| {
                    serde_json::json!({
                        "type": "function",
                        "function": {
                            "name": t.name,
                            "description": t.description,
                            "parameters": t.parameters,
                        },
                    })
                })
                .collect(),
        );

        let body = serde_json::json!({
            "model": self.model,
            "messages": Self::messages_json(messages),
            "tools": tools_json,
            "temperature": temperature.unwrap_or(0.7),
        });

        let resp = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(LLMError::ApiError(format!(
                "OpenAI error ({status}): {body}"
            )));
        }

        let data: serde_json::Value = resp.json().await?;
        let content = data["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let tokens_used = data["usage"]["total_tokens"]
            .as_u64()
            .and_then(|v| u32::try_from(v).ok());

        Ok(ChatResponse {
            content,
            tokens_used,
            model: self.model.clone(),
        })
    }
}
