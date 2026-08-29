//! `ask` command — answer a natural-language question about the codebase.
//!
//! Workflow:
//! 1. Extract keywords from the question.
//! 2. Optionally resolve `--repo` to a repository UUID via name/URL match.
//! 3. Search symbols whose names fuzzy-match those keywords, restricted to the
//!    resolved repo (or all repos if none specified).
//! 4. For every hit, fetch: file, repository, deps, sibling symbols, commits,
//!    and DB evidence.
//! 5. Build a rich context prompt with all of the above.
//! 6. Send to the configured LLM provider and print the AI answer.
//!    Falls back to the plain rule-based explainer if no LLM is configured.

use anyhow::{Context, Result};
use archaeologist_core::models::{File, Repository, Symbol, SymbolDependency};
use archaeologist_db::{
    create_pool, repositories::get_commit, repositories::get_evidence_for_symbol,
    repositories::get_file, repositories::get_repository, repositories::list_repositories,
    repositories::list_symbol_commits, repositories::list_symbol_dependencies, run_migrations,
};
use archaeologist_evidence::{aggregate_evidence, explain_symbol, EvidenceItem, Explanation};
use archaeologist_llm::{
    build_ask_prompt, create_provider, system_prompt, LLMConfig, SymbolContext,
};
use archaeologist_search::symbol_search::{search_symbols, SymbolQuery};
use sqlx::PgPool;
use tracing::info;
use uuid::Uuid;

/// Options for the `ask` sub-command.
#[derive(Debug)]
pub struct AskOptions {
    /// The natural-language question.
    pub question: String,
    /// Optional language filter (e.g. "rust", "go").
    pub language: Option<String>,
    /// Optional repository filter: name, URL substring, or UUID string.
    pub repo: Option<String>,
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

    // ── 1. Resolve --repo to a UUID ───────────────────────────────────────────
    let repo_filter = if let Some(ref repo_hint) = opts.repo {
        let id = resolve_repo(&pool, repo_hint)
            .await
            .with_context(|| format!("resolve repository '{repo_hint}'"))?;
        info!(repo_id = %id, hint = %repo_hint, "repository filter applied");
        Some(id)
    } else {
        None
    };

    // ── 2. Extract keywords ───────────────────────────────────────────────────
    let keywords = extract_keywords(&opts.question);
    info!(?keywords, "extracted keywords");

    if keywords.is_empty() {
        println!("Could not extract keywords from: {:?}", opts.question);
        return Ok(());
    }

    // ── 3. Search for relevant symbols ────────────────────────────────────────
    let symbols = search_question_symbols(&pool, &opts, keywords, repo_filter).await?;

    println!("Question: {}\n", opts.question);

    if symbols.is_empty() {
        println!("No symbols found matching: {:?}", opts.question);
        if opts.repo.is_some() {
            println!(
                "Tip: check that the repository is indexed and --repo matches its name or URL."
            );
        } else {
            println!("Tip: index a repository first with `archaeologist index <path>`.");
        }
    }

    // ── 4. Per-symbol: fetch rich context + aggregate evidence ────────────────
    let bundles = build_bundles(&pool, &symbols).await;

