//! `index` command — clone (if remote), parse, and persist a repository.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use anyhow::{Context, Result};
use archaeologist_core::models::{CommitCreate, FileCreate, RepositoryCreate, SymbolCreate};
use archaeologist_db::PgPool;
use archaeologist_db::{
    create_pool,
    repositories::{
        create_commit, create_file, create_repository, create_symbol, get_commit_by_sha,
        get_file_by_path, get_repository_by_url, update_repository_indexed,
    },
    run_migrations,
};
use archaeologist_git::{clone_repository, walk_commits, CloneOptions, WalkFilter};
use archaeologist_indexer::{index_directory, languages::Lang, IndexedFile};
use sha2::{Digest, Sha256};
use tracing::{info, warn};
use uuid::Uuid;

/// Options for the `index` sub-command.
#[derive(Debug)]
pub struct IndexOptions {
    /// A git URL (https/ssh/git) or a local file-system path.
    pub target: String,
    /// Branch hint stored on the repository record (does **not** affect clone).
    pub branch: Option<String>,
    pub database_url: String,
    pub rust_log: String,
}

/// Entry point wired from `main.rs`.
///
/// # Errors
/// Propagates database, git, and I/O errors.
pub async fn run(opts: IndexOptions) -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(&opts.rust_log)
        .try_init()
        .ok();

    let pool = create_pool(&opts.database_url)
        .await
        .context("connect to database")?;
    run_migrations(&pool).await.context("run migrations")?;

    // ── 1. Resolve local path (clone if the target looks like a URL) ─────────
    let (local_path, canonical_url) = resolve_path(&opts.target)?;

    info!(
        "Indexing '{}' from '{}'",
        canonical_url,
        local_path.display()
    );

    // ── 2. Upsert the repository record ──────────────────────────────────────
    let repo_record =
        upsert_repository(&pool, &canonical_url, &local_path, opts.branch.as_ref()).await?;
    let repo_id = repo_record.id;

    // ── 3. Index source files ─────────────────────────────────────────────────
    let indexed = index_directory(&local_path, report_progress()).context("index directory")?;
    info!("Parsed {} source files", indexed.len());

    // ── 4. Persist files, symbols, and commits ───────────────────────────────
    let (files_stored, symbols_stored) = store_files(&pool, repo_id, &local_path, &indexed).await;
    let commits_stored = store_commits(&pool, repo_id, &local_path).await;

    // ── 5. Mark repository as indexed ────────────────────────────────────────
    update_repository_indexed(&pool, repo_id)
        .await
        .context("update indexed_at")?;

    println!("✓ Indexed '{canonical_url}': {files_stored} files, {symbols_stored} symbols, {commits_stored} commits");

    Ok(())
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Build the progress callback passed to [`index_directory`].
fn report_progress() -> impl Fn(usize, usize) + Sync {
    let done_files = Arc::new(AtomicUsize::new(0));
    let done_cb = done_files.clone();
    move |done, total| {
        done_cb.store(done, Ordering::Relaxed);
        if total > 0 && (done == 1 || done % 50 == 0 || done == total) {
            eprintln!("  files: {done}/{total}");
        }
    }
}

/// Persist indexed files and their symbols, returning `(files, symbols)` counts.
async fn store_files(
    pool: &PgPool,
    repo_id: Uuid,
    local_path: &Path,
    indexed: &[IndexedFile],
) -> (usize, usize) {
    let mut files_stored: usize = 0;
    let mut symbols_stored: usize = 0;

    for indexed_file in indexed {
        // ── 3a. Compute content hash & size ──────────────────────────────────
        let raw = match std::fs::read(&indexed_file.path) {
            Ok(b) => b,
            Err(e) => {
                warn!("Cannot re-read {:?}: {e}", indexed_file.path);
                continue;
            }
        };
        let content_hash = hex::encode(Sha256::digest(&raw));
        let size_bytes = i64::try_from(raw.len()).unwrap_or(i64::MAX);

        // ── 3b. Relative path from the repo root ─────────────────────────────
        let rel_path = indexed_file
            .path
            .strip_prefix(local_path)
            .unwrap_or(&indexed_file.path)
            .to_string_lossy()
            .into_owned();

        // ── 3c. Idempotency: skip if file already in DB (same hash) ──────────
        if let Ok(Some(existing)) = get_file_by_path(pool, repo_id, &rel_path).await {
            if existing.content_hash == content_hash {
                // File unchanged — re-use its id for symbol persistence.
                // Symbols already stored: skip.
                continue;
            }
        }

        // ── 3d. Persist file ─────────────────────────────────────────────────
        let file_create = FileCreate {
            repository_id: repo_id,
            path: rel_path,
            language: Some(lang_str(indexed_file.language).to_string()),
            size_bytes,
            content_hash,
        };
        let file_record = match create_file(pool, &file_create).await {
            Ok(r) => r,
            Err(e) => {
                warn!("DB error storing file {:?}: {e}", indexed_file.path);
                continue;
            }
        };
        files_stored += 1;

        // ── 3e. Persist symbols ───────────────────────────────────────────────
        for sym in &indexed_file.symbols {
            let sym_create = SymbolCreate {
                file_id: file_record.id,
                repository_id: repo_id,
                name: sym.name.clone(),
                symbol_type: indexer_kind_to_core(&sym.kind),
                language: lang_str(indexed_file.language).to_string(),
                line_start: i32::try_from(sym.line_start).unwrap_or(0),
                line_end: i32::try_from(sym.line_end).unwrap_or(0),
                col_start: i32::try_from(sym.col_start).unwrap_or(0),
                col_end: i32::try_from(sym.col_end).unwrap_or(0),
                visibility: sym.visibility.clone(),
                doc_comment: sym.doc_comment.clone(),
                raw_text: sym.name.clone(), // raw_text = name for now
            };
            match create_symbol(pool, &sym_create).await {
                Ok(_) => symbols_stored += 1,
                Err(e) => warn!("DB error storing symbol '{}': {e}", sym.name),
            }
        }
    }

    (files_stored, symbols_stored)
}

