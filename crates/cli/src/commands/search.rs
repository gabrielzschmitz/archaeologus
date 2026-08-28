//! `search` command — query indexed symbols and files.

use anyhow::{Context, Result};
use archaeologist_db::{create_pool, run_migrations};
use archaeologist_search::{
    code_search::{search_code, search_files, CodeQuery},
    symbol_search::{search_symbols, SymbolQuery},
};
use tracing::info;

/// Options for the `search` sub-command.
#[derive(Debug)]
pub struct SearchOptions {
    /// The search term.
    pub query: String,
    /// Filter by symbol type (e.g. "function", "class").
    pub symbol_type: Option<String>,
    /// Filter by language (e.g. "rust", "python").
    pub language: Option<String>,
    /// Search mode: "symbols" (default), "files", or "code".
    pub mode: SearchMode,
    /// Maximum results to display (default 20).
    pub limit: i64,
    /// Pagination offset (default 0).
    pub offset: i64,
    pub database_url: String,
    pub rust_log: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchMode {
    /// Search symbol names (default).
    Symbols,
    /// Search file paths.
    Files,
    /// Search symbol `raw_text` / code content.
    Code,
}

impl std::str::FromStr for SearchMode {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self> {
        match s.to_ascii_lowercase().as_str() {
            "symbols" | "symbol" => Ok(Self::Symbols),
            "files" | "file" => Ok(Self::Files),
            "code" => Ok(Self::Code),
            other => anyhow::bail!("unknown search mode '{other}'; choose symbols, files, or code"),
        }
    }
}

/// Entry point wired from `main.rs`.
///
/// # Errors
/// Propagates database errors.
pub async fn run(opts: SearchOptions) -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(&opts.rust_log)
        .try_init()
        .ok();

    let pool = create_pool(&opts.database_url)
        .await
        .context("connect to database")?;
    run_migrations(&pool).await.context("run migrations")?;

    info!(
        query = %opts.query,
        mode = ?opts.mode,
        "running search"
    );

    match opts.mode {
        SearchMode::Symbols => {
            let result = search_symbols(&pool, &build_symbol_query(&opts))
                .await
                .context("symbol search")?;

            print_range(
                "Symbols",
                &opts.query,
                result.total,
                result.offset,
                result.items.len(),
            );
            for sym in &result.items {
                println!(
                    "  [{lang}] {ty} {name}  @ {file}:{line}",
                    lang = sym.language,
                    ty = sym.symbol_type,
                    name = sym.name,
                    file = sym.file_id,
                    line = sym.line_start + 1,
                );
            }
            if result.items.is_empty() {
                println!("  (no results)");
            }
        }

        SearchMode::Files => {
            let result = search_files(&pool, &build_code_query(&opts))
                .await
                .context("file search")?;

            print_range(
                "Files",
                &opts.query,
                result.total,
                result.offset,
                result.items.len(),
            );
            for f in &result.items {
                let lang = f.language.as_deref().unwrap_or("?");
                println!("  [{lang}] {path}", path = f.path);
            }
            if result.items.is_empty() {
                println!("  (no results)");
            }
        }

        SearchMode::Code => {
            let result = search_code(&pool, &build_code_query(&opts))
                .await
                .context("code search")?;

            print_range(
                "Code",
                &opts.query,
                result.total,
                result.offset,
                result.items.len(),
            );
            for sym in &result.items {
                println!(
                    "  [{lang}] {ty} {name}  raw={raw:?}",
                    lang = sym.language,
                    ty = sym.symbol_type,
                    name = sym.name,
                    raw = sym.raw_text,
                );
            }
            if result.items.is_empty() {
                println!("  (no results)");
            }
        }
    }

    Ok(())
}

/// Build a [`SymbolQuery`] from the CLI options.
fn build_symbol_query(opts: &SearchOptions) -> SymbolQuery<'_> {
    let mut q = SymbolQuery::new(&opts.query)
        .limit(opts.limit)
        .offset(opts.offset);
    if let Some(ref t) = opts.symbol_type {
        q = q.symbol_type(t.as_str());
    }
    if let Some(ref l) = opts.language {
        q = q.language(l.as_str());
    }
    q
}

/// Build a [`CodeQuery`] from the CLI options.
fn build_code_query(opts: &SearchOptions) -> CodeQuery<'_> {
    let mut q = CodeQuery::new(&opts.query)
        .limit(opts.limit)
        .offset(opts.offset);
    if let Some(ref l) = opts.language {
        q = q.language(l.as_str());
    }
    q
}

/// Print a "showing X-Y of N" header line.
fn print_range(label: &str, query: &str, total: i64, offset: i64, shown: usize) {
    println!(
        "{label} matching {query:?}  ({total} total, showing {}-{}):",
        offset + 1,
        offset + i64::try_from(shown).unwrap_or(i64::MAX),
    );
}
