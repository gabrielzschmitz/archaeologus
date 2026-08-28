#![allow(clippy::missing_errors_doc)]

use std::path::Path;

/// Detect programming language from file path using case-insensitive extension matching.
#[must_use]
pub fn detect_language(path: &str) -> Option<String> {
    let ext = Path::new(path).extension()?.to_str()?.to_ascii_lowercase();
    match ext.as_str() {
        "rs" => Some("rust".into()),
        "py" => Some("python".into()),
        "js" | "mjs" | "cjs" => Some("javascript".into()),
        "ts" | "mts" | "cts" | "tsx" => Some("typescript".into()),
        "go" => Some("go".into()),
        "java" => Some("java".into()),
        "c" | "h" => Some("c".into()),
        "cpp" | "hpp" | "cc" | "cxx" | "hxx" | "C" => Some("cpp".into()),
        _ => None,
    }
}
