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
pub mod provider;
pub mod providers;

pub use config::{create_provider, LLMConfig};
pub use provider::{ChatMessage, ChatResponse, LLMError, LLMProvider, Role, ToolDefinition};
