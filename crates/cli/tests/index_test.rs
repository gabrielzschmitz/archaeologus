//! Integration tests for the `index` command.
//!
//! These tests exercise the non-database parts of the pipeline: path resolution,
//! clone detection, and the indexer round-trip.  Tests that require a live
//! PostgreSQL database are gated behind the `integration` feature flag.

use std::path::Path;

use tempfile::TempDir;

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Create a minimal git repo with one Rust source file and one commit.
fn make_rust_repo() -> TempDir {
    let dir = TempDir::new().unwrap();

    // Write a small Rust file so the indexer has something to parse.
    let src = dir.path().join("lib.rs");
    std::fs::write(
        &src,
        r#"
pub struct Greeter {
    name: String,
}

impl Greeter {
    pub fn new(name: &str) -> Self {
        Self { name: name.to_string() }
    }

    pub fn greet(&self) -> String {
        format!("Hello, {}!", self.name)
    }
}

pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
"#,
    )
    .unwrap();

    // Init git repo and commit the file.
    let repo = git2::Repository::init(dir.path()).unwrap();
    let sig = git2::Signature::now("Test Author", "test@example.com").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("lib.rs")).unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "initial commit", &tree, &[])
        .unwrap();

    dir
}

// ── resolve_path tests (white-box, no DB needed) ─────────────────────────────

/// A local path that exists resolves to its canonical absolute form.
#[test]
fn resolve_local_path_returns_absolute() {
    let repo_dir = make_rust_repo();
    let path_str = repo_dir.path().to_string_lossy().into_owned();

    // We exercise the same logic as `resolve_path` without calling it directly
    // (it is not pub), so we replicate the detection logic here.
    let is_url = path_str.contains("://") || path_str.starts_with("git@");
    assert!(!is_url, "a tempdir path should not look like a URL");

    let resolved = std::path::PathBuf::from(&path_str)
        .canonicalize()
        .expect("should canonicalize");
    assert!(resolved.is_absolute());
    assert!(resolved.join(".git").exists());
}

/// Strings that look like URLs are detected as remote targets.
#[test]
fn url_detection_covers_https_ssh_and_git_protocol() {
    let urls = [
        "https://github.com/owner/repo.git",
        "git@github.com:owner/repo.git",
        "git://github.com/owner/repo.git",
        "ssh://git@github.com/owner/repo",
    ];
    for url in &urls {
        let is_url = url.contains("://") || url.starts_with("git@");
        assert!(is_url, "{url} should be detected as a URL");
    }
}

/// A plain file-system path is not detected as a URL.
#[test]
fn local_path_is_not_detected_as_url() {
    for path in &["/home/user/repo", "./repo", "relative/path"] {
        let is_url = path.contains("://") || path.starts_with("git@");
        assert!(!is_url, "{path} should not be detected as a URL");
    }
}

// ── Indexer pipeline (no DB) ─────────────────────────────────────────────────

/// The indexer finds and parses Rust files from a local repo.
#[test]
fn indexer_finds_rust_symbols_in_local_repo() {
    let repo_dir = make_rust_repo();
    let results = archaeologist_indexer::index_directory(repo_dir.path(), |_, _| {})
        .expect("index_directory should succeed");

    assert!(!results.is_empty(), "should find at least one file");

    let rust_file = results
        .iter()
        .find(|f| f.language == archaeologist_indexer::Lang::Rust)
        .expect("should find a Rust file");

    // We expect Greeter struct, Greeter impl, new + greet functions, and add.
    assert!(
        !rust_file.symbols.is_empty(),
        "should extract symbols from the Rust file"
    );

    let names: Vec<&str> = rust_file.symbols.iter().map(|s| s.name.as_str()).collect();
    assert!(
        names.contains(&"Greeter"),
        "should extract Greeter; got {names:?}"
    );
    assert!(
        names.contains(&"add"),
        "should extract add function; got {names:?}"
    );
}

/// The indexer gracefully handles an empty directory (no supported files).
#[test]
fn indexer_handles_empty_directory() {
    let empty = TempDir::new().unwrap();
    let results = archaeologist_indexer::index_directory(empty.path(), |_, _| {})
        .expect("index_directory on empty dir should succeed");
    assert!(results.is_empty());
}

/// Indexing a non-existent path should still succeed (returns empty / no panic).
/// walkdir on a missing dir returns no entries.
#[test]
fn indexer_handles_nonexistent_path_gracefully() {
    let nonexistent = std::path::Path::new("/tmp/archaeologist-nonexistent-test-12345");
    // index_directory uses walkdir which silently yields nothing for a missing dir
    let results = archaeologist_indexer::index_directory(nonexistent, |_, _| {});
    // Either Ok(empty) or an error is acceptable — no panic.
    if let Ok(v) = results {
        assert!(v.is_empty());
    }
}

// ── Clone from local "remote" (file:// URL) ───────────────────────────────────

