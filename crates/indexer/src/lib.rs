//! `archaeologist-indexer` — parallel source-code indexing crate.
//!
//! # Quick start
//! ```no_run
//! use std::path::Path;
//! use archaeologist_indexer::index_directory;
//!
//! let results = index_directory(Path::new("src"), |done, total| {
//!     eprintln!("{done}/{total}");
//! }).unwrap();
//! ```

pub mod dependencies;
pub mod languages;
pub mod parser;
pub mod symbols;

pub use dependencies::{
    build_graph, extract_dependencies, Dependency, DependencyGraph, DependencyKind,
};
pub use languages::{detect_language, Lang};
pub use parser::{parse, parse_incremental, ParseResult};
pub use symbols::{extract_symbols, ExtractedSymbol, SymbolKind};

use rayon::prelude::*;
use std::path::Path;
use tracing::{debug, warn};
use walkdir::WalkDir;

#[derive(Debug)]
pub struct IndexedFile {
    pub path: std::path::PathBuf,
    pub language: Lang,
    pub symbols: Vec<ExtractedSymbol>,
    pub dependencies: Vec<Dependency>,
    pub has_errors: bool,
}

/// Index every source file under `dir` in parallel using rayon.
///
/// `on_progress` is called after each file is processed with `(done, total)`.
/// The callback runs on rayon worker threads; keep it cheap (e.g. just a
/// channel send or an atomic increment).
///
/// Files whose extension is not one of the 8 supported languages are silently
/// skipped (no error).
///
/// # Errors
/// Returns an error only if `dir` cannot be walked at all.
pub fn index_directory<F>(dir: &Path, on_progress: F) -> anyhow::Result<Vec<IndexedFile>>
where
    F: Fn(usize, usize) + Sync,
{
    let entries: Vec<std::path::PathBuf> = WalkDir::new(dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .map(walkdir::DirEntry::into_path)
        .filter(|p| p.to_str().and_then(detect_language).is_some())
        .collect();

    let total = entries.len();
    let done = std::sync::atomic::AtomicUsize::new(0);

    let results: Vec<IndexedFile> = entries
        .par_iter()
        .filter_map(|path| {
            let result = index_file(path);
            let n = done.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
            on_progress(n, total);
            result
        })
        .collect();

    Ok(results)
}

fn index_file(path: &Path) -> Option<IndexedFile> {
    let lang = detect_language(path.to_str()?)?;
    let source = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            warn!("Failed to read {path:?}: {e}");
            return None;
        }
    };

    let parse_result = match parse(&source, lang) {
        Ok(r) => r,
        Err(e) => {
            warn!("Failed to parse {path:?}: {e}");
            return None;
        }
    };

    let has_errors = parse_result.has_errors;
    if has_errors {
        debug!("Parse errors in {path:?}");
    }

    let syms = extract_symbols(&parse_result, &source);
    let deps = extract_dependencies(&parse_result, &source);

    Some(IndexedFile {
        path: path.to_path_buf(),
        language: lang,
        symbols: syms,
        dependencies: deps,
        has_errors,
    })
}
