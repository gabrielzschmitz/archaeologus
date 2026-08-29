//! `ask` command — answer a natural-language question about the codebase.
//!
//! Workflow:
//! 1. Search symbols whose names fuzzy-match keywords extracted from the
//!    question.
//! 2. For every hit, fetch commits, dependencies, and DB evidence.
//! 3. Aggregate & deduplicate evidence with the `evidence` crate.
//! 4. Render a human-readable explanation via the `explainer`.

use anyhow::{Context, Result};
use archaeologist_db::{
    create_pool, repositories::get_evidence_for_symbol, repositories::list_symbol_commits,
    repositories::get_commit, run_migrations,
};
use archaeologist_evidence::{aggregate_evidence, explain_symbol};
use archaeologist_search::symbol_search::{search_symbols, SymbolQuery};
use tracing::info;

/// Options for the `ask` sub-command.
#[derive(Debug)]
pub struct AskOptions {
    /// The natural-language question.
    pub question: String,
    /// Optional language filter (e.g. "rust", "go").
    pub language: Option<String>,
    pub database_url: String,
    pub rust_log: String,
}

/// Entry point wired from `main.rs`.
///
/// # Errors
/// Propagates database errors.
pub async fn run(opts: AskOptions) -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(&opts.rust_log)
        .try_init()
        .ok();

    let pool = create_pool(&opts.database_url)
        .await
        .context("connect to database")?;
    run_migrations(&pool).await.context("run migrations")?;

    info!(question = %opts.question, "ask command");

    // ── 1. Extract keywords (simple whitespace split, strip punctuation) ──────
    let keywords = extract_keywords(&opts.question);
    info!(?keywords, "extracted keywords");

    if keywords.is_empty() {
        println!("Could not extract keywords from: {:?}", opts.question);
        return Ok(());
    }

    // ── 2. Search for relevant symbols for each keyword ───────────────────────
    let mut seen_ids = std::collections::HashSet::new();
    let mut symbols = Vec::new();

    for kw in &keywords {
        let mut q = SymbolQuery::new(kw).limit(5);
        if let Some(ref lang) = opts.language {
            q = q.language(lang.as_str());
        }
        let result = search_symbols(&pool, &q)
            .await
            .context("symbol search")?;
        for sym in result.items {
            if seen_ids.insert(sym.id) {
                symbols.push(sym);
            }
        }
    }

    if symbols.is_empty() {
        println!("No symbols found matching: {:?}", opts.question);
        println!("Tip: index a repository first with `archaeologist index <path>`.");
        return Ok(());
    }

    println!(
        "Question: {}\n",
        opts.question
    );

    // ── 3 & 4. Per-symbol: aggregate evidence + explain ───────────────────────
    for sym in symbols.iter().take(3) {
        // Fetch commits that touched this symbol.
        let sc_links = list_symbol_commits(&pool, sym.id).await.unwrap_or_default();
        let mut commits = Vec::new();
        for link in &sc_links {
            if let Ok(Some(c)) = get_commit(&pool, link.commit_id).await {
                commits.push(c);
            }
        }

        // Fetch existing DB evidence.
        let db_ev = get_evidence_for_symbol(&pool, sym.id)
            .await
            .unwrap_or_default();

        let evidence = aggregate_evidence(sym.id, Some(sym), &commits, &[], &db_ev);
        let explanation = explain_symbol(&sym.name, &evidence);

        println!("{}", explanation.to_display_string());
        println!("{}", "─".repeat(60));
    }

    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Split a question into lowercase alpha-numeric keyword tokens (≥ 3 chars).
pub fn extract_keywords(question: &str) -> Vec<String> {
    let stop_words = [
        "the", "a", "an", "is", "it", "in", "of", "for", "and", "or",
        "to", "what", "why", "how", "does", "do", "did", "was", "are",
        "this", "that", "with", "where", "when", "who", "which",
    ];
    question
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|t| t.len() >= 3)
        .map(|t| t.to_lowercase())
        .filter(|t| !stop_words.contains(&t.as_str()))
        .collect::<std::collections::LinkedList<_>>()
        .into_iter()
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keywords_extracted_from_question() {
        let kw = extract_keywords("What does the authenticate function do?");
        assert!(kw.contains(&"authenticate".to_string()));
        assert!(kw.contains(&"function".to_string()));
        // Stop words removed.
        assert!(!kw.contains(&"what".to_string()));
        assert!(!kw.contains(&"the".to_string()));
        assert!(!kw.contains(&"does".to_string()));
    }

    #[test]
    fn keywords_empty_question_returns_empty() {
        assert!(extract_keywords("").is_empty());
    }

    #[test]
    fn keywords_short_tokens_filtered_out() {
        let kw = extract_keywords("do it");
        // "do" (2) and "it" (2) both < 3 chars
        assert!(kw.is_empty());
    }

    #[test]
    fn keywords_deduplicates_words() {
        let kw = extract_keywords("parse parse parse");
        assert_eq!(kw.len(), 1);
    }
}
