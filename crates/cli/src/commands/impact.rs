//! `impact` command — analyse the blast radius of changing a symbol.
//!
//! Reports:
//! * Direct callers (symbols that depend on the target)
//! * Indirect callers (one hop further)
//! * Tests that reference the symbol
//! * A risk estimate based on caller count and test coverage

use anyhow::{Context, Result};
use archaeologist_core::models::Symbol;
use archaeologist_db::{create_pool, run_migrations, PgPool};
use archaeologist_search::symbol_search::{search_symbols, SymbolQuery};
use tracing::info;
use uuid::Uuid;

/// Options for the `impact` sub-command.
#[derive(Debug)]
pub struct ImpactOptions {
    /// Symbol name to analyse.
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
pub async fn run(opts: ImpactOptions) -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(&opts.rust_log)
        .try_init()
        .ok();

    let pool = create_pool(&opts.database_url)
        .await
        .context("connect to database")?;
    run_migrations(&pool).await.context("run migrations")?;

    info!(symbol = %opts.symbol, "impact command");

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
        print_impact(&pool, sym).await;
        println!("{}", "─".repeat(60));
    }

    Ok(())
}

// ── Core impact analysis ──────────────────────────────────────────────────────

async fn print_impact(pool: &PgPool, sym: &Symbol) {
    println!(
        "Impact analysis: {} {} [{}]",
        sym.language, sym.name, sym.symbol_type
    );

    // ── Direct callers ────────────────────────────────────────────────────────
    let direct_callers = find_direct_callers(pool, sym.id).await;

    // ── Indirect callers (callers of callers) ─────────────────────────────────
    let mut indirect_callers: Vec<Symbol> = Vec::new();
    let mut seen_ids: std::collections::HashSet<Uuid> =
        direct_callers.iter().map(|s| s.id).collect();
    seen_ids.insert(sym.id);

    for caller in &direct_callers {
        let second_level = find_direct_callers(pool, caller.id).await;
        for s in second_level {
            if seen_ids.insert(s.id) {
                indirect_callers.push(s);
            }
        }
    }

    // ── Tests ─────────────────────────────────────────────────────────────────
    let test_callers: Vec<&Symbol> = direct_callers
        .iter()
        .chain(indirect_callers.iter())
        .filter(|s| is_test_symbol(s))
        .collect();

    // ── Print results ─────────────────────────────────────────────────────────
    let total_callers = direct_callers.len() + indirect_callers.len();

    if direct_callers.is_empty() {
        println!("  No direct callers found.");
    } else {
        println!("  Direct callers ({}):", direct_callers.len());
        for caller in &direct_callers {
            let test_tag = if is_test_symbol(caller) {
                " [TEST]"
            } else {
                ""
            };
            println!(
                "    [{lang}] {ty} {name}{test_tag}",
                lang = caller.language,
                ty = caller.symbol_type,
                name = caller.name,
            );
        }
    }

    if !indirect_callers.is_empty() {
        println!("  Indirect callers ({}):", indirect_callers.len());
        for caller in &indirect_callers {
            let test_tag = if is_test_symbol(caller) {
                " [TEST]"
            } else {
                ""
            };
            println!(
                "    [{lang}] {ty} {name}{test_tag}",
                lang = caller.language,
                ty = caller.symbol_type,
                name = caller.name,
            );
        }
    }

    if test_callers.is_empty() {
        println!("  Tests: none found — consider adding tests before changing this symbol.");
    } else {
        println!("  Tests covering this symbol ({}):", test_callers.len());
        for t in &test_callers {
            println!("    {}", t.name);
        }
    }

    // ── Risk estimate ─────────────────────────────────────────────────────────
    let risk = estimate_risk(total_callers, test_callers.len());
    println!();
    println!("  Risk estimate : {risk}");
    println!(
        "  Total affected: {} symbol(s) ({} direct, {} indirect)",
        total_callers,
        direct_callers.len(),
        indirect_callers.len()
    );
    println!("  Test coverage : {} test(s)", test_callers.len());
}

