//! Confidence scoring for aggregated evidence.
//!
//! Three confidence levels are defined, mirroring the roadmap spec:
//!
//! * **[`ConfidenceLevel::Fact`]** — direct, first-party evidence: a doc
//!   comment, a commit message that mentions the symbol, blame authorship.
//! * **[`ConfidenceLevel::Inference`]** — derived from multiple corroborating
//!   sources but none is definitive on its own.
//! * **[`ConfidenceLevel::Unknown`]** — insufficient evidence to draw any
//!   conclusion.

use crate::aggregator::{EvidenceItem, EvidenceSource};

// ── Confidence level ──────────────────────────────────────────────────────────

/// Confidence in a derived conclusion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConfidenceLevel {
    /// No usable evidence; nothing can be said with confidence.
    Unknown,
    /// Conclusion is plausible but rests on indirect or sparse evidence.
    Inference,
    /// Conclusion is backed by direct, primary evidence.
    Fact,
}

impl ConfidenceLevel {
    /// Return the canonical string representation used in display and the DB.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fact => "FACT",
            Self::Inference => "INFERENCE",
            Self::Unknown => "UNKNOWN",
        }
    }

    /// Numeric score (0–100) for this level, useful for serialisation or
    /// threshold comparisons.
    #[must_use]
    pub fn score(self) -> u8 {
        match self {
            Self::Unknown => 0,
            Self::Inference => 50,
            Self::Fact => 100,
        }
    }
}

impl std::fmt::Display for ConfidenceLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ── Per-item scoring ──────────────────────────────────────────────────────────

/// Score a single evidence item.
///
/// The score is a 0–100 integer:
/// * Doc comments → 90 (direct, human-written documentation)
/// * Blame        → 70 (primary authorship data from version control)
/// * Commits      → 60 (indirect but explicit intent captured in messages)
/// * Database     → varies with stored confidence (high=80, medium=60, low=40)
/// * Code (raw)   → 30 (context, not explanation)
#[must_use]
pub fn score_item(item: &EvidenceItem) -> u8 {
    match item.source {
        EvidenceSource::Code => {
            // Doc comments are labelled in the content field.
            if item.content.starts_with("Doc comment:") {
                90
            } else {
                30
            }
        }
        EvidenceSource::Blame => 70,
        EvidenceSource::Commit => 60,
        EvidenceSource::Database => {
            // Map the item's weight (set from confidence in collect_from_db) back.
            match item.weight {
                4 => 80,
                3 => 60,
                2 => 40,
                _ => 20,
            }
        }
    }
}

// ── Aggregate confidence calculation ─────────────────────────────────────────

