//! `archaeologist-llm` — LLM provider abstraction.
//!
//! Provides a unified [`LLMProvider`] trait over multiple AI backends.
//! IBM watsonx is the primary provider (P0) for the hackathon.
//!
//! # Quick start
//!
//! ```rust,no_run
//! use archaeologist_llm::config::{LLMConfig, create_provider};
//!
//! # async fn example() -> Result<(), archaeologist_llm::provider::LLMError> {
//! let config = LLMConfig::from_env()?;
//! let provider = create_provider(&config)?;
//! # Ok(())
//! # }
//! ```

pub mod config;
pub mod prompt;
pub mod provider;
pub mod providers;

pub use config::{create_provider, LLMConfig};
pub use prompt::{
    build_ask_prompt, deps_context, evidence_context, explanation_context, file_context,
    repo_context, siblings_context, symbol_context, system_prompt, SymbolContext,
};
pub use provider::{ChatMessage, ChatResponse, LLMError, LLMProvider, Role, ToolDefinition};
