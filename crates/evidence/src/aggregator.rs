//! Evidence aggregator — collects and deduplicates evidence for a symbol from
//! multiple sources (commits, code, blame) and returns a ranked list.

use archaeologist_core::models::{Commit, Evidence, Symbol};
use tracing::{debug, info};
use uuid::Uuid;

// ── Source-specific evidence items (decoupled from the DB model) ─────────────

/// A single unit of evidence produced by the aggregator before persistence.
#[derive(Debug, Clone)]
pub struct EvidenceItem {
    /// Which source this evidence came from.
    pub source: EvidenceSource,
    /// A human-readable summary of the evidence.
    pub content: String,
    /// Optional reference string (commit SHA, file path, …).
    pub source_ref: Option<String>,
    /// Weight used during ranking (higher = more important).
    pub weight: u32,
}

/// All possible evidence sources.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvidenceSource {
    /// Derived from a git commit message or diff.
    Commit,
    /// Derived from the symbol's raw source code (doc-comment, `raw_text`, …).
    Code,
    /// Derived from `git blame` information (authorship, change history).
    Blame,
    /// Derived from existing [`Evidence`] records already in the database.
    Database,
}

impl EvidenceSource {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Commit => "commit",
            Self::Code => "code",
            Self::Blame => "blame",
            Self::Database => "database",
        }
    }
}

impl std::fmt::Display for EvidenceSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ── Collection helpers ────────────────────────────────────────────────────────

/// Collect evidence from a list of commits related to the symbol.
///
/// Each commit message is turned into one evidence item. Merge commits
/// (more than one parent) are skipped because their messages are typically
/// uninformative.
#[must_use]
pub fn collect_from_commits(commits: &[Commit]) -> Vec<EvidenceItem> {
    commits
        .iter()
        .filter(|c| {
            // Skip merge commits — their messages rarely carry useful signal.
            c.parent_shas.len() <= 1
        })
        .map(|c| {
            let message = c.message.trim().to_string();
            let content = if let Some(author) = &c.author_name {
                format!("Commit by {author}: {message}")
            } else {
                format!("Commit: {message}")
            };

            EvidenceItem {
                source: EvidenceSource::Commit,
                content,
                source_ref: Some(c.sha.clone()),
                // Recent commits get a slightly higher weight; we approximate by
                // giving every commit the same baseline and let ranking sort them.
                weight: 2,
            }
        })
        .collect()
}

/// Collect evidence from the symbol's source-code representation.
///
/// This checks:
/// 1. The symbol's doc-comment (highest signal).
/// 2. The symbol's raw source text (lower signal, still useful context).
#[must_use]
pub fn collect_from_code(symbol: &Symbol) -> Vec<EvidenceItem> {
    let mut items = Vec::new();

    if let Some(doc) = &symbol.doc_comment {
        let doc = doc.trim();
        if !doc.is_empty() {
            items.push(EvidenceItem {
                source: EvidenceSource::Code,
                content: format!("Doc comment: {doc}"),
                source_ref: Some(symbol.id.to_string()),
                weight: 4, // Doc comments are high-quality intentional documentation.
            });
        }
    }

    let raw = symbol.raw_text.trim();
    if !raw.is_empty() {
        // We only store a short excerpt to avoid noisy, oversized items.
        let excerpt: String = raw.chars().take(200).collect();
        let excerpt = if raw.len() > 200 {
            format!("{excerpt}…")
        } else {
            excerpt
        };
        items.push(EvidenceItem {
            source: EvidenceSource::Code,
            content: format!(
                "Source code ({} {}): {excerpt}",
                symbol.language, symbol.symbol_type
            ),
            source_ref: Some(format!("{}:{}", symbol.file_id, symbol.line_start)),
            weight: 1,
        });
    }

    items
}

/// Collect evidence from blame information.
///
/// `blame_entries` is a list of `(author_name, commit_sha, line_count)` tuples
/// produced by walking blame hunks over the symbol's line range.
#[must_use]
pub fn collect_from_blame(blame_entries: &[(String, String, usize)]) -> Vec<EvidenceItem> {
    if blame_entries.is_empty() {
        return vec![];
    }

    // Aggregate total lines per author.
    let mut author_lines: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for (author, _, lines) in blame_entries {
        *author_lines.entry(author.as_str()).or_insert(0) += lines;
    }

    let total_lines: usize = author_lines.values().sum();

    author_lines
        .iter()
        .map(|(author, lines)| {
            let pct = (*lines * 100).checked_div(total_lines).unwrap_or_default();
            EvidenceItem {
                source: EvidenceSource::Blame,
                content: format!("{author} authored {lines} lines ({pct}% of symbol)"),
                source_ref: blame_entries
                    .iter()
                    .find(|(a, _, _)| a.as_str() == *author)
                    .map(|(_, sha, _)| sha.clone()),
                weight: 3,
            }
        })
        .collect()
}