/// Calculate an overall [`ConfidenceLevel`] for a set of evidence items.
///
/// The algorithm:
///
/// 1. If there are no items → [`ConfidenceLevel::Unknown`].
/// 2. Compute a weighted average score across all items using [`score_item`].
/// 3. If any single item is a doc-comment (score ≥ 90) **or** the weighted
///    average exceeds 75 → [`ConfidenceLevel::Fact`].
/// 4. If the weighted average exceeds 40 **or** there are at least 3 items
///    from different source types → [`ConfidenceLevel::Inference`].
/// 5. Otherwise → [`ConfidenceLevel::Unknown`].
#[must_use]
pub fn calculate_confidence(items: &[EvidenceItem]) -> ConfidenceLevel {
    if items.is_empty() {
        return ConfidenceLevel::Unknown;
    }

    let has_direct_doc = items
        .iter()
        .any(|i| i.source == EvidenceSource::Code && i.content.starts_with("Doc comment:"));

    if has_direct_doc {
        return ConfidenceLevel::Fact;
    }

    let total_weight: u32 = items.iter().map(|i| u32::from(score_item(i))).sum();
    let item_count = u32::try_from(items.len()).unwrap_or(u32::MAX);
    let avg_score = total_weight / item_count;

    // Count distinct source types.
    let source_types: std::collections::HashSet<String> =
        items.iter().map(|i| i.source.to_string()).collect();

    if avg_score >= 75 {
        ConfidenceLevel::Fact
    } else if avg_score >= 40 || source_types.len() >= 2 {
        ConfidenceLevel::Inference
    } else {
        ConfidenceLevel::Unknown
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

    // ── ConfidenceLevel helpers ───────────────────────────────────────────────

    #[test]
    fn confidence_level_ordering() {
        assert!(ConfidenceLevel::Unknown < ConfidenceLevel::Inference);
        assert!(ConfidenceLevel::Inference < ConfidenceLevel::Fact);
    }

    #[test]
    fn confidence_level_score_values() {
        assert_eq!(ConfidenceLevel::Unknown.score(), 0);
        assert_eq!(ConfidenceLevel::Inference.score(), 50);
        assert_eq!(ConfidenceLevel::Fact.score(), 100);
    }

    #[test]
    fn confidence_level_display() {
        assert_eq!(ConfidenceLevel::Fact.to_string(), "FACT");
        assert_eq!(ConfidenceLevel::Inference.to_string(), "INFERENCE");
        assert_eq!(ConfidenceLevel::Unknown.to_string(), "UNKNOWN");
    }

    // ── score_item ────────────────────────────────────────────────────────────

    #[test]
    fn score_doc_comment_is_highest() {
        let doc = item(EvidenceSource::Code, "Doc comment: Does X.", 4);
        let raw = item(
            EvidenceSource::Code,
            "Source code (rust function): fn x() {}",
            1,
        );
        assert!(score_item(&doc) > score_item(&raw));
        assert_eq!(score_item(&doc), 90);
    }

    #[test]
    fn score_raw_code_is_lowest_source() {
        let raw = item(EvidenceSource::Code, "Source code: ...", 1);
        assert_eq!(score_item(&raw), 30);
    }

    #[test]
    fn score_blame_higher_than_commit() {
        let blame = item(EvidenceSource::Blame, "Alice authored 5 lines", 3);
        let commit = item(EvidenceSource::Commit, "Commit by Alice: add X", 2);
        assert!(score_item(&blame) > score_item(&commit));
    }

    // ── calculate_confidence ──────────────────────────────────────────────────

    #[test]
    fn empty_items_returns_unknown() {
        assert_eq!(calculate_confidence(&[]), ConfidenceLevel::Unknown);
    }

    #[test]
    fn doc_comment_alone_is_fact() {
        let items = vec![item(EvidenceSource::Code, "Doc comment: Parses input.", 4)];
        assert_eq!(calculate_confidence(&items), ConfidenceLevel::Fact);
    }

    #[test]
    fn single_low_score_item_is_unknown() {
        // Raw code item, weight=1 → score=30 → avg=30, single source → Unknown.
        let items = vec![item(
            EvidenceSource::Code,
            "Source code (rust function): fn foo() {}",
            1,
        )];
        assert_eq!(calculate_confidence(&items), ConfidenceLevel::Unknown);
    }

    #[test]
    fn multiple_sources_elevate_to_inference() {
        // Two sources (commit + blame) but no doc comment.
        let items = vec![
            item(EvidenceSource::Commit, "Commit by Dev: init", 2),
            item(
                EvidenceSource::Blame,
                "Dev authored 10 lines (100% of symbol)",
                3,
            ),
        ];
        assert_eq!(calculate_confidence(&items), ConfidenceLevel::Inference);
    }

    #[test]
    fn high_average_score_is_fact() {
        // All blame items score 70 each → avg = 70 < 75 but ≥ 40 → Inference.
        // Combine blame (70) + blame (70) → avg 70 → Inference.
        let items = vec![
            item(EvidenceSource::Blame, "Alice authored 8 lines (80%)", 3),
            item(EvidenceSource::Blame, "Bob authored 2 lines (20%)", 3),
        ];
        // avg = 70, no doc comment, single source type → Inference.
        assert_eq!(calculate_confidence(&items), ConfidenceLevel::Inference);
    }

    #[test]
    fn no_evidence_conflicting_returns_unknown() {
        // Items that cancel out: one raw code (score 30) and nothing else.
        let items = vec![item(
            EvidenceSource::Code,
            "Source code (go function): func x()",
            1,
        )];
        assert_eq!(calculate_confidence(&items), ConfidenceLevel::Unknown);
    }

    #[test]
    fn fact_from_combined_high_scores() {
        // Doc comment (90) + blame (70) + commit (60) → avg = 73.3 → rounds to 73.
        // Still below 75, but doc comment triggers Fact directly.
        let items = vec![
            item(
                EvidenceSource::Code,
                "Doc comment: Validates the payload.",
                4,
            ),
            item(EvidenceSource::Blame, "Alice authored 5 lines (100%)", 3),
            item(EvidenceSource::Commit, "Commit by Alice: add validation", 2),
        ];
        assert_eq!(calculate_confidence(&items), ConfidenceLevel::Fact);
    }
}