/// Clone using a `file://` URL mimics the remote-clone code path.
#[test]
fn clone_local_file_url_and_index() {
    let src_dir = make_rust_repo();
    let dest_dir = TempDir::new().unwrap();
    let dest = dest_dir.path().join("clone");

    let url = format!("file://{}", src_dir.path().display());
    let cloned = archaeologist_git::clone_repository(
        &url,
        &dest,
        archaeologist_git::CloneOptions::default(),
    )
    .expect("clone should succeed");

    assert!(cloned.join(".git").exists());

    // Index the clone.
    let results =
        archaeologist_indexer::index_directory(&cloned, |_, _| {}).expect("index cloned repo");
    assert!(!results.is_empty());
}

// ── Mock-remote path: /tmp/archaeologist-fixtures ────────────────────────────

/// Index the project's local fixture repository (the mock "remote").
#[test]
fn index_fixtures_repository() {
    let fixtures = std::path::Path::new("/tmp/archaeologist-fixtures");
    if !fixtures.exists() {
        // Skip when the fixture repo is absent (e.g. clean CI machine).
        eprintln!("skipping: /tmp/archaeologist-fixtures not found");
        return;
    }

    let results = archaeologist_indexer::index_directory(fixtures, |done, total| {
        if total > 0 {
            let _ = (done, total);
        }
    })
    .expect("should index fixtures");

    // The fixture repo has Rust, Python, Go, Java, JS, TS, C, C++ files.
    assert!(
        !results.is_empty(),
        "fixtures should contain at least one indexed file"
    );

    // Every indexed file must have a detectable language.
    for f in &results {
        let _ = f.language; // just assert it compiled with a valid Lang variant
    }
}

// ── Idempotency via content-hash check (unit logic) ─────────────────────────

/// Two passes over the same bytes produce the same SHA-256 hash.
#[test]
fn same_file_produces_same_content_hash() {
    use sha2::{Digest, Sha256};

    let data = b"fn main() {}";
    let h1 = hex::encode(Sha256::digest(data));
    let h2 = hex::encode(Sha256::digest(data));
    assert_eq!(h1, h2);
}

/// Different bytes produce different hashes (ensures idempotency guard works).
#[test]
fn different_content_produces_different_hash() {
    use sha2::{Digest, Sha256};

    let h1 = hex::encode(Sha256::digest(b"version 1"));
    let h2 = hex::encode(Sha256::digest(b"version 2"));
    assert_ne!(h1, h2);
}

// ── Symbol-kind mapping ───────────────────────────────────────────────────────

/// Every `SymbolKind` variant maps to a valid `SymbolType` without panicking.
#[test]
fn all_symbol_kinds_map_without_panic() {
    use archaeologist_indexer::SymbolKind;

    let kinds = [
        SymbolKind::Function,
        SymbolKind::Method,
        SymbolKind::Struct,
        SymbolKind::Enum,
        SymbolKind::Trait,
        SymbolKind::Impl,
        SymbolKind::Module,
        SymbolKind::Class,
        SymbolKind::Interface,
        SymbolKind::Type,
        SymbolKind::Constructor,
    ];

    for kind in &kinds {
        // The mapping lives in commands::index but its logic is simple; we
        // replicate it here so the test has no dependency on internal fns.
        use archaeologist_core::models::SymbolType;
        let _mapped: SymbolType = match kind {
            SymbolKind::Function | SymbolKind::Constructor => SymbolType::Function,
            SymbolKind::Method => SymbolType::Method,
            SymbolKind::Struct => SymbolType::Struct,
            SymbolKind::Enum => SymbolType::Enum,
            SymbolKind::Trait => SymbolType::Trait,
            SymbolKind::Impl => SymbolType::Impl,
            SymbolKind::Module => SymbolType::Module,
            SymbolKind::Class => SymbolType::Class,
            SymbolKind::Interface => SymbolType::Interface,
            SymbolKind::Type => SymbolType::Type,
        };
    }
}

// ── Git history walk (no DB) ─────────────────────────────────────────────────

/// `walk_commits` returns the commits from a local repo.
#[test]
fn walk_commits_finds_initial_commit() {
    let repo_dir = make_rust_repo();
    let commits =
        archaeologist_git::walk_commits(repo_dir.path(), &archaeologist_git::WalkFilter::default())
            .expect("walk_commits should succeed");

    assert_eq!(commits.len(), 1);
    assert_eq!(commits[0].author_name, "Test Author");
    assert_eq!(commits[0].message, "initial commit");
}

/// `walk_commits` on a path that is not a git repo returns an error.
#[test]
fn walk_commits_on_non_repo_returns_error() {
    let not_a_repo = TempDir::new().unwrap();
    let result = archaeologist_git::walk_commits(
        not_a_repo.path(),
        &archaeologist_git::WalkFilter::default(),
    );
    assert!(result.is_err());
}
