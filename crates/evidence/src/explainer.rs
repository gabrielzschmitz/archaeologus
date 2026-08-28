//! Human-readable explanation generation.
//!
//! Given a list of [`EvidenceItem`]s and a computed [`ConfidenceLevel`], the
//! explainer formats a structured, printable [`Explanation`] and also provides
//! a plain-text summary for CLI output.

use crate::aggregator::{EvidenceItem, EvidenceSource};
use crate::confidence::ConfidenceLevel;
use std::fmt::Write;

// ── Output types ──────────────────────────────────────────────────────────────

/// A single cited source included in an [`Explanation`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceCitation {
    /// The kind of source (e.g. `"commit"`, `"blame"`, `"code"`).
    pub source_type: String,
    /// The human-readable evidence text.
    pub content: String,
    /// Optional reference (SHA, file path, …).
    pub source_ref: Option<String>,
    /// Individual item score (0–100).
    pub score: u8,
}

/// A fully structured explanation of a symbol.
#[derive(Debug, Clone)]
pub struct Explanation {
    /// The symbol being explained (name or ID).
    pub subject: String,
    /// A concise, one-paragraph prose summary.
    pub summary: String,
    /// The computed confidence level.
    pub confidence: ConfidenceLevel,
    /// Ordered list of evidence citations (highest score first).
    pub citations: Vec<EvidenceCitation>,
}

impl Explanation {
    /// Render the explanation as a multi-line human-readable string suitable
    /// for terminal output.
    #[must_use]
    pub fn to_display_string(&self) -> String {
        let mut out = String::new();

        writeln!(out, "Subject  : {}", self.subject).expect("writing to String cannot fail");
        writeln!(
            out,
            "Confidence: {} ({})",
            self.confidence,
            self.confidence.score()
        )
        .expect("writing to String cannot fail");
        out.push('\n');
        writeln!(out, "Summary\n-------\n{}", self.summary).expect("writing to String cannot fail");

        if self.citations.is_empty() {
            out.push_str("\nNo evidence sources available.\n");
        } else {
            out.push('\n');
            out.push_str("Evidence Sources\n----------------\n");
            for (i, c) in self.citations.iter().enumerate() {
                let ref_str = c
                    .source_ref
                    .as_deref()
                    .map(|r| format!(" [{r}]"))
                    .unwrap_or_default();
                writeln!(
                    out,
                    "  {}. [{}]{ref_str} (score {}) — {}",
                    i + 1,
                    c.source_type.to_uppercase(),
                    c.score,
                    c.content,
                )
                .expect("writing to String cannot fail");
            }
        }

        out
    }
}

// ── Summary generation ────────────────────────────────────────────────────────

/// Build a concise prose summary from the top evidence items.
///
/// The strategy:
/// 1. Use the first doc-comment item verbatim if present.
/// 2. Otherwise, weave together the top-scoring commit message(s) and blame
///    authorship into a short paragraph.
/// 3. Fall back to a generic "insufficient evidence" phrase.
fn build_summary(subject: &str, items: &[EvidenceItem], confidence: ConfidenceLevel) -> String {
    // Try doc comment first (highest signal).
    if let Some(doc) = items
        .iter()
        .find(|i| i.source == EvidenceSource::Code && i.content.starts_with("Doc comment:"))
    {
        let text = doc.content.trim_start_matches("Doc comment:").trim();
        return format!("`{subject}` — {text}");
    }

    // Gather commit messages (up to 2).
    let commit_msgs: Vec<&str> = items
        .iter()
        .filter(|i| i.source == EvidenceSource::Commit)
        .take(2)
        .map(|i| i.content.as_str())
        .collect();

    // Find the primary blame author (first blame item after ranking).
    let blame_author: Option<&str> = items
        .iter()
        .find(|i| i.source == EvidenceSource::Blame)
        .map(|i| i.content.as_str());

    match (commit_msgs.is_empty(), blame_author) {
        (false, Some(blame)) => {
            let msgs = commit_msgs.join("; ");
            format!("`{subject}` was introduced via: {msgs}. Primary author: {blame}.")
        }
        (false, None) => {
            let msgs = commit_msgs.join("; ");
            format!("`{subject}` was introduced via: {msgs}.")
        }
        (true, Some(blame)) => {
            format!("`{subject}` — {blame}.")
        }
        (true, None) => match confidence {
            ConfidenceLevel::Unknown => {
                format!("`{subject}` — insufficient evidence to determine purpose or origin.")
            }
            _ => format!("`{subject}` — purpose inferred from available evidence (see citations)."),
        },
    }
}

// ── Top-level entry point ─────────────────────────────────────────────────────