/// Wrap existing [`Evidence`] DB records as [`EvidenceItem`]s so they can be
/// ranked alongside freshly collected items.
#[must_use]
pub fn collect_from_db(db_evidence: &[Evidence]) -> Vec<EvidenceItem> {
    db_evidence
        .iter()
        .map(|e| {
            // Map the stored confidence string back to a weight.
            let weight = match e.confidence.as_str() {
                "high" => 4,
                "medium" => 3,
                "low" => 2,
                _ => 1,
            };
            EvidenceItem {
                source: EvidenceSource::Database,
                content: e.content.clone(),
                source_ref: e.source_ref.clone(),
                weight,
            }
        })
        .collect()
}

// ── Deduplication & ranking ───────────────────────────────────────────────────

/// Deduplicate and rank evidence items.
///
/// Two items are considered duplicates when their `content` strings are
/// identical (case-insensitive, whitespace-normalised). The surviving item
/// is the one with the higher `weight`. Items are then sorted
/// highest-weight first.
#[must_use]
pub fn deduplicate_and_rank(mut items: Vec<EvidenceItem>) -> Vec<EvidenceItem> {
    // Stable sort so that items with equal weight preserve insertion order.
    items.sort_by_key(|a| std::cmp::Reverse(a.weight));

    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    items
        .into_iter()
        .filter(|item| {
            // Normalise: lowercase + collapse whitespace.
            let key: String = item
                .content
                .to_lowercase()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            seen.insert(key)
        })
        .collect()
}

// ── Top-level entry point ─────────────────────────────────────────────────────

