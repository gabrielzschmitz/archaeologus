//! Symbol search backed by PostgreSQL `pg_trgm`.
//!
//! Two search modes:
//!
//! * **Fuzzy** — `similarity(name, $query) > 0.1`, ordered by descending
//!   similarity score.  Good for typo-tolerant look-up.
//! * **Prefix / exact** — `name ILIKE $query%`, ordered by name length
//!   (shortest = most specific first).  Used when the caller wraps the query
//!   in `%…%` wildcards themselves, or passes an exact token.
//!
//! Both modes share the same optional filters and pagination parameters
//! through [`SymbolQuery`].

use archaeologist_core::models::Symbol;
use sqlx::PgPool;
use tracing::debug;
use uuid::Uuid;

/// Input for a symbol search.
#[derive(Debug, Clone)]
pub struct SymbolQuery<'a> {
    /// The search term.  Fuzzy matching is applied when this is a plain word;
    /// pass `%word%` to force a LIKE pattern instead.
    pub query: &'a str,
    /// Restrict results to a single repository.  `None` searches all repos.
    pub repository_id: Option<Uuid>,
    /// Filter by `symbol_type` string (e.g. `"function"`, `"class"`).
    pub symbol_type: Option<&'a str>,
    /// Filter by language (e.g. `"rust"`, `"python"`).
    pub language: Option<&'a str>,
    /// Maximum rows to return (default 20, max 200).
    pub limit: i64,
    /// Row offset for pagination (default 0).
    pub offset: i64,
}

impl<'a> SymbolQuery<'a> {
    #[must_use]
    pub fn new(query: &'a str) -> Self {
        Self {
            query,
            repository_id: None,
            symbol_type: None,
            language: None,
            limit: 20,
            offset: 0,
        }
    }

    #[must_use]
    pub fn repo(mut self, id: Uuid) -> Self {
        self.repository_id = Some(id);
        self
    }

    #[must_use]
    pub fn symbol_type(mut self, t: &'a str) -> Self {
        self.symbol_type = Some(t);
        self
    }

    #[must_use]
    pub fn language(mut self, l: &'a str) -> Self {
        self.language = Some(l);
        self
    }

    #[must_use]
    pub fn limit(mut self, n: i64) -> Self {
        self.limit = n.clamp(1, 200);
        self
    }

    #[must_use]
    pub fn offset(mut self, n: i64) -> Self {
        self.offset = n.max(0);
        self
    }
}

/// Result envelope returned by [`search_symbols`].
#[derive(Debug, Clone)]
pub struct SymbolSearchResult {
    pub items: Vec<Symbol>,
    /// Total matching rows (without limit/offset), for pagination metadata.
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

/// Search symbols using `pg_trgm` fuzzy matching.
///
/// When `q.query` contains `%` it is used as a raw `ILIKE` pattern; otherwise
/// `similarity()` is used with a 0.1 threshold so even partial matches surface.
///
/// # Errors
/// Propagates `sqlx::Error` on database failure.
pub async fn search_symbols(
    pool: &PgPool,
    q: &SymbolQuery<'_>,
) -> Result<SymbolSearchResult, sqlx::Error> {
    let effective_limit = q.limit.clamp(1, 200);
    let effective_offset = q.offset.max(0);
    let is_pattern = q.query.contains('%') || q.query.contains('_');

    debug!(
        query = q.query,
        repo = ?q.repository_id,
        symbol_type = ?q.symbol_type,
        language = ?q.language,
        limit = effective_limit,
        offset = effective_offset,
        "symbol search"
    );

    // ── Count query ───────────────────────────────────────────────────────────
    let total: i64 = if is_pattern {
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM symbols
             WHERE name ILIKE $1
             AND ($2::UUID IS NULL OR repository_id = $2)
             AND ($3::TEXT IS NULL OR symbol_type = $3)
             AND ($4::TEXT IS NULL OR language = $4)",
        )
        .bind(q.query)
        .bind(q.repository_id)
        .bind(q.symbol_type)
        .bind(q.language)
        .fetch_one(pool)
        .await?
    } else {
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM symbols
             WHERE similarity(name, $1) > 0.1
             AND ($2::UUID IS NULL OR repository_id = $2)
             AND ($3::TEXT IS NULL OR symbol_type = $3)
             AND ($4::TEXT IS NULL OR language = $4)",
        )
        .bind(q.query)
        .bind(q.repository_id)
        .bind(q.symbol_type)
        .bind(q.language)
        .fetch_one(pool)
        .await?
    };

    // ── Data query ────────────────────────────────────────────────────────────
    let items: Vec<Symbol> = if is_pattern {
        sqlx::query_as(
            "SELECT id, file_id, repository_id, name, symbol_type, language,
                    line_start, line_end, col_start, col_end,
                    visibility, doc_comment, raw_text, created_at
             FROM symbols
             WHERE name ILIKE $1
             AND ($2::UUID IS NULL OR repository_id = $2)
             AND ($3::TEXT IS NULL OR symbol_type = $3)
             AND ($4::TEXT IS NULL OR language = $4)
             ORDER BY length(name), name
             LIMIT $5 OFFSET $6",
        )
        .bind(q.query)
        .bind(q.repository_id)
        .bind(q.symbol_type)
        .bind(q.language)
        .bind(effective_limit)
        .bind(effective_offset)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as(
            "SELECT id, file_id, repository_id, name, symbol_type, language,
                    line_start, line_end, col_start, col_end,
                    visibility, doc_comment, raw_text, created_at
             FROM symbols
             WHERE similarity(name, $1) > 0.1
             AND ($2::UUID IS NULL OR repository_id = $2)
             AND ($3::TEXT IS NULL OR symbol_type = $3)
             AND ($4::TEXT IS NULL OR language = $4)
             ORDER BY similarity(name, $1) DESC, name
             LIMIT $5 OFFSET $6",
        )
        .bind(q.query)
        .bind(q.repository_id)
        .bind(q.symbol_type)
        .bind(q.language)
        .bind(effective_limit)
        .bind(effective_offset)
        .fetch_all(pool)
        .await?
    };

    Ok(SymbolSearchResult {
        items,
        total,
        limit: effective_limit,
        offset: effective_offset,
    })
}