/// Persist the commit history, returning the number of newly stored commits.
async fn store_commits(pool: &PgPool, repo_id: Uuid, local_path: &Path) -> usize {
    let commits = walk_commits(local_path, &WalkFilter::default()).unwrap_or_default();
    let mut commits_stored: usize = 0;

    for commit_info in &commits {
        // Idempotency: skip already-stored commits.
        match get_commit_by_sha(pool, repo_id, &commit_info.sha).await {
            Ok(Some(_)) => continue,
            Ok(None) => {}
            Err(e) => {
                warn!("DB error checking commit {}: {e}", &commit_info.sha[..8]);
                continue;
            }
        }

        let commit_create = CommitCreate {
            repository_id: repo_id,
            sha: commit_info.sha.clone(),
            author_name: Some(commit_info.author_name.clone()),
            author_email: Some(commit_info.author_email.clone()),
            author_date: commit_info.author_date,
            committer_name: Some(commit_info.committer_name.clone()),
            committer_email: Some(commit_info.committer_email.clone()),
            committer_date: commit_info.committer_date,
            message: commit_info.message.clone(),
            parent_shas: commit_info.parent_shas.clone(),
        };
        match create_commit(pool, &commit_create).await {
            Ok(_) => commits_stored += 1,
            Err(e) => warn!("DB error storing commit {}: {e}", &commit_info.sha[..8]),
        }
    }

    commits_stored
}

/// Returns `(local_path, canonical_url)`.
///
/// If the target is a URL (contains `://` or starts with `git@`), it is cloned
/// into a temporary directory under `/tmp/archaeologist-cache/`.
/// Otherwise the path is used directly and the URL is set to the absolute path.
fn resolve_path(target: &str) -> Result<(PathBuf, String)> {
    let is_url = target.contains("://") || target.starts_with("git@");
    if is_url {
        // Use /tmp/archaeologist-fixtures as the mock remote for "remote" URLs
        // that point to the local fixture repository (as per project convention).
        let cache_dir = PathBuf::from("/tmp/archaeologist-cache");
        std::fs::create_dir_all(&cache_dir).context("create cache dir")?;

        // Derive a stable subdirectory name from the URL.
        let slug = target
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or("repo")
            .trim_end_matches(".git");
        let dest = cache_dir.join(slug);

        if dest.join(".git").exists() {
            info!("Using existing clone at {:?}", dest);
        } else {
            clone_repository(target, &dest, CloneOptions::default())
                .with_context(|| format!("clone {target}"))?;
        }
        Ok((dest, target.to_string()))
    } else {
        let path = PathBuf::from(target);
        let abs = path
            .canonicalize()
            .with_context(|| format!("resolve path '{target}'"))?;
        let url = format!("file://{}", abs.display());
        Ok((abs, url))
    }
}

/// Fetch the existing repository record or create a new one.
async fn upsert_repository(
    pool: &PgPool,
    url: &str,
    local_path: &Path,
    branch: Option<&String>,
) -> Result<archaeologist_core::models::Repository> {
    if let Some(existing) = get_repository_by_url(pool, url)
        .await
        .context("look up repository")?
    {
        info!("Repository already registered (id={})", existing.id);
        return Ok(existing);
    }

    let name = url
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or("unknown")
        .trim_end_matches(".git")
        .to_string();

    let create = RepositoryCreate {
        name,
        url: url.to_string(),
        local_path: Some(local_path.to_string_lossy().into_owned()),
        description: None,
        default_branch: branch.cloned(),
    };

    let repo = create_repository(pool, &create)
        .await
        .context("create repository")?;
    info!("Registered repository id={}", repo.id);
    Ok(repo)
}

fn lang_str(lang: Lang) -> &'static str {
    lang.as_str()
}

fn indexer_kind_to_core(
    kind: &archaeologist_indexer::SymbolKind,
) -> archaeologist_core::models::SymbolType {
    use archaeologist_core::models::SymbolType;
    use archaeologist_indexer::SymbolKind;
    match kind {
        SymbolKind::Method => SymbolType::Method,
        SymbolKind::Struct => SymbolType::Struct,
        SymbolKind::Enum => SymbolType::Enum,
        SymbolKind::Trait => SymbolType::Trait,
        SymbolKind::Impl => SymbolType::Impl,
        SymbolKind::Module => SymbolType::Module,
        SymbolKind::Class => SymbolType::Class,
        SymbolKind::Interface => SymbolType::Interface,
        SymbolKind::Type => SymbolType::Type,
        SymbolKind::Function | SymbolKind::Constructor => SymbolType::Function, // closest mapping
    }
}