    // ── 5 & 6. LLM answer or rule-based fallback ─────────────────────────────
    match try_llm_answer(&opts.question, &bundles).await {
        Ok(answer) => {
            println!("{answer}");
        }
        Err(e) => {
            tracing::warn!("LLM unavailable — falling back to rule-based explainer");
            tracing::warn!("  cause: {e:#}");
            for bundle in &bundles {
                println!("{}", bundle.explanation.to_display_string());
                println!("{}", "─".repeat(60));
            }
            if bundles.is_empty() {
                println!(
                    "No evidence found. Index a repository first with \
                     `archaeologist index <path>`."
                );
            }
        }
    }

    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Search symbols matching the extracted keywords, optionally restricted to a
/// repository. Deduplicates results across keywords.
async fn search_question_symbols(
    pool: &PgPool,
    opts: &AskOptions,
    keywords: Vec<String>,
    repo_filter: Option<Uuid>,
) -> Result<Vec<Symbol>> {
    let mut seen_ids = std::collections::HashSet::new();
    let mut symbols = Vec::new();

    for kw in &keywords {
        let mut q = SymbolQuery::new(kw).limit(5);
        if let Some(ref lang) = opts.language {
            q = q.language(lang.as_str());
        }
        if let Some(rid) = repo_filter {
            q = q.repo(rid);
        }
        let result = search_symbols(pool, &q).await.context("symbol search")?;
        for sym in result.items {
            if seen_ids.insert(sym.id) {
                symbols.push(sym);
            }
        }
    }

    Ok(symbols)
}

/// Fetch rich context (file, repo, deps, siblings, evidence) for the top
/// symbols and bundle it for the prompt builder / fallback explainer.
async fn build_bundles(pool: &PgPool, symbols: &[Symbol]) -> Vec<SymbolBundle> {
    let mut bundles: Vec<SymbolBundle> = Vec::new();

    for sym in symbols.iter().take(3) {
        // File that contains this symbol.
        let file = get_file(pool, sym.file_id).await.unwrap_or_default();

        // Repository the symbol belongs to.
        let repo = get_repository(pool, sym.repository_id)
            .await
            .unwrap_or_default();

        // Dependency edges.
        let deps = list_symbol_dependencies(pool, sym.id)
            .await
            .unwrap_or_default();

        // Sibling symbols: all symbols in the same file, excluding self.
        let siblings = if let Some(ref f) = file {
            fetch_siblings(pool, f.id, sym.id).await
        } else {
            vec![]
        };

        // Commit history for evidence.
        let sc_links = list_symbol_commits(pool, sym.id).await.unwrap_or_default();
        let mut commits = Vec::new();
        for link in &sc_links {
            if let Ok(Some(c)) = get_commit(pool, link.commit_id).await {
                commits.push(c);
            }
        }

        let db_ev = get_evidence_for_symbol(pool, sym.id)
            .await
            .unwrap_or_default();

        let evidence = aggregate_evidence(sym.id, Some(sym), &commits, &[], &db_ev);
        let explanation = explain_symbol(&sym.name, &evidence);

        bundles.push(SymbolBundle {
            symbol: sym.clone(),
            file,
            repo,
            deps,
            siblings,
            evidence,
            explanation,
        });
    }

    bundles
}

/// Resolve a user-supplied `--repo` hint to a repository UUID.
///
/// Accepts (in order of preference):
/// 1. A literal UUID string
/// 2. An exact name match
/// 3. A URL substring match
/// 4. A case-insensitive name substring match
///
/// # Errors
/// Returns an error if no repository matches the hint.
pub async fn resolve_repo(pool: &PgPool, hint: &str) -> Result<Uuid> {
    // 1. Literal UUID.
    if let Ok(id) = hint.parse::<Uuid>() {
        return Ok(id);
    }

    let repos = list_repositories(pool).await.context("list repositories")?;

    // 2. Exact name.
    if let Some(r) = repos.iter().find(|r| r.name == hint) {
        return Ok(r.id);
    }

    // 3. URL substring.
    if let Some(r) = repos.iter().find(|r| r.url.contains(hint)) {
        return Ok(r.id);
    }

    // 4. Case-insensitive name substring.
    let hint_lc = hint.to_lowercase();
    if let Some(r) = repos
        .iter()
        .find(|r| r.name.to_lowercase().contains(&hint_lc))
    {
        return Ok(r.id);
    }

    let names: Vec<&str> = repos.iter().map(|r| r.name.as_str()).collect();
    anyhow::bail!(
        "no repository matches '{hint}'. Indexed repositories: [{}]",
        names.join(", ")
    )
}

/// Fetch all symbols in the same file as a given symbol, excluding `self_id`.
async fn fetch_siblings(pool: &PgPool, file_id: Uuid, self_id: Uuid) -> Vec<Symbol> {
    sqlx::query_as::<_, Symbol>(
        "SELECT id, file_id, repository_id, name, symbol_type, language,
                line_start, line_end, col_start, col_end,
                visibility, doc_comment, raw_text, created_at
         FROM symbols
         WHERE file_id = $1 AND id != $2
         ORDER BY line_start
         LIMIT 30",
    )
    .bind(file_id)
    .bind(self_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
}

/// Try to answer via the configured LLM provider.
async fn try_llm_answer(question: &str, bundles: &[SymbolBundle]) -> Result<String> {
    let config =
        LLMConfig::from_env().context("read LLM config — set LLM_PROVIDER (default: watsonx)")?;

    let provider =
        create_provider(&config).context("initialise LLM provider — check provider env vars")?;

    info!(provider = %provider.name(), "sending question to LLM");

    let sym_contexts: Vec<SymbolContext<'_>> = bundles
        .iter()
        .map(|b| SymbolContext {
            symbol: &b.symbol,
            file: b.file.as_ref(),
            repo: b.repo.as_ref(),
            deps: &b.deps,
            siblings: &b.siblings,
            evidence: &b.evidence,
            explanation: &b.explanation,
        })
        .collect();

    let messages = vec![system_prompt(), build_ask_prompt(question, &sym_contexts)];

    let response = provider
        .chat(&messages, config.temperature, config.max_tokens)
        .await
        .context("LLM chat call")?;

    info!(
        model = %response.model,
        tokens = ?response.tokens_used,
        "LLM response received"
    );

    Ok(response.content)
}

// Private alias so `try_llm_answer` can name it without `self::`.
struct SymbolBundle {
    symbol: Symbol,
    file: Option<File>,
    repo: Option<Repository>,
    deps: Vec<SymbolDependency>,
    siblings: Vec<Symbol>,
    evidence: Vec<EvidenceItem>,
    explanation: Explanation,
}

/// Split a question into lowercase alpha-numeric keyword tokens (≥ 3 chars).
#[must_use]
pub fn extract_keywords(question: &str) -> Vec<String> {
    let stop_words = [
        "the", "a", "an", "is", "it", "in", "of", "for", "and", "or", "to", "what", "why", "how",
        "does", "do", "did", "was", "are", "this", "that", "with", "where", "when", "who", "which",
    ];
    question
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|t| t.len() >= 3)
        .map(str::to_lowercase)
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
        assert!(kw.is_empty());
    }

    #[test]
    fn keywords_deduplicates_words() {
        let kw = extract_keywords("parse parse parse");
        assert_eq!(kw.len(), 1);
    }
}