/// Generate an [`Explanation`] for `subject` from ranked evidence items.
///
/// `items` should already be ranked (highest weight first) — pass the output
/// of [`crate::aggregator::deduplicate_and_rank`] directly.
#[must_use]
pub fn explain_symbol(subject: &str, items: &[EvidenceItem]) -> Explanation {
    use crate::confidence::{calculate_confidence, score_item};

    let confidence = calculate_confidence(items);
    let summary = build_summary(subject, items, confidence);

    let mut citations: Vec<EvidenceCitation> = items
        .iter()
        .map(|item| EvidenceCitation {
            source_type: item.source.to_string(),
            content: item.content.clone(),
            source_ref: item.source_ref.clone(),
            score: score_item(item),
        })
        .collect();

    // Sort citations by score descending.
    citations.sort_by_key(|c| std::cmp::Reverse(c.score));

    Explanation {
        subject: subject.to_string(),
        summary,
        confidence,
        citations,
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aggregator::{EvidenceItem, EvidenceSource};

    fn item(source: EvidenceSource, content: &str, weight: u32) -> EvidenceItem {
        EvidenceItem {
            source,
            content: content.to_string(),
            source_ref: None,
            weight,
        }
    }

    fn item_with_ref(source: EvidenceSource, content: &str, weight: u32, r: &str) -> EvidenceItem {
        EvidenceItem {
            source,
            content: content.to_string(),
            source_ref: Some(r.to_string()),
            weight,
        }
    }

    // ── explain_symbol: confidence propagation ────────────────────────────────

    #[test]
    fn no_evidence_returns_unknown_confidence() {
        let exp = explain_symbol("my_fn", &[]);
        assert_eq!(exp.confidence, ConfidenceLevel::Unknown);
    }

    #[test]
    fn doc_comment_yields_fact_confidence() {
        let items = vec![item(
            EvidenceSource::Code,
            "Doc comment: Validates request headers.",
            4,
        )];
        let exp = explain_symbol("validate_headers", &items);
        assert_eq!(exp.confidence, ConfidenceLevel::Fact);
    }

    #[test]
    fn multiple_sources_yield_inference() {
        let items = vec![
            item(EvidenceSource::Commit, "Commit by Dev: initial impl", 2),
            item(EvidenceSource::Blame, "Dev authored 10 lines (100%)", 3),
        ];
        let exp = explain_symbol("process", &items);
        assert_eq!(exp.confidence, ConfidenceLevel::Inference);
    }

    // ── explain_symbol: summary content ──────────────────────────────────────

    #[test]
    fn summary_prefers_doc_comment() {
        let items = vec![
            item(
                EvidenceSource::Code,
                "Doc comment: Parses the incoming request.",
                4,
            ),
            item(
                EvidenceSource::Commit,
                "Commit by Dev: add parse_request",
                2,
            ),
        ];
        let exp = explain_symbol("parse_request", &items);
        assert!(exp.summary.contains("Parses the incoming request."));
    }

    #[test]
    fn summary_falls_back_to_commit_message() {
        let items = vec![item(
            EvidenceSource::Commit,
            "Commit by Alice: introduce hash_password",
            2,
        )];
        let exp = explain_symbol("hash_password", &items);
        assert!(exp.summary.contains("introduce hash_password"));
    }

    #[test]
    fn summary_includes_blame_when_no_commit() {
        let items = vec![item(
            EvidenceSource::Blame,
            "Alice authored 5 lines (100% of symbol)",
            3,
        )];
        let exp = explain_symbol("do_work", &items);
        assert!(exp.summary.contains("Alice"));
    }

    #[test]
    fn summary_for_empty_evidence_mentions_insufficient() {
        let exp = explain_symbol("mystery_fn", &[]);
        assert!(exp.summary.contains("insufficient evidence"));
    }

    // ── explain_symbol: citations ─────────────────────────────────────────────

    #[test]
    fn citations_sorted_by_score_descending() {
        let items = vec![
            item(EvidenceSource::Code, "Source code (rust fn): fn x(){}", 1),
            item(EvidenceSource::Code, "Doc comment: Does Y.", 4),
            item(EvidenceSource::Commit, "Commit by Dev: add x", 2),
        ];
        let exp = explain_symbol("x", &items);
        assert!(!exp.citations.is_empty());
        let scores: Vec<u8> = exp.citations.iter().map(|c| c.score).collect();
        for pair in scores.windows(2) {
            assert!(pair[0] >= pair[1], "citations not sorted: {scores:?}");
        }
    }

    #[test]
    fn citations_include_source_ref_when_present() {
        let items = vec![item_with_ref(
            EvidenceSource::Commit,
            "Commit by Dev: add feature",
            2,
            "deadbeef",
        )];
        let exp = explain_symbol("feature", &items);
        assert_eq!(exp.citations[0].source_ref.as_deref(), Some("deadbeef"));
    }

    #[test]
    fn no_evidence_citations_are_empty() {
        let exp = explain_symbol("ghost_fn", &[]);
        assert!(exp.citations.is_empty());
    }

    // ── to_display_string ─────────────────────────────────────────────────────

    #[test]
    fn display_string_contains_subject() {
        let items = vec![item(EvidenceSource::Code, "Doc comment: Does Z.", 4)];
        let exp = explain_symbol("do_z", &items);
        let text = exp.to_display_string();
        assert!(text.contains("do_z"));
    }

    #[test]
    fn display_string_shows_confidence() {
        let items = vec![item(EvidenceSource::Code, "Doc comment: Does Z.", 4)];
        let exp = explain_symbol("do_z", &items);
        let text = exp.to_display_string();
        assert!(text.contains("FACT"));
    }

    #[test]
    fn display_string_no_evidence_message() {
        let exp = explain_symbol("hidden_fn", &[]);
        let text = exp.to_display_string();
        assert!(text.contains("No evidence sources available."));
    }

    #[test]
    fn display_string_lists_numbered_citations() {
        let items = vec![
            item(EvidenceSource::Code, "Doc comment: Entry point.", 4),
            item_with_ref(
                EvidenceSource::Commit,
                "Commit by Dev: initial commit",
                2,
                "abc123",
            ),
        ];
        let exp = explain_symbol("main", &items);
        let text = exp.to_display_string();
        assert!(text.contains("1."));
        assert!(text.contains("2."));
        assert!(text.contains("abc123"));
    }

    // ── edge cases ────────────────────────────────────────────────────────────

    #[test]
    fn explain_symbol_with_only_raw_code() {
        let items = vec![item(
            EvidenceSource::Code,
            "Source code (python function): def foo(): pass",
            1,
        )];
        let exp = explain_symbol("foo", &items);
        // A raw code item alone is not enough to be FACT or INFERENCE.
        assert_eq!(exp.confidence, ConfidenceLevel::Unknown);
        assert_eq!(exp.citations.len(), 1);
    }
}