/// Find all symbols that have a dependency on `target_id`.
async fn find_direct_callers(pool: &PgPool, target_id: Uuid) -> Vec<Symbol> {
    // Look up symbol_dependencies where depends_on_symbol_id = target_id,
    // then JOIN to get the owning symbol record.
    let rows: Vec<Symbol> = sqlx::query_as(
        "SELECT s.id, s.file_id, s.repository_id, s.name, s.symbol_type, s.language,
                s.line_start, s.line_end, s.col_start, s.col_end,
                s.visibility, s.doc_comment, s.raw_text, s.created_at
         FROM symbol_dependencies sd
         JOIN symbols s ON s.id = sd.symbol_id
         WHERE sd.depends_on_symbol_id = $1",
    )
    .bind(target_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    rows
}

/// Return `true` if the symbol looks like a test function.
#[must_use]
pub fn is_test_symbol(sym: &Symbol) -> bool {
    let name_lc = sym.name.to_lowercase();
    name_lc.starts_with("test")
        || name_lc.ends_with("_test")
        || name_lc.contains("_test_")
        || sym
            .doc_comment
            .as_deref()
            .is_some_and(|d| d.contains("#[test]") || d.contains("@test"))
}

/// Produce a qualitative risk label.
#[must_use]
pub fn estimate_risk(total_callers: usize, test_count: usize) -> &'static str {
    match (total_callers, test_count) {
        (0, _) => "LOW — no known callers, safe to change",
        (1..=5, t) if t > 0 => "LOW — few callers and tests exist",
        (1..=5, _) => "MEDIUM — few callers but no tests",
        (6..=20, t) if t > 0 => "MEDIUM — moderate callers, some test coverage",
        (6..=20, _) => "HIGH — moderate callers with no tests",
        (_, t) if t > 0 => "HIGH — many callers; test coverage present but change carefully",
        _ => "CRITICAL — many callers and no tests; high risk of regression",
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn risk_no_callers_is_low() {
        assert_eq!(
            estimate_risk(0, 0),
            "LOW — no known callers, safe to change"
        );
    }

    #[test]
    fn risk_few_callers_with_tests_is_low() {
        let r = estimate_risk(3, 2);
        assert!(r.starts_with("LOW"));
    }

    #[test]
    fn risk_many_callers_no_tests_is_critical() {
        let r = estimate_risk(50, 0);
        assert!(r.starts_with("CRITICAL"));
    }

    #[test]
    fn risk_many_callers_with_tests_is_high() {
        let r = estimate_risk(50, 5);
        assert!(r.starts_with("HIGH"));
    }

    #[test]
    fn test_symbol_detection_by_name_prefix() {
        use chrono::Utc;
        let sym = archaeologist_core::models::Symbol {
            id: uuid::Uuid::new_v4(),
            file_id: uuid::Uuid::new_v4(),
            repository_id: uuid::Uuid::new_v4(),
            name: "test_authenticate".to_string(),
            symbol_type: "function".to_string(),
            language: "rust".to_string(),
            line_start: 1,
            line_end: 10,
            col_start: 0,
            col_end: 0,
            visibility: None,
            doc_comment: None,
            raw_text: String::new(),
            created_at: Utc::now(),
        };
        assert!(is_test_symbol(&sym));
    }

    #[test]
    fn test_symbol_detection_by_name_suffix() {
        use chrono::Utc;
        let sym = archaeologist_core::models::Symbol {
            id: uuid::Uuid::new_v4(),
            file_id: uuid::Uuid::new_v4(),
            repository_id: uuid::Uuid::new_v4(),
            name: "auth_test".to_string(),
            symbol_type: "function".to_string(),
            language: "rust".to_string(),
            line_start: 1,
            line_end: 10,
            col_start: 0,
            col_end: 0,
            visibility: None,
            doc_comment: None,
            raw_text: String::new(),
            created_at: Utc::now(),
        };
        assert!(is_test_symbol(&sym));
    }

    #[test]
    fn non_test_symbol_not_detected() {
        use chrono::Utc;
        let sym = archaeologist_core::models::Symbol {
            id: uuid::Uuid::new_v4(),
            file_id: uuid::Uuid::new_v4(),
            repository_id: uuid::Uuid::new_v4(),
            name: "authenticate".to_string(),
            symbol_type: "function".to_string(),
            language: "rust".to_string(),
            line_start: 1,
            line_end: 10,
            col_start: 0,
            col_end: 0,
            visibility: None,
            doc_comment: None,
            raw_text: String::new(),
            created_at: Utc::now(),
        };
        assert!(!is_test_symbol(&sym));
    }
}
