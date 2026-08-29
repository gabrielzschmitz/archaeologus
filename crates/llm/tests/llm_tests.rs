//! Integration tests for the `archaeologist-llm` crate.
//!
//! These tests use the [`MockProvider`] to avoid real network calls, and also
//! verify that provider initialization from environment variables, the prompt
//! builder, and provider-switching logic all work correctly.

use archaeologist_llm::{
    build_ask_prompt,
    config::{create_provider, LLMConfig},
    provider::{ChatMessage, LLMError, LLMProvider, ProviderType, Role},
    providers::{mock::MockProvider, ollama::OllamaProvider, openai::OpenAIProvider},
    system_prompt,
};
use std::str::FromStr;

// ── Provider type parsing ─────────────────────────────────────────────────────

#[test]
fn provider_type_parses_watsonx_aliases() {
    assert_eq!(
        ProviderType::from_str("watsonx").unwrap(),
        ProviderType::Watsonx
    );
    assert_eq!(
        ProviderType::from_str("ibm").unwrap(),
        ProviderType::Watsonx
    );
}

#[test]
fn provider_type_parses_openai_aliases() {
    assert_eq!(
        ProviderType::from_str("openai").unwrap(),
        ProviderType::OpenAI
    );
    assert_eq!(
        ProviderType::from_str("chatgpt").unwrap(),
        ProviderType::OpenAI
    );
}

#[test]
fn provider_type_parses_anthropic_aliases() {
    assert_eq!(
        ProviderType::from_str("anthropic").unwrap(),
        ProviderType::Anthropic
    );
    assert_eq!(
        ProviderType::from_str("claude").unwrap(),
        ProviderType::Anthropic
    );
}

#[test]
fn provider_type_parses_ollama_aliases() {
    assert_eq!(
        ProviderType::from_str("ollama").unwrap(),
        ProviderType::Ollama
    );
    assert_eq!(
        ProviderType::from_str("local").unwrap(),
        ProviderType::Ollama
    );
}

#[test]
fn provider_type_parses_mock_aliases() {
    assert_eq!(ProviderType::from_str("mock").unwrap(), ProviderType::Mock);
    assert_eq!(ProviderType::from_str("test").unwrap(), ProviderType::Mock);
}

#[test]
fn provider_type_rejects_unknown() {
    let err = ProviderType::from_str("grok").unwrap_err();
    assert!(matches!(err, LLMError::ProviderUnavailable(_)));
}

#[test]
fn provider_type_display() {
    assert_eq!(ProviderType::Watsonx.to_string(), "watsonx");
    assert_eq!(ProviderType::OpenAI.to_string(), "openai");
    assert_eq!(ProviderType::Anthropic.to_string(), "anthropic");
    assert_eq!(ProviderType::Ollama.to_string(), "ollama");
    assert_eq!(ProviderType::Mock.to_string(), "mock");
}

// ── LLMConfig from env ────────────────────────────────────────────────────────

#[test]
fn llm_config_defaults_to_watsonx() {
    // Ensure the env var is not set so we get the default.
    std::env::remove_var("LLM_PROVIDER");
    let cfg = LLMConfig::from_env().unwrap();
    assert_eq!(cfg.provider_type, ProviderType::Watsonx);
}

#[test]
fn llm_config_reads_provider_from_env() {
    std::env::set_var("LLM_PROVIDER", "mock");
    let cfg = LLMConfig::from_env().unwrap();
    assert_eq!(cfg.provider_type, ProviderType::Mock);
    std::env::remove_var("LLM_PROVIDER");
}

#[test]
fn llm_config_reads_temperature_from_env() {
    std::env::set_var("LLM_TEMPERATURE", "0.3");
    std::env::set_var("LLM_PROVIDER", "mock");
    let cfg = LLMConfig::from_env().unwrap();
    assert_eq!(cfg.temperature, Some(0.3_f32));
    std::env::remove_var("LLM_TEMPERATURE");
    std::env::remove_var("LLM_PROVIDER");
}

#[test]
fn llm_config_reads_max_tokens_from_env() {
    std::env::set_var("LLM_MAX_TOKENS", "1024");
    std::env::set_var("LLM_PROVIDER", "mock");
    let cfg = LLMConfig::from_env().unwrap();
    assert_eq!(cfg.max_tokens, Some(1024_u32));
    std::env::remove_var("LLM_MAX_TOKENS");
    std::env::remove_var("LLM_PROVIDER");
}

