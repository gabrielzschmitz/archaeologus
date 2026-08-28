mod commands;

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
    /// Ask a question about the codebase
    Ask {
        /// Your question
        question: String,
    },
    /// Explain a symbol's purpose and history
    Explain {
        /// Symbol name or file path
        target: String,
    },
    /// Show commit history for a symbol
    History {
        /// Symbol name
        symbol: String,
    },
    /// Analyze impact of changing a symbol
    Impact {
        /// Symbol name
        symbol: String,
    },
    /// Search for symbols in the codebase
    Search {
        /// Search query
        query: String,
        /// Symbol type filter
        #[arg(short, long)]
        symbol_type: Option<String>,
        /// Language filter
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
        Commands::Ask { question } => {
            init_tracing(&config.rust_log);
            tracing::info!("Asking: {question}");
            let pool = archaeologist_db::create_pool(&config.database_url).await?;
            archaeologist_db::run_migrations(&pool).await?;
            println!("Answer for: {question}");
        }
        Commands::Explain { target } => {
            init_tracing(&config.rust_log);
            tracing::info!("Explaining: {target}");
            let pool = archaeologist_db::create_pool(&config.database_url).await?;
            archaeologist_db::run_migrations(&pool).await?;
            println!("Explanation for: {target}");
        }
        Commands::History { symbol } => {
            init_tracing(&config.rust_log);
            tracing::info!("History for: {symbol}");
            let pool = archaeologist_db::create_pool(&config.database_url).await?;
            archaeologist_db::run_migrations(&pool).await?;
            println!("History for: {symbol}");
        }
        Commands::Impact { symbol } => {
            init_tracing(&config.rust_log);
            tracing::info!("Impact analysis for: {symbol}");
            let pool = archaeologist_db::create_pool(&config.database_url).await?;
            archaeologist_db::run_migrations(&pool).await?;
            println!("Impact analysis for: {symbol}");
        }
        Commands::Search {
            query,
            symbol_type: _,
            language: _,
        } => {
            init_tracing(&config.rust_log);
            tracing::info!("Searching: {query}");
            let pool = archaeologist_db::create_pool(&config.database_url).await?;
            archaeologist_db::run_migrations(&pool).await?;
            println!("Search results for: {query}");
        }
        Commands::Mcp { transport, port } => {
            init_tracing(&config.rust_log);
            tracing::info!("Starting MCP server (transport: {transport}, port: {port})");
            let pool = archaeologist_db::create_pool(&config.database_url).await?;
            archaeologist_db::run_migrations(&pool).await?;
            println!("MCP server started");
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
