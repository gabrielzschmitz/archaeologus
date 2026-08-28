//! `archaeologist-evidence` — evidence aggregation, confidence scoring, and
//! human-readable explanation generation.
//!
//! # Quick start
//!
//! ```rust,no_run
//! use archaeologist_evidence::aggregator::aggregate_evidence;
//! use archaeologist_evidence::explainer::explain_symbol;
//! use uuid::Uuid;
//!
//! let items = aggregate_evidence(Uuid::new_v4(), None, &[], &[], &[]);
//! let explanation = explain_symbol("my_symbol", &items);
//! println!("{}", explanation.to_display_string());
//! ```

pub mod aggregator;
pub mod confidence;
pub mod explainer;

// ── Aggregator re-exports ──────────────────────────────────────────────────────
pub use aggregator::{
    aggregate_evidence, collect_from_blame, collect_from_code, collect_from_commits,
    collect_from_db, deduplicate_and_rank, EvidenceItem, EvidenceSource,
};

// ── Confidence re-exports ──────────────────────────────────────────────────────
pub use confidence::{calculate_confidence, score_item, ConfidenceLevel};

// ── Explainer re-exports ───────────────────────────────────────────────────────
pub use explainer::{explain_symbol, EvidenceCitation, Explanation};