#[test]
fn llm_config_invalid_provider_returns_error() {
    std::env::set_var("LLM_PROVIDER", "nonexistent_provider");
    let result = LLMConfig::from_env();
    std::env::remove_var("LLM_PROVIDER");
    assert!(result.is_err());
}

// ── Provider factory ──────────────────────────────────────────────────────────

#[test]
fn create_provider_returns_mock_for_mock_type() {
    std::env::set_var("LLM_PROVIDER", "mock");
    let cfg = LLMConfig::from_env().unwrap();
    let provider = create_provider(&cfg).unwrap();
    assert_eq!(provider.name(), "mock");
    assert_eq!(provider.provider_type(), ProviderType::Mock);
    std::env::remove_var("LLM_PROVIDER");
}

#[test]
fn create_provider_ollama_always_succeeds() {
    // Ollama has no required env vars.
    std::env::set_var("LLM_PROVIDER", "ollama");
    let cfg = LLMConfig::from_env().unwrap();
    let provider = create_provider(&cfg).unwrap();
    assert_eq!(provider.name(), "ollama");
    std::env::remove_var("LLM_PROVIDER");
}

#[test]
fn create_provider_watsonx_fails_without_api_key() {
    std::env::set_var("LLM_PROVIDER", "watsonx");
    std::env::remove_var("WATSONX_API_KEY");
    std::env::remove_var("WATSONX_PROJECT_ID");
    let cfg = LLMConfig::from_env().unwrap();
    let result = create_provider(&cfg);
    std::env::remove_var("LLM_PROVIDER");
    // Should fail with an auth error (missing env var).
    assert!(matches!(result, Err(LLMError::AuthError(_))));
}

#[test]
fn create_provider_openai_fails_without_api_key() {
    std::env::set_var("LLM_PROVIDER", "openai");
    std::env::remove_var("OPENAI_API_KEY");
    let cfg = LLMConfig::from_env().unwrap();
    let result = create_provider(&cfg);
    std::env::remove_var("LLM_PROVIDER");
    assert!(result.is_err());
}

#[test]
fn create_provider_anthropic_fails_without_api_key() {
    // This test manipulates env vars; run under a mutex to avoid races.
    use std::sync::Mutex;
    static LOCK: Mutex<()> = Mutex::new(());
    let _guard = LOCK.lock().unwrap();

    std::env::set_var("LLM_PROVIDER", "anthropic");
    let saved = std::env::var("ANTHROPIC_API_KEY").ok();
    std::env::remove_var("ANTHROPIC_API_KEY");
    let cfg = LLMConfig::from_env().unwrap();
    let result = create_provider(&cfg);
    if let Some(k) = saved {
        std::env::set_var("ANTHROPIC_API_KEY", k);
    }
    std::env::remove_var("LLM_PROVIDER");
    assert!(result.is_err());
}

// ── MockProvider ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn mock_provider_returns_fixed_response() {
    let provider = MockProvider::with_response("mocked answer");
    let messages = vec![ChatMessage::user("hello")];
    let resp = provider.chat(&messages, None, None).await.unwrap();
    assert_eq!(resp.content, "mocked answer");
    assert_eq!(resp.model, "mock");
    assert_eq!(resp.tokens_used, Some(0));
}

#[tokio::test]
async fn mock_provider_with_tools_returns_fixed_response() {
    let provider = MockProvider::with_response("tool answer");
    let messages = vec![ChatMessage::user("what tools?")];
    let resp = provider
        .chat_with_tools(&messages, &[], None)
        .await
        .unwrap();
    assert_eq!(resp.content, "tool answer");
}

#[tokio::test]
async fn mock_provider_default_returns_empty_string() {
    let provider = MockProvider::new();
    let messages = vec![ChatMessage::user("hello")];
    let resp = provider.chat(&messages, None, None).await.unwrap();
    assert_eq!(resp.content, "");
}

// ── OpenAI / Ollama constructors ──────────────────────────────────────────────

