//! `history` command — show every commit that touched a symbol, blame
//! information, and a summary of how the symbol evolved over time.

use anyhow::{Context, Result};
use archaeologist_db::{
    create_pool,
    repositories::{get_commit, list_symbol_commits},
    run_migrations,
};
use archaeologist_search::symbol_search::{search_symbols, SymbolQuery};
use tracing::info;

/// Options for the `history` sub-command.
#[derive(Debug)]
pub struct HistoryOptions {
    /// Symbol name to inspect.
    pub symbol: String,
    /// Optional language filter (e.g. "rust", "go").
    pub language: Option<String>,
    pub database_url: String,
    pub rust_log: String,
}

/// Entry point wired from `main.rs`.
///
/// # Errors
/// Propagates database errors.
pub async fn run(opts: HistoryOptions) -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(&opts.rust_log)
        .try_init()
        .ok();

    let pool = create_pool(&opts.database_url)
        .await
        .context("connect to database")?;
    run_migrations(&pool).await.context("run migrations")?;

    info!(symbol = %opts.symbol, "history command");

    let mut q = SymbolQuery::new(&opts.symbol).limit(5);
    if let Some(ref lang) = opts.language {
        q = q.language(lang.as_str());
    }
    let result = search_symbols(&pool, &q).await.context("symbol search")?;

    if result.items.is_empty() {
        println!("No symbol matching {:?} found.", opts.symbol);
        println!("Tip: index a repository first with `archaeologist index <path>`.");
        return Ok(());
    }

    for sym in &result.items {
        let sc_links = list_symbol_commits(&pool, sym.id).await.unwrap_or_default();

        let mut commits = Vec::new();
        for link in &sc_links {
            if let Ok(Some(c)) = get_commit(&pool, link.commit_id).await {
                commits.push((c, link.change_type.clone()));
            }
        }

        // Sort by date descending (most recent first).
        commits.sort_by_key(|(a, _)| std::cmp::Reverse(a.author_date));

        println!(
            "Symbol: {} {} [{}]  (file: {})",
            sym.language, sym.name, sym.symbol_type, sym.file_id
        );
        println!("  Lines {}-{}", sym.line_start + 1, sym.line_end + 1);

        if let Some(doc) = &sym.doc_comment {
            if !doc.trim().is_empty() {
                println!("  Doc  : {}", doc.trim());
            }
        }

        if commits.is_empty() {
            println!("\n  No commit history found for this symbol.");
        } else {
            println!("\n  Commit history ({} commit(s)):", commits.len());
            println!(
                "  {:<10}  {:<12}  {:<20}  {:<12}  Message",
                "SHA", "Date", "Author", "Change"
            );
            println!("  {}", "─".repeat(90));

            for (c, change_type) in &commits {
                let date = c.author_date.format("%Y-%m-%d");
                let author = c.author_name.as_deref().unwrap_or("?");
                let msg = c.message.lines().next().unwrap_or("").trim();
                println!(
                    "  {sha:<10}  {date:<12}  {author:<20}  {ct:<12}  {msg}",
                    sha = &c.sha[..c.sha.len().min(8)],
                    ct = change_type,
                );
            }

            // ── Evolution summary ─────────────────────────────────────────────
            let added = commits.iter().filter(|(_, ct)| ct == "added").count();
            let deleted = commits.iter().filter(|(_, ct)| ct == "deleted").count();
            let modified = commits.iter().filter(|(_, ct)| ct == "modified").count();

            println!();
            println!(
                "  Evolution: {added} added, {modified} modified, {deleted} deleted across {} commit(s)",
                commits.len()
            );

            // Unique authors
            let mut authors: Vec<&str> = commits
                .iter()
                .filter_map(|(c, _)| c.author_name.as_deref())
                .collect();
            authors.dedup();
            authors.sort_unstable();
            authors.dedup();
            println!("  Authors  : {}", authors.join(", "));
        }

        println!("{}", "─".repeat(60));
    }

    Ok(())
}
