//! `explain` command — show purpose, origin, author and history of a symbol
//! or file.

use anyhow::{Context, Result};
use archaeologist_db::{
    create_pool,
    repositories::{
        get_commit, get_evidence_for_symbol, get_file_by_path, list_repositories,
        list_symbol_commits, list_symbol_dependencies,
    },
    run_migrations,
};
use archaeologist_evidence::{aggregate_evidence, explain_symbol};
use archaeologist_search::symbol_search::{search_symbols, SymbolQuery};
use tracing::info;

/// Options for the `explain` sub-command.
#[derive(Debug)]
pub struct ExplainOptions {
    /// Symbol name or file path to explain.
    pub target: String,
    /// Optional language filter (e.g. "rust", "go").
    pub language: Option<String>,
    pub database_url: String,
    pub rust_log: String,
}

/// Entry point wired from `main.rs`.
///
/// # Errors
/// Propagates database errors.
pub async fn run(opts: ExplainOptions) -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(&opts.rust_log)
        .try_init()
        .ok();

    let pool = create_pool(&opts.database_url)
        .await
        .context("connect to database")?;
    run_migrations(&pool).await.context("run migrations")?;

    info!(target = %opts.target, "explain command");

    // Detect whether target looks like a file path (contains '/' or '.').
    let looks_like_path =
        opts.target.contains('/') || opts.target.contains('\\') || opts.target.contains('.');

    if looks_like_path {
        explain_file(&pool, &opts.target).await?;
    } else {
        explain_symbol_cmd(&pool, &opts.target, opts.language.as_deref()).await?;
    }

    Ok(())
}

// ── File explanation ──────────────────────────────────────────────────────────

async fn explain_file(pool: &archaeologist_db::PgPool, target: &str) -> Result<()> {
    // Look up the file across all known repositories.
    let repos = list_repositories(pool).await.context("list repositories")?;

    let mut found = false;
    for repo in &repos {
        if let Some(file) = get_file_by_path(pool, repo.id, target)
            .await
            .unwrap_or(None)
        {
            found = true;
            println!("File      : {}", file.path);
            println!("Repository: {} ({})", repo.name, repo.url);
            println!("Language  : {}", file.language.as_deref().unwrap_or("?"));
            println!("Size      : {} bytes", file.size_bytes);

            // List symbols in this file.
            let syms: Vec<archaeologist_core::models::Symbol> = sqlx::query_as(
                "SELECT id, file_id, repository_id, name, symbol_type, language,
                            line_start, line_end, col_start, col_end,
                            visibility, doc_comment, raw_text, created_at
                     FROM symbols WHERE file_id = $1 ORDER BY line_start",
            )
            .bind(file.id)
            .fetch_all(pool)
            .await
            .unwrap_or_default();

            if syms.is_empty() {
                println!("\nNo indexed symbols.");
            } else {
                println!("\nSymbols ({}):", syms.len());
                for s in &syms {
                    let vis = s.visibility.as_deref().unwrap_or("");
                    println!(
                        "  [{vis:>3}] {ty} {name}  (line {l})",
                        ty = s.symbol_type,
                        name = s.name,
                        l = s.line_start + 1,
                    );
                }
            }
            break;
        }
    }

    if !found {
        println!("No indexed file matching {target:?}.");
        println!("Tip: index a repository first with `archaeologist index <path>`.");
    }

    Ok(())
}

// ── Symbol explanation ────────────────────────────────────────────────────────

async fn explain_symbol_cmd(
    pool: &archaeologist_db::PgPool,
    target: &str,
    language: Option<&str>,
) -> Result<()> {
    let mut q = SymbolQuery::new(target).limit(5);
    if let Some(lang) = language {
        q = q.language(lang);
    }
    let result = search_symbols(pool, &q).await.context("symbol search")?;

    if result.items.is_empty() {
        println!("No symbol matching {target:?} found.");
        println!("Tip: index a repository first with `archaeologist index <path>`.");
        return Ok(());
    }

    for sym in &result.items {
        // Commits that touched this symbol.
        let sc_links = list_symbol_commits(pool, sym.id).await.unwrap_or_default();
        let mut commits = Vec::new();
        for link in &sc_links {
            if let Ok(Some(c)) = get_commit(pool, link.commit_id).await {
                commits.push(c);
            }
        }

        // Dependencies.
        let deps = list_symbol_dependencies(pool, sym.id)
            .await
            .unwrap_or_default();

        // Existing DB evidence.
        let db_ev = get_evidence_for_symbol(pool, sym.id)
            .await
            .unwrap_or_default();

        let evidence = aggregate_evidence(sym.id, Some(sym), &commits, &[], &db_ev);
        let explanation = explain_symbol(&sym.name, &evidence);

        println!("{}", explanation.to_display_string());

        // Show dependencies.
        if !deps.is_empty() {
            println!("Dependencies ({}):", deps.len());
            for d in &deps {
                println!(
                    "  [{ty}] {name}",
                    ty = d.dependency_type,
                    name = d.dependency_name
                );
            }
        }

        // Show recent commits.
        if !commits.is_empty() {
            println!("\nCommit history ({} commit(s)):", commits.len());
            for c in commits.iter().take(10) {
                let date = c.author_date.format("%Y-%m-%d");
                let author = c.author_name.as_deref().unwrap_or("?");
                let msg = c.message.lines().next().unwrap_or("").trim();
                println!("  {sha:.8}  {date}  {author}  {msg}", sha = c.sha);
            }
        }

        println!("{}", "─".repeat(60));
    }

    Ok(())
}
