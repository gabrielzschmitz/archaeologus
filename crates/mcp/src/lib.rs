//! `archaeologus-mcp` — Model Context Protocol server for the AI Software Archaeologus.
//!
//! Exposes all archaeologus capabilities as MCP tools so that any MCP-compatible
//! AI client (IBM watsonx, Claude, `ChatGPT`, `OpenCode`, …) can query them.
//!
//! # Quick start
//!
//! ```bash
//! # stdio mode (Claude Desktop, OpenCode, …)
//! archaeologus mcp --transport stdio
//!
//! # HTTP mode (IBM watsonx, ChatGPT, remote clients)
//! archaeologus mcp --transport http --port 8080
//! ```

pub mod server;

pub use server::ArchaeologusServer;
