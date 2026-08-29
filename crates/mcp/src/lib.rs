//! `archaeologist-mcp` — Model Context Protocol server for the AI Software Archaeologist.
//!
//! Exposes all archaeologist capabilities as MCP tools so that any MCP-compatible
//! AI client (IBM watsonx, Claude, `ChatGPT`, `OpenCode`, …) can query them.
//!
//! # Quick start
//!
//! ```bash
//! # stdio mode (Claude Desktop, OpenCode, …)
//! archaeologist mcp --transport stdio
//!
//! # HTTP mode (IBM watsonx, ChatGPT, remote clients)
//! archaeologist mcp --transport http --port 8080
//! ```

pub mod server;

pub use server::ArchaeologistServer;
