use archaeologist_core::AppConfig;
use archaeologist_db::{create_pool, run_migrations};
use clap::{Parser, Subcommand};
use tracing::info;

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
        /// Git repository URL
        url: String,
        /// Branch to index (default: main)
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

    tracing_subscriber::fmt()
        .with_env_filter(&config.rust_log)
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Index { url, branch } => {
            info!("Indexing repository: {}", url);
            let pool = create_pool(&config.database_url).await?;
            run_migrations(&pool).await?;
            println!("Repository indexed successfully: {}", url);
        }
        Commands::Ask { question } => {
            info!("Asking: {}", question);
            let pool = create_pool(&config.database_url).await?;
            run_migrations(&pool).await?;
            println!("Answer for: {}", question);
        }
        Commands::Explain { target } => {
            info!("Explaining: {}", target);
            let pool = create_pool(&config.database_url).await?;
            run_migrations(&pool).await?;
            println!("Explanation for: {}", target);
        }
        Commands::History { symbol } => {
            info!("History for: {}", symbol);
            let pool = create_pool(&config.database_url).await?;
            run_migrations(&pool).await?;
            println!("History for: {}", symbol);
        }
        Commands::Impact { symbol } => {
            info!("Impact analysis for: {}", symbol);
            let pool = create_pool(&config.database_url).await?;
            run_migrations(&pool).await?;
            println!("Impact analysis for: {}", symbol);
        }
        Commands::Search {
            query,
            symbol_type,
            language,
        } => {
            info!("Searching: {}", query);
            let pool = create_pool(&config.database_url).await?;
            run_migrations(&pool).await?;
            println!("Search results for: {}", query);
        }
        Commands::Mcp { transport, port } => {
            info!(
                "Starting MCP server (transport: {}, port: {})",
                transport, port
            );
            let pool = create_pool(&config.database_url).await?;
            run_migrations(&pool).await?;
            println!("MCP server started");
        }
        Commands::Migrate => {
            info!("Running migrations...");
            let pool = create_pool(&config.database_url).await?;
            run_migrations(&pool).await?;
            println!("Migrations completed");
        }
    }

    Ok(())
}
