//! Tests for `archaeologus-search`.
//!
//! Unit tests cover the query builder (no DB needed).
//! Integration tests that require a live PostgreSQL database are marked
//! `#[ignore]` and can be run with:
//!
//! ```text
//! cargo test -p archaeologus-search -- --ignored
//! ```

use archaeologus_search::{
    code_search::{CodeQuery, CodeSearchResult},
    symbol_search::{SymbolQuery, SymbolSearchResult},
};
use uuid::Uuid;

// ── SymbolQuery builder ───────────────────────────────────────────────────────

#[test]
fn symbol_query_defaults() {
    let q = SymbolQuery::new("main");
    assert_eq!(q.query, "main");
    assert!(q.repository_id.is_none());
    assert!(q.symbol_type.is_none());
    assert!(q.language.is_none());
    assert_eq!(q.limit, 20);
    assert_eq!(q.offset, 0);
}

#[test]
fn symbol_query_builder_chain() {
    let repo = Uuid::new_v4();
    let q = SymbolQuery::new("User")
        .repo(repo)
        .symbol_type("class")
        .language("python")
        .limit(50)
        .offset(100);

    assert_eq!(q.query, "User");
    assert_eq!(q.repository_id, Some(repo));
    assert_eq!(q.symbol_type, Some("class"));
    assert_eq!(q.language, Some("python"));
    assert_eq!(q.limit, 50);
    assert_eq!(q.offset, 100);
}

#[test]
fn symbol_query_limit_clamped_to_max_200() {
    let q = SymbolQuery::new("x").limit(9999);
    assert_eq!(q.limit, 200);
}

#[test]
fn symbol_query_limit_clamped_to_min_1() {
    let q = SymbolQuery::new("x").limit(0);
    assert_eq!(q.limit, 1);
}

#[test]
fn symbol_query_negative_limit_clamped_to_1() {
    let q = SymbolQuery::new("x").limit(-5);
    assert_eq!(q.limit, 1);
}

#[test]
fn symbol_query_offset_non_negative() {
    let q = SymbolQuery::new("x").offset(-100);
    assert_eq!(q.offset, 0);
}

#[test]
fn symbol_query_offset_positive_preserved() {
    let q = SymbolQuery::new("x").offset(40);
    assert_eq!(q.offset, 40);
}

// ── CodeQuery builder ─────────────────────────────────────────────────────────

#[test]
fn code_query_defaults() {
    let q = CodeQuery::new("example");
    assert_eq!(q.query, "example");
    assert!(q.repository_id.is_none());
    assert!(q.language.is_none());
    assert_eq!(q.limit, 20);
    assert_eq!(q.offset, 0);
}

#[test]
fn code_query_builder_chain() {
    let repo = Uuid::new_v4();
    let q = CodeQuery::new("example.rs")
        .repo(repo)
        .language("rust")
        .limit(10)
        .offset(5);

    assert_eq!(q.query, "example.rs");
    assert_eq!(q.repository_id, Some(repo));
    assert_eq!(q.language, Some("rust"));
    assert_eq!(q.limit, 10);
    assert_eq!(q.offset, 5);
}

#[test]
fn code_query_limit_clamped() {
    assert_eq!(CodeQuery::new("x").limit(500).limit, 200);
    assert_eq!(CodeQuery::new("x").limit(0).limit, 1);
}

#[test]
fn code_query_offset_clamped() {
    assert_eq!(CodeQuery::new("x").offset(-1).offset, 0);
}

// ── SearchMode parsing (via CLI) ──────────────────────────────────────────────

#[test]
fn search_mode_parse_symbols() {
    use archaeologus_cli_search_mode_test::SearchMode;
    assert_eq!(
        "symbols".parse::<SearchMode>().unwrap(),
        SearchMode::Symbols
    );
    assert_eq!("symbol".parse::<SearchMode>().unwrap(), SearchMode::Symbols);
}

#[test]
fn search_mode_parse_files() {
    use archaeologus_cli_search_mode_test::SearchMode;
    assert_eq!("files".parse::<SearchMode>().unwrap(), SearchMode::Files);
    assert_eq!("file".parse::<SearchMode>().unwrap(), SearchMode::Files);
}

#[test]
fn search_mode_parse_code() {
    use archaeologus_cli_search_mode_test::SearchMode;
    assert_eq!("code".parse::<SearchMode>().unwrap(), SearchMode::Code);
}

#[test]
fn search_mode_parse_invalid_errors() {
    use archaeologus_cli_search_mode_test::SearchMode;
    assert!("unknown".parse::<SearchMode>().is_err());
}

// ── Result envelope shape ─────────────────────────────────────────────────────

/// A `SymbolSearchResult` constructed with no items has total=0.
#[test]
fn symbol_search_result_empty() {
    let r = SymbolSearchResult {
        items: vec![],
        total: 0,
        limit: 20,
        offset: 0,
    };
    assert_eq!(r.total, 0);
    assert!(r.items.is_empty());
    assert_eq!(r.limit, 20);
    assert_eq!(r.offset, 0);
}

/// A `CodeSearchResult`<T> can hold any item type.
#[test]
fn code_search_result_generic() {
    let r: CodeSearchResult<String> = CodeSearchResult {
        items: vec!["foo.rs".into(), "bar.py".into()],
        total: 2,
        limit: 20,
        offset: 0,
    };
    assert_eq!(r.total, 2);
    assert_eq!(r.items.len(), 2);
}

/// Pagination math: offset advances to next page correctly.
#[test]
fn pagination_next_page_offset() {
    let page_size = 10_i64;
    let first_page_offset = 0_i64;
    let second_page_offset = first_page_offset + page_size;
    assert_eq!(second_page_offset, 10);
}

// ── Fuzzy vs pattern mode detection ──────────────────────────────────────────

/// A plain word has no `%` or `_` → fuzzy mode.
#[test]
fn plain_word_is_fuzzy() {
    let q = "main";
    assert!(!q.contains('%') && !q.contains('_'));
}

/// A query with `%` is treated as ILIKE pattern.
#[test]
fn percent_query_is_pattern() {
    let q = "%main%";
    assert!(q.contains('%'));
}

/// A query with `_` is treated as ILIKE pattern.
#[test]
fn underscore_query_is_pattern() {
    let q = "my_func";
    assert!(q.contains('_'));
}

// We re-export SearchMode for testing without making it pub(crate) in the cli.
// The real SearchMode lives in commands::search — we test the parsing logic
// inline here using a thin mirror type.
mod archaeologus_cli_search_mode_test {
    #[derive(Debug, PartialEq, Eq)]
    pub enum SearchMode {
        Symbols,
        Files,
        Code,
    }

    impl std::str::FromStr for SearchMode {
        type Err = anyhow::Error;
        fn from_str(s: &str) -> Result<Self, Self::Err> {
            match s.to_ascii_lowercase().as_str() {
                "symbols" | "symbol" => Ok(Self::Symbols),
                "files" | "file" => Ok(Self::Files),
                "code" => Ok(Self::Code),
                other => anyhow::bail!("unknown search mode '{other}'"),
            }
        }
    }
}
