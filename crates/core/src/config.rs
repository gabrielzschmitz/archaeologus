use std::env;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub rust_log: String,
    pub llm_provider: String,
    pub llm_model: Option<String>,
    pub mcp_transport: String,
    pub mcp_http_addr: Option<String>,
}

impl AppConfig {
    /// # Errors
    ///
    /// Returns an error if any required environment variable cannot be read.
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            database_url: env::var("DATABASE_URL").unwrap_or_else(|_| {
                "postgres://archaeologist:archaeologist_dev@localhost:5432/archaeologist".into()
            }),
            rust_log: env::var("RUST_LOG").unwrap_or_else(|_| "info,sqlx=warn".into()),
            llm_provider: env::var("LLM_PROVIDER").unwrap_or_else(|_| "watsonx".into()),
            llm_model: env::var("LLM_MODEL").ok(),
            mcp_transport: env::var("MCP_TRANSPORT").unwrap_or_else(|_| "stdio".into()),
            mcp_http_addr: env::var("MCP_HTTP_ADDR").ok(),
        })
    }
}
