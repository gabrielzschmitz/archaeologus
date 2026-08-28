//! Code search — find files and symbols by raw text content.
//!
//! Two entry points:
//!
//! * [`search_files`] — searches the `files` table by path (ILIKE) and
//!   language, returning matching [`File`] records with their repository.
//! * [`search_code`] — searches `symbols.raw_text` (and symbol name) using
//!   `pg_trgm` similarity + ILIKE, returning the matching symbols so the
//!   caller can locate the exact source lines.

use archaeologist_core::models::{File, Symbol};
use sqlx::PgPool;
use tracing::debug;
use uuid::Uuid;

/// Input for a file-path / code search.
#[derive(Debug, Clone)]
pub struct CodeQuery<'a> {
    /// Search term matched against file paths (ILIKE `%query%`) and symbol
    /// `raw_text` (`pg_trgm` similarity > 0.1).
    pub query: &'a str,
    /// Restrict to one repository.
    pub repository_id: Option<Uuid>,
    /// Filter by language (e.g. `"go"`, `"typescript"`).
    pub language: Option<&'a str>,
    /// Maximum rows (default 20, max 200).
    pub limit: i64,
    /// Row offset (default 0).
    pub offset: i64,
}

impl<'a> CodeQuery<'a> {
    #[must_use]
    pub fn new(query: &'a str) -> Self {
        Self {
            query,
            repository_id: None,
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

/// Result envelope for both file and code searches.
#[derive(Debug, Clone)]
pub struct CodeSearchResult<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

/// Search **files** by path substring and optional language.
///
/// Returns files whose `path` contains `q.query` (case-insensitive).
///
/// # Errors
/// Propagates `sqlx::Error` on database failure.
pub async fn search_files(
    pool: &PgPool,
    q: &CodeQuery<'_>,
) -> Result<CodeSearchResult<File>, sqlx::Error> {
    let effective_limit = q.limit.clamp(1, 200);
    let effective_offset = q.offset.max(0);
    let pattern = format!("%{}%", q.query);

    debug!(
        query = q.query,
        repo = ?q.repository_id,
        language = ?q.language,
        "file search"
    );

    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM files
         WHERE path ILIKE $1
         AND ($2::UUID IS NULL OR repository_id = $2)
         AND ($3::TEXT IS NULL OR language = $3)",
    )
    .bind(&pattern)
    .bind(q.repository_id)
    .bind(q.language)
    .fetch_one(pool)
    .await?;

    let items: Vec<File> = sqlx::query_as(
        "SELECT id, repository_id, path, language, size_bytes, content_hash, indexed_at
         FROM files
         WHERE path ILIKE $1
         AND ($2::UUID IS NULL OR repository_id = $2)
         AND ($3::TEXT IS NULL OR language = $3)
         ORDER BY path
         LIMIT $4 OFFSET $5",
    )
    .bind(&pattern)
    .bind(q.repository_id)
    .bind(q.language)
    .bind(effective_limit)
    .bind(effective_offset)
    .fetch_all(pool)
    .await?;

    Ok(CodeSearchResult {
        items,
        total,
        limit: effective_limit,
        offset: effective_offset,
    })
}

/// Search **code** by matching `raw_text` and `name` of symbols.
///
/// Uses `pg_trgm` `similarity()` when the query is a plain word, or `ILIKE`
/// when the query already contains `%` / `_` wildcards.  Results are ordered
/// by relevance (similarity score desc) or by path then line when using ILIKE.
///
/// # Errors
/// Propagates `sqlx::Error` on database failure.
pub async fn search_code(
    pool: &PgPool,
    q: &CodeQuery<'_>,
) -> Result<CodeSearchResult<Symbol>, sqlx::Error> {
    let effective_limit = q.limit.clamp(1, 200);
    let effective_offset = q.offset.max(0);
    let is_pattern = q.query.contains('%') || q.query.contains('_');

    debug!(
        query = q.query,
        repo = ?q.repository_id,
        language = ?q.language,
        "code search"
    );

    let total: i64 = if is_pattern {
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM symbols
             WHERE (raw_text ILIKE $1 OR name ILIKE $1)
             AND ($2::UUID IS NULL OR repository_id = $2)
             AND ($3::TEXT IS NULL OR language = $3)",
        )
        .bind(q.query)
        .bind(q.repository_id)
        .bind(q.language)
        .fetch_one(pool)
        .await?
    } else {
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM symbols
             WHERE (similarity(raw_text, $1) > 0.1 OR similarity(name, $1) > 0.1)
             AND ($2::UUID IS NULL OR repository_id = $2)
             AND ($3::TEXT IS NULL OR language = $3)",
        )
        .bind(q.query)
        .bind(q.repository_id)
        .bind(q.language)
        .fetch_one(pool)
        .await?
    };

    let items: Vec<Symbol> = if is_pattern {
        sqlx::query_as(
            "SELECT id, file_id, repository_id, name, symbol_type, language,
                    line_start, line_end, col_start, col_end,
                    visibility, doc_comment, raw_text, created_at
             FROM symbols
             WHERE (raw_text ILIKE $1 OR name ILIKE $1)
             AND ($2::UUID IS NULL OR repository_id = $2)
             AND ($3::TEXT IS NULL OR language = $3)
             ORDER BY file_id, line_start
             LIMIT $4 OFFSET $5",
        )
        .bind(q.query)
        .bind(q.repository_id)
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
             WHERE (similarity(raw_text, $1) > 0.1 OR similarity(name, $1) > 0.1)
             AND ($2::UUID IS NULL OR repository_id = $2)
             AND ($3::TEXT IS NULL OR language = $3)
             ORDER BY GREATEST(similarity(raw_text, $1), similarity(name, $1)) DESC,
                      file_id, line_start
             LIMIT $4 OFFSET $5",
        )
        .bind(q.query)
        .bind(q.repository_id)
        .bind(q.language)
        .bind(effective_limit)
        .bind(effective_offset)
        .fetch_all(pool)
        .await?
    };

    Ok(CodeSearchResult {
        items,
        total,
        limit: effective_limit,
        offset: effective_offset,
    })
}
