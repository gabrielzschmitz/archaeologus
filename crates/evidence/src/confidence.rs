#![allow(clippy::missing_errors_doc)]

#[must_use]
pub fn calculate_confidence(evidence_count: usize) -> String {
    match evidence_count {
        0 => "unknown".to_string(),
        1..=2 => "low".to_string(),
        3..=5 => "medium".to_string(),
        _ => "high".to_string(),
    }
}