/// Aggregate all available evidence for `symbol_id`.
///
/// Callers provide pre-fetched data; this function is intentionally pure so
/// it can be unit-tested without a database.
///
/// Returns items ordered by descending weight (most relevant first).
pub fn aggregate_evidence(
    symbol_id: Uuid,
    symbol: Option<&Symbol>,
    commits: &[Commit],
    blame_entries: &[(String, String, usize)],
    db_evidence: &[Evidence],
) -> Vec<EvidenceItem> {
    info!("Aggregating evidence for symbol {symbol_id}");

    let mut all: Vec<EvidenceItem> = Vec::new();

    // 1. Evidence from commits.
    let commit_items = collect_from_commits(commits);
    debug!(count = commit_items.len(), "commit evidence items");
    all.extend(commit_items);

    // 2. Evidence from source code.
    if let Some(sym) = symbol {
        let code_items = collect_from_code(sym);
        debug!(count = code_items.len(), "code evidence items");
        all.extend(code_items);
    }

    // 3. Evidence from blame.
    let blame_items = collect_from_blame(blame_entries);
    debug!(count = blame_items.len(), "blame evidence items");
    all.extend(blame_items);

    // 4. Existing DB evidence.
    let db_items = collect_from_db(db_evidence);
    debug!(count = db_items.len(), "database evidence items");
    all.extend(db_items);

    // 5. Deduplicate and rank.
    let ranked = deduplicate_and_rank(all);
    info!("Aggregated {} unique evidence items", ranked.len());
    ranked
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    fn make_commit(sha: &str, message: &str, author: Option<&str>, parents: Vec<String>) -> Commit {
        Commit {
            id: Uuid::new_v4(),
            repository_id: Uuid::new_v4(),
            sha: sha.to_string(),
            author_name: author.map(ToString::to_string),
            author_email: Some("a@example.com".to_string()),
            author_date: Utc::now(),
            committer_name: Some("committer".to_string()),
            committer_email: Some("c@example.com".to_string()),
            committer_date: Utc::now(),
            message: message.to_string(),
            parent_shas: parents,
            created_at: Utc::now(),
        }
    }

    fn make_symbol(doc: Option<&str>, raw: &str) -> Symbol {
        Symbol {
            id: Uuid::new_v4(),
            file_id: Uuid::new_v4(),
            repository_id: Uuid::new_v4(),
            name: "test_fn".to_string(),
            symbol_type: "function".to_string(),
            language: "rust".to_string(),
            line_start: 1,
            line_end: 10,
            col_start: 0,
            col_end: 0,
            visibility: Some("pub".to_string()),
            doc_comment: doc.map(ToString::to_string),
            raw_text: raw.to_string(),
            created_at: Utc::now(),
        }
    }

    // ── collect_from_commits ──────────────────────────────────────────────────

    #[test]
    fn collect_commits_filters_merges() {
        let merge = make_commit(
            "abc",
            "Merge branch X",
            Some("Alice"),
            vec!["parent1".to_string(), "parent2".to_string()],
        );
        let regular = make_commit("def", "Fix bug", Some("Bob"), vec!["parent0".to_string()]);
        let items = collect_from_commits(&[merge, regular]);
        assert_eq!(items.len(), 1);
        assert!(items[0].content.contains("Fix bug"));
    }

    #[test]
    fn collect_commits_empty_returns_empty() {
        let items = collect_from_commits(&[]);
        assert!(items.is_empty());
    }

    #[test]
    fn collect_commits_includes_author_when_present() {
        let c = make_commit("abc", "Initial commit", Some("Alice"), vec![]);
        let items = collect_from_commits(&[c]);
        assert!(items[0].content.contains("Alice"));
        assert_eq!(items[0].source_ref.as_deref(), Some("abc"));
    }

    #[test]
    fn collect_commits_handles_missing_author() {
        let c = make_commit("abc", "Initial commit", None, vec![]);
        let items = collect_from_commits(&[c]);
        assert!(items[0].content.starts_with("Commit:"));
    }

    // ── collect_from_code ─────────────────────────────────────────────────────

    #[test]
    fn collect_code_with_doc_comment() {
        let sym = make_symbol(Some("Computes the total."), "fn total() {}");
        let items = collect_from_code(&sym);
        assert!(items
            .iter()
            .any(|i| i.content.contains("Computes the total.")));
        let doc_item = items.iter().find(|i| i.content.starts_with("Doc")).unwrap();
        assert_eq!(doc_item.weight, 4);
    }

    #[test]
    fn collect_code_no_doc_comment_still_returns_raw_text() {
        let sym = make_symbol(None, "fn total() {}");
        let items = collect_from_code(&sym);
        assert_eq!(items.len(), 1);
        assert!(items[0].content.contains("fn total()"));
    }

    #[test]
    fn collect_code_empty_raw_text_and_no_doc_returns_empty() {
        let sym = make_symbol(None, "");
        let items = collect_from_code(&sym);
        assert!(items.is_empty());
    }

    #[test]
    fn collect_code_truncates_long_raw_text() {
        let long_raw = "x".repeat(300);
        let sym = make_symbol(None, &long_raw);
        let items = collect_from_code(&sym);
        assert_eq!(items.len(), 1);
        // Content should contain the ellipsis marker.
        assert!(items[0].content.contains('…'));
    }

    // ── collect_from_blame ────────────────────────────────────────────────────

    #[test]
    fn collect_blame_empty_returns_empty() {
        let items = collect_from_blame(&[]);
        assert!(items.is_empty());
    }

    #[test]
    fn collect_blame_computes_percentage() {
        let entries = vec![
            ("Alice".to_string(), "abc".to_string(), 8),
            ("Bob".to_string(), "def".to_string(), 2),
        ];
        let items = collect_from_blame(&entries);
        let alice = items.iter().find(|i| i.content.contains("Alice")).unwrap();
        assert!(alice.content.contains("80%"));
        let bob = items.iter().find(|i| i.content.contains("Bob")).unwrap();
        assert!(bob.content.contains("20%"));
    }

    // ── deduplicate_and_rank ──────────────────────────────────────────────────

    #[test]
    fn dedup_removes_exact_duplicates() {
        let items = vec![
            EvidenceItem {
                source: EvidenceSource::Code,
                content: "Same content".to_string(),
                source_ref: None,
                weight: 2,
            },
            EvidenceItem {
                source: EvidenceSource::Commit,
                content: "Same content".to_string(),
                source_ref: None,
                weight: 3,
            },
        ];
        let ranked = deduplicate_and_rank(items);
        assert_eq!(ranked.len(), 1);
        // The higher-weight item (weight=3) should survive.
        assert_eq!(ranked[0].weight, 3);
    }

    #[test]
    fn dedup_removes_case_insensitive_duplicates() {
        let items = vec![
            EvidenceItem {
                source: EvidenceSource::Code,
                content: "Hello World".to_string(),
                source_ref: None,
                weight: 1,
            },
            EvidenceItem {
                source: EvidenceSource::Blame,
                content: "hello world".to_string(),
                source_ref: None,
                weight: 2,
            },
        ];
        let ranked = deduplicate_and_rank(items);
        assert_eq!(ranked.len(), 1);
    }

    #[test]
    fn dedup_preserves_distinct_items() {
        let items = vec![
            EvidenceItem {
                source: EvidenceSource::Code,
                content: "Alpha".to_string(),
                source_ref: None,
                weight: 1,
            },
            EvidenceItem {
                source: EvidenceSource::Commit,
                content: "Beta".to_string(),
                source_ref: None,
                weight: 2,
            },
        ];
        let ranked = deduplicate_and_rank(items);
        assert_eq!(ranked.len(), 2);
        // Higher weight first.
        assert_eq!(ranked[0].content, "Beta");
    }

    // ── aggregate_evidence ────────────────────────────────────────────────────

    #[test]
    fn aggregate_no_evidence_returns_empty() {
        let items = aggregate_evidence(Uuid::new_v4(), None, &[], &[], &[]);
        assert!(items.is_empty());
    }

    #[test]
    fn aggregate_combines_all_sources() {
        let sym = make_symbol(Some("Does something useful."), "fn do_thing() {}");
        let commit = make_commit("sha1", "Add do_thing", Some("Dev"), vec![]);
        let blame = vec![("Dev".to_string(), "sha1".to_string(), 5)];

        let items = aggregate_evidence(sym.id, Some(&sym), &[commit], &blame, &[]);
        // Expect at least: doc comment, raw text, commit, blame
        assert!(items.len() >= 3);
    }

    #[test]
    fn aggregate_type_mapping_commit_source() {
        let commit = make_commit("sha1", "Initial commit", Some("Dev"), vec![]);
        let items = aggregate_evidence(Uuid::new_v4(), None, &[commit], &[], &[]);
        assert!(items.iter().any(|i| i.source == EvidenceSource::Commit));
    }
}
