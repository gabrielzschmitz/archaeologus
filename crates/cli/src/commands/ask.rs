//! `ask` command — answer a natural-language question about the codebase.
//!
//! Workflow:
//! 1. Extract keywords from the question.
//! 2. Search symbols whose names fuzzy-match those keywords.
//! 3. For every hit, fetch commits, dependencies, and DB evidence.
//! 4. Aggregate & deduplicate evidence with the `evidence` crate.
//! 5. Build a rich context prompt (question + symbols + evidence).
//! 6. Send to the configured LLM provider and print the AI answer.
//!    Falls back to the plain rule-based explainer if no LLM is configured.

use anyhow::{Context, Result};
use archaeologist_core::models::Symbol;
use archaeologist_db::{
    create_pool, repositories::get_commit, repositories::get_evidence_for_symbol,
    repositories::list_symbol_commits, run_migrations,
};
use archaeologist_evidence::{aggregate_evidence, explain_symbol, EvidenceItem, Explanation};
use archaeologist_llm::{
    build_ask_prompt, create_provider, system_prompt, LLMConfig, SymbolContext,
};
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

    // ── 1. Extract keywords ───────────────────────────────────────────────────
    let keywords = extract_keywords(&opts.question);
    info!(?keywords, "extracted keywords");

    if keywords.is_empty() {
        println!("Could not extract keywords from: {:?}", opts.question);
        return Ok(());
    }

    // ── 2. Search for relevant symbols ────────────────────────────────────────
    let mut seen_ids = std::collections::HashSet::new();
    let mut symbols = Vec::new();

    for kw in &keywords {
        let mut q = SymbolQuery::new(kw).limit(5);
        if let Some(ref lang) = opts.language {
            q = q.language(lang.as_str());
        }
        let result = search_symbols(&pool, &q).await.context("symbol search")?;
        for sym in result.items {
            if seen_ids.insert(sym.id) {
                symbols.push(sym);
            }
        }
    }

    println!("Question: {}\n", opts.question);

    if symbols.is_empty() {
        println!("No symbols found matching: {:?}", opts.question);
        println!("Tip: index a repository first with `archaeologist index <path>`.");
    }

    // ── 3 & 4. Per-symbol: aggregate evidence ────────────────────────────────
    let mut llm_contexts: Vec<(Symbol, Vec<EvidenceItem>, Explanation)> = Vec::new();

    for sym in symbols.iter().take(3) {
        let sc_links = list_symbol_commits(&pool, sym.id).await.unwrap_or_default();
        let mut commits = Vec::new();
        for link in &sc_links {
            if let Ok(Some(c)) = get_commit(&pool, link.commit_id).await {
                commits.push(c);
            }
        }

        let db_ev = get_evidence_for_symbol(&pool, sym.id)
            .await
            .unwrap_or_default();

        let evidence = aggregate_evidence(sym.id, Some(sym), &commits, &[], &db_ev);
        let explanation = explain_symbol(&sym.name, &evidence);
        llm_contexts.push((sym.clone(), evidence, explanation));
    }

    // ── 5 & 6. LLM answer or rule-based fallback ─────────────────────────────
    match try_llm_answer(&opts.question, &llm_contexts).await {
        Ok(answer) => {
            println!("{answer}");
        }
        Err(e) => {
            // LLM unavailable or not configured — degrade gracefully.
            // Log the full chain so users can diagnose provider issues.
            tracing::warn!("LLM unavailable — falling back to rule-based explainer");
            tracing::warn!("  cause: {e:#}");
            for (_, _, explanation) in &llm_contexts {
                println!("{}", explanation.to_display_string());
                println!("{}", "─".repeat(60));
            }
            if llm_contexts.is_empty() {
                println!(
                    "No evidence found. Index a repository first with \
                     `archaeologist index <path>`."
                );
            }
        }
    }

    Ok(())
}

/// Try to answer via the configured LLM provider.
///
/// Returns the answer string, or an error if no provider is configured /
/// the provider call fails.
async fn try_llm_answer(
    question: &str,
    contexts: &[(Symbol, Vec<EvidenceItem>, Explanation)],
) -> Result<String> {
    let config =
        LLMConfig::from_env().context("read LLM config — set LLM_PROVIDER (default: watsonx)")?;

    let provider =
        create_provider(&config).context("initialise LLM provider — check provider env vars")?;

    info!(provider = %provider.name(), "sending question to LLM");

    let sym_contexts: Vec<SymbolContext<'_>> = contexts
        .iter()
        .map(|(sym, ev, expl)| SymbolContext {
            symbol: sym,
            evidence: ev,
            explanation: expl,
        })
        .collect();

    let messages = vec![system_prompt(), build_ask_prompt(question, &sym_contexts)];

    let config_temp = config.temperature;
    let config_max = config.max_tokens;

    let response = provider
        .chat(&messages, config_temp, config_max)
        .await
        .context("LLM chat call")?;

    info!(
        model = %response.model,
        tokens = ?response.tokens_used,
        "LLM response received"
    );

    Ok(response.content)
}

// ── Helpers ───────────────────────────────────────────────────────────────────

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
