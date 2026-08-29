//! IBM watsonx provider — PRIMARY (P0).
//!
//! Authenticates via IBM Cloud IAM and calls the watsonx text generation API.

use async_trait::async_trait;
use tracing::debug;

use crate::provider::{
    ChatMessage, ChatResponse, LLMError, LLMProvider, ProviderType, Role, ToolDefinition,
};

// ── Provider struct ───────────────────────────────────────────────────────────

/// IBM watsonx provider.  Configure via environment variables:
///
/// * `WATSONX_API_KEY` — IBM Cloud API key (required)
/// * `WATSONX_PROJECT_ID` — watsonx project ID (required)
/// * `WATSONX_MODEL` — model ID (default: `ibm/granite-3-3-8b-instruct`)
/// * `WATSONX_BASE_URL` — service endpoint (default: `https://us-south.ml.cloud.ibm.com`)
pub struct WatsonxProvider {
    client: reqwest::Client,
    api_key: String,
    project_id: String,
    model: String,
    base_url: String,
}

impl WatsonxProvider {
    /// Create from explicit parameters.
    #[must_use]
    pub fn new(api_key: String, project_id: String, model: String, base_url: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            project_id,
            model,
            base_url,
        }
    }

    /// Create from environment variables.
    ///
    /// # Errors
    /// Returns [`LLMError::AuthError`] if required env vars are missing.
    pub fn from_env() -> Result<Self, LLMError> {
        let api_key = std::env::var("WATSONX_API_KEY")
            .map_err(|_| LLMError::AuthError("WATSONX_API_KEY not set".into()))?;
        let project_id = std::env::var("WATSONX_PROJECT_ID")
            .map_err(|_| LLMError::AuthError("WATSONX_PROJECT_ID not set".into()))?;
        let model = std::env::var("WATSONX_MODEL")
            .unwrap_or_else(|_| "ibm/granite-4-h-small".to_string());
        let base_url = std::env::var("WATSONX_BASE_URL")
            .unwrap_or_else(|_| "https://us-south.ml.cloud.ibm.com".to_string());

        Ok(Self::new(api_key, project_id, model, base_url))
    }

    /// Exchange IBM Cloud API key for an IAM bearer token.
    async fn get_token(&self) -> Result<String, LLMError> {
        let resp = self
            .client
            .post("https://iam.cloud.ibm.com/identity/token")
            .form(&[
                ("grant_type", "urn:ibm:params:oauth:grant-type:apikey"),
                ("apikey", self.api_key.as_str()),
            ])
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(LLMError::AuthError(format!(
                "IAM token request failed ({status}): {body}"
            )));
        }

        let data: serde_json::Value = resp.json().await?;
        data["access_token"]
            .as_str()
            .map(std::string::ToString::to_string)
            .ok_or_else(|| LLMError::AuthError("Missing access_token in IAM response".into()))
    }

    /// Convert our [`ChatMessage`] slice to the watsonx JSON format.
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

    /// Extract the generated text from a watsonx response body.
    fn extract_text(data: &serde_json::Value) -> String {
        // Chat completions endpoint uses choices[0].message.content
        if let Some(content) = data["choices"][0]["message"]["content"].as_str() {
            return content.to_string();
        }
        // Text generation endpoint uses results[0].generated_text
        data["results"][0]["generated_text"]
            .as_str()
            .unwrap_or("")
            .to_string()
    }
}

// ── LLMProvider impl ──────────────────────────────────────────────────────────

#[async_trait]
impl LLMProvider for WatsonxProvider {
    fn name(&self) -> &'static str {
        "watsonx"
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::Watsonx
    }

    async fn chat(
        &self,
        messages: &[ChatMessage],
        temperature: Option<f32>,
        max_tokens: Option<u32>,
    ) -> Result<ChatResponse, LLMError> {
        let token = self.get_token().await?;

        let body = serde_json::json!({
            "model_id": self.model,
            "project_id": self.project_id,
            "messages": Self::messages_json(messages),
            "parameters": {
                "temperature": temperature.unwrap_or(0.7),
                "max_new_tokens": max_tokens.unwrap_or(4096),
            },
        });

        let url = format!("{}/ml/v1/text/chat?version=2025-02-06", self.base_url);

        debug!(url = %url, model = %self.model, "watsonx chat request");

        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {token}"))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(LLMError::ApiError(format!(
                "watsonx chat failed ({status}): {body}"
            )));
        }

        let data: serde_json::Value = resp.json().await?;
        let content = Self::extract_text(&data);
        let tokens_used = data["usage"]["total_tokens"]
            .as_u64()
            .or_else(|| data["results"][0]["input_token_count"].as_u64())
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
        let token = self.get_token().await?;

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
            "model_id": self.model,
            "project_id": self.project_id,
            "messages": Self::messages_json(messages),
            "tools": tools_json,
            "parameters": {
                "temperature": temperature.unwrap_or(0.7),
                "max_new_tokens": 4096,
            },
        });

        let url = format!("{}/ml/v1/text/chat?version=2025-02-06", self.base_url);

        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {token}"))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(LLMError::ApiError(format!(
                "watsonx chat_with_tools failed ({status}): {body}"
            )));
        }

        let data: serde_json::Value = resp.json().await?;
        let content = Self::extract_text(&data);
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
