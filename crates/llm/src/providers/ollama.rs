//! Ollama local model provider.

use async_trait::async_trait;

use crate::provider::{
    ChatMessage, ChatResponse, LLMError, LLMProvider, ProviderType, Role, ToolDefinition,
};

pub struct OllamaProvider {
    client: reqwest::Client,
    base_url: String,
    model: String,
}

impl OllamaProvider {
    #[must_use]
    pub fn new(base_url: String, model: String) -> Self {
        Self { client: reqwest::Client::new(), base_url, model }
    }

    /// Create from environment variables.
    ///
    /// # Errors
    /// This function currently never fails (all env vars have defaults).
    pub fn from_env() -> Result<Self, LLMError> {
        let base_url = std::env::var("OLLAMA_BASE_URL")
            .unwrap_or_else(|_| "http://localhost:11434".to_string());
        let model = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "llama3".to_string());
        Ok(Self::new(base_url, model))
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
impl LLMProvider for OllamaProvider {
    fn name(&self) -> &'static str {
        "ollama"
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::Ollama
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
            "stream": false,
            "options": {
                "temperature": temperature.unwrap_or(0.7),
                "num_predict": max_tokens.unwrap_or(4096),
            },
        });

        let resp = self
            .client
            .post(format!("{}/api/chat", self.base_url))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(LLMError::ApiError(format!("Ollama error ({status}): {body}")));
        }

        let data: serde_json::Value = resp.json().await?;
        let content = data["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        Ok(ChatResponse { content, tokens_used: None, model: self.model.clone() })
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
            "stream": false,
            "options": {
                "temperature": temperature.unwrap_or(0.7),
            },
        });

        let resp = self
            .client
            .post(format!("{}/api/chat", self.base_url))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(LLMError::ApiError(format!("Ollama error ({status}): {body}")));
        }

        let data: serde_json::Value = resp.json().await?;
        let content = data["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        Ok(ChatResponse { content, tokens_used: None, model: self.model.clone() })
    }
}
