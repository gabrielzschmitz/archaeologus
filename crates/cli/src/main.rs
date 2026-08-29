use archaeologist_cli::commands;
use archaeologist_core::AppConfig;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "archaeologist")]
#[command(about = "AI Software Archaeologist - answers 'why is the code like this?'")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Index a git repository for analysis
    Index {
        /// Git repository URL or local path
        target: String,
        /// Branch to record (does not affect clone)
        #[arg(short, long)]
        branch: Option<String>,
    },
    /// Search for symbols, files, or code in indexed repositories
    Search {
        /// Search term (fuzzy-matched against symbol names or file paths)
        query: String,
        /// What to search: symbols (default), files, or code
        #[arg(short, long, default_value = "symbols")]
        mode: String,
        /// Filter by symbol type (function, class, struct, …)
        #[arg(short = 't', long)]
        symbol_type: Option<String>,
        /// Filter by language (rust, python, go, …)
        #[arg(short, long)]
        language: Option<String>,
        /// Maximum results to show (default 20)
        #[arg(short = 'n', long, default_value = "20")]
        limit: i64,
        /// Pagination offset (default 0)
        #[arg(short, long, default_value = "0")]
        offset: i64,
    },
    /// Ask a question about the codebase
    Ask {
        /// Your question
        question: String,
        /// Filter by language (rust, python, go, …)
        #[arg(short, long)]
        language: Option<String>,
    },
    /// Explain a symbol's purpose and history
    Explain {
        /// Symbol name or file path
        target: String,
        /// Filter by language (rust, python, go, …)
        #[arg(short, long)]
        language: Option<String>,
    },
    /// Show commit history for a symbol
    History {
        /// Symbol name
        symbol: String,
        /// Filter by language (rust, python, go, …)
        #[arg(short, long)]
        language: Option<String>,
    },
    /// Analyze impact of changing a symbol
    Impact {
        /// Symbol name
        symbol: String,
        /// Filter by language (rust, python, go, …)
        #[arg(short, long)]
        language: Option<String>,
    },
    /// Start MCP server for AI client connections
    Mcp {
        /// Transport mode (stdio or http)
        #[arg(short, long, default_value = "stdio")]
        transport: String,
        /// HTTP port (only used with http transport)
        #[arg(short, long, default_value = "8080")]
        port: u16,
    },
    /// Start the HTTP REST API server
    Serve {
        /// Address to bind (e.g. 0.0.0.0:3000)
        #[arg(short, long, default_value = "0.0.0.0:3000")]
        addr: String,
    },
    /// Run database migrations
    Migrate,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = AppConfig::from_env()?;
    let cli = Cli::parse();

    match cli.command {
        Commands::Index { target, branch } => {
            commands::index::run(commands::index::IndexOptions {
                target,
                branch,
                database_url: config.database_url,
                rust_log: config.rust_log,
            })
            .await?;
        }

        Commands::Search {
            query,
            mode,
            symbol_type,
            language,
            limit,
            offset,
        } => {
            let mode = mode
                .parse::<commands::search::SearchMode>()
                .unwrap_or(commands::search::SearchMode::Symbols);
            commands::search::run(commands::search::SearchOptions {
                query,
                mode,
                symbol_type,
                language,
                limit,
                offset,
                database_url: config.database_url,
                rust_log: config.rust_log,
            })
            .await?;
        }

        Commands::Ask { question, language } => {
            commands::ask::run(commands::ask::AskOptions {
                question,
                language,
                database_url: config.database_url,
                rust_log: config.rust_log,
            })
            .await?;
        }
        Commands::Explain { target, language } => {
            commands::explain::run(commands::explain::ExplainOptions {
                target,
                language,
                database_url: config.database_url,
                rust_log: config.rust_log,
            })
            .await?;
        }
        Commands::History { symbol, language } => {
            commands::history::run(commands::history::HistoryOptions {
                symbol,
                language,
                database_url: config.database_url,
                rust_log: config.rust_log,
            })
            .await?;
        }
        Commands::Impact { symbol, language } => {
            commands::impact::run(commands::impact::ImpactOptions {
                symbol,
                language,
                database_url: config.database_url,
                rust_log: config.rust_log,
            })
            .await?;
        }
        Commands::Mcp { transport, port } => {
            init_tracing(&config.rust_log);
            tracing::info!("Starting MCP server (transport={transport}, port={port})");
            let pool = archaeologist_db::create_pool(&config.database_url).await?;
            archaeologist_db::run_migrations(&pool).await?;
            run_mcp_server(pool, &transport, port).await?;
        }
        Commands::Serve { addr } => {
            init_tracing(&config.rust_log);
            let pool = archaeologist_db::create_pool(&config.database_url).await?;
            archaeologist_db::run_migrations(&pool).await?;
            archaeologist_api::serve(pool, &addr).await?;
        }
        Commands::Migrate => {
            init_tracing(&config.rust_log);
            tracing::info!("Running migrations...");
            let pool = archaeologist_db::create_pool(&config.database_url).await?;
            archaeologist_db::run_migrations(&pool).await?;
            println!("Migrations completed");
        }
    }

    Ok(())
}

fn init_tracing(filter: &str) {
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .try_init()
        .ok();
}

async fn run_mcp_server(
    pool: archaeologist_db::PgPool,
    transport: &str,
    port: u16,
) -> anyhow::Result<()> {
    use archaeologist_mcp::ArchaeologistServer;
    use rmcp::ServiceExt;

    let server = ArchaeologistServer::new(pool);

    match transport {
        "stdio" => {
            tracing::info!("MCP server listening on stdio");
            let service = server.serve(rmcp::transport::io::stdio()).await?;
            service.waiting().await?;
        }
        "http" => {
            use rmcp::transport::streamable_http_server::{
                StreamableHttpServerConfig, StreamableHttpService,
            };
            use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
            use std::sync::Arc;

            let addr = format!("0.0.0.0:{port}");
            tracing::info!("MCP server listening on http://{addr}/mcp");

            let pool = server.pool.clone();
            let mcp_service = StreamableHttpService::new(
                move || Ok(ArchaeologistServer::new(pool.clone())),
                Arc::new(LocalSessionManager::default()),
                StreamableHttpServerConfig::default(),
            );

            let app = axum::Router::new().route_service("/mcp", mcp_service);

            let listener = tokio::net::TcpListener::bind(&addr).await?;
            axum::serve(listener, app).await?;
        }
        other => {
            anyhow::bail!("Unknown MCP transport: '{other}'. Use 'stdio' or 'http'.");
        }
    }

    Ok(())
}