#[test]
fn openai_provider_new_stores_fields() {
    let p = OpenAIProvider::new(
        "key".to_string(),
        "gpt-4o".to_string(),
        "https://api.openai.com/v1".to_string(),
    );
    assert_eq!(p.name(), "openai");
    assert_eq!(p.provider_type(), ProviderType::OpenAI);
}

#[test]
fn ollama_provider_new_stores_fields() {
    let p = OllamaProvider::new("http://localhost:11434".to_string(), "llama3".to_string());
    assert_eq!(p.name(), "ollama");
    assert_eq!(p.provider_type(), ProviderType::Ollama);
}

// ── Prompt builder ────────────────────────────────────────────────────────────

#[test]
fn system_prompt_has_system_role() {
    let msg = system_prompt();
    assert_eq!(msg.role, Role::System);
    assert!(msg.content.contains("Archaeologist"));
}

#[test]
fn build_ask_prompt_has_user_role_and_question() {
    let msg = build_ask_prompt("why does auth exist?", &[]);
    assert_eq!(msg.role, Role::User);
    assert!(msg.content.contains("why does auth exist?"));
}

#[test]
fn build_ask_prompt_no_context_mentions_no_symbols() {
    let msg = build_ask_prompt("explain this", &[]);
    assert!(msg.content.contains("No matching symbols"));
}

#[test]
fn build_ask_prompt_with_full_context_structure() {
    use archaeologist_llm::SymbolContext;
    use archaeologist_evidence::{aggregate_evidence, explain_symbol};
    use archaeologist_core::models::{Symbol, File, Repository};
    use chrono::Utc;
    use uuid::Uuid;

    let sym = Symbol {
        id: Uuid::new_v4(),
        file_id: Uuid::new_v4(),
        repository_id: Uuid::new_v4(),
        name: "handleRequest".to_string(),
        symbol_type: "function".to_string(),
        language: "go".to_string(),
        line_start: 42,
        line_end: 80,
        col_start: 0,
        col_end: 0,
        visibility: Some("pub".to_string()),
        doc_comment: Some("HandleRequest processes incoming HTTP requests.".to_string()),
        raw_text: "func handleRequest(w http.ResponseWriter, r *http.Request) {}".to_string(),
        created_at: Utc::now(),
    };
    let file = File {
        id: Uuid::new_v4(),
        repository_id: sym.repository_id,
        path: "internal/api/handler.go".to_string(),
        language: Some("go".to_string()),
        size_bytes: 2048,
        content_hash: "abc".to_string(),
        indexed_at: Utc::now(),
    };
    let repo = Repository {
        id: sym.repository_id,
        name: "my-api".to_string(),
        url: "https://github.com/org/my-api".to_string(),
        local_path: None,
        description: None,
        default_branch: "main".to_string(),
        indexed_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let evidence = aggregate_evidence(sym.id, Some(&sym), &[], &[], &[]);
    let expl = explain_symbol(&sym.name, &evidence);
    let ctx = SymbolContext {
        symbol: &sym,
        file: Some(&file),
        repo: Some(&repo),
        deps: &[],
        siblings: &[],
        evidence: &evidence,
        explanation: &expl,
    };
    let msg = build_ask_prompt("how is handleRequest used?", &[ctx]);
    assert!(msg.content.contains("how is handleRequest used?"));
    assert!(msg.content.contains("my-api"));
    assert!(msg.content.contains("internal/api/handler.go"));
    assert!(msg.content.contains("42"));
    assert!(msg.content.contains("HandleRequest processes incoming HTTP requests."));
}

// ── ChatMessage helpers ───────────────────────────────────────────────────────

#[test]
fn chat_message_constructors_set_roles() {
    assert_eq!(ChatMessage::system("s").role, Role::System);
    assert_eq!(ChatMessage::user("u").role, Role::User);
    assert_eq!(ChatMessage::assistant("a").role, Role::Assistant);
}

// ── Provider switching via env var ────────────────────────────────────────────

#[test]
fn provider_switching_via_env_var() {
    // Simulate switching between providers.
    for (env_val, expected_type) in [
        ("mock", ProviderType::Mock),
        ("ollama", ProviderType::Ollama),
    ] {
        std::env::set_var("LLM_PROVIDER", env_val);
        let cfg = LLMConfig::from_env().unwrap();
        assert_eq!(cfg.provider_type, expected_type);
    }
    std::env::remove_var("LLM_PROVIDER");
}
