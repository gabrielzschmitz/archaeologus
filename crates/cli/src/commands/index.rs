//! `index` command — clone (if remote), parse, and persist a repository.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use anyhow::{Context, Result};
use archaeologist_core::models::{
    BranchCreate, CommitCreate, CommitFileCreate, FileCreate, RepositoryCreate, SymbolCommitCreate,
    SymbolCreate, SymbolDependencyCreate, TagCreate,
};
use archaeologist_db::PgPool;
use archaeologist_db::{
    create_pool,
    repositories::{
        create_commit, create_commit_file, create_file, create_repository,
        create_symbol_dependency, get_commit_by_sha, get_file_by_path, get_repository_by_url,
        update_repository_indexed, upsert_branch, upsert_symbol, upsert_symbol_commit, upsert_tag,
    },
    run_migrations,
};
use archaeologist_git::{
    clone_repository, diff_commit, list_branches, list_tags, walk_commits, CloneOptions, WalkFilter,
};
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

    // ── 4. Persist files + symbols; collect file-path → DB id map ───────────
    let (files_stored, symbols_stored, file_id_map, symbol_id_map) =
        store_files(&pool, repo_id, &local_path, &indexed).await;

    // ── 5. Persist commits ────────────────────────────────────────────────────
    let (commits_stored, commit_id_map) = store_commits(&pool, repo_id, &local_path).await;

    // ── 6. Persist commit_files (diff per commit) ────────────────────────────
    let commit_files_stored = store_commit_files(&pool, &local_path, &commit_id_map).await;

    // ── 7. Link symbols to commits (symbol_commits) ──────────────────────────
    let symbol_commits_stored = store_symbol_commits(
        &pool,
        &local_path,
        &file_id_map,
        &symbol_id_map,
        &commit_id_map,
    )
    .await;

    // ── 8. Persist branches and tags ─────────────────────────────────────────
    let (branches_stored, tags_stored) = store_refs(&pool, repo_id, &local_path).await;

    // ── 9. Persist symbol dependencies ───────────────────────────────────────
    let deps_stored =
        store_symbol_dependencies(&pool, repo_id, &local_path, &indexed, &file_id_map).await;

    // ── 10. Mark repository as indexed ───────────────────────────────────────
    update_repository_indexed(&pool, repo_id)
        .await
        .context("update indexed_at")?;

    println!(
        "✓ Indexed '{canonical_url}': \
         {files_stored} files, {symbols_stored} symbols, {commits_stored} commits, \
         {commit_files_stored} commit_files, {symbol_commits_stored} symbol_commits, \
         {branches_stored} branches, {tags_stored} tags, {deps_stored} dependencies"
    );

    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

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

/// Persist indexed files and their symbols.
///
/// Returns `(files_stored, symbols_stored, file_path→id, symbol_name→[id])`.
async fn store_files(
    pool: &PgPool,
    repo_id: Uuid,
    local_path: &Path,
    indexed: &[IndexedFile],
) -> (
    usize,
    usize,
    HashMap<String, Uuid>,
    HashMap<String, Vec<Uuid>>,
) {
    let mut files_stored: usize = 0;
    let mut symbols_stored: usize = 0;
    // rel_path → file DB id
    let mut file_id_map: HashMap<String, Uuid> = HashMap::new();
    // symbol name → [symbol DB ids]  (multiple files can have the same name)
    let mut symbol_id_map: HashMap<String, Vec<Uuid>> = HashMap::new();

    for indexed_file in indexed {
        let raw = match std::fs::read(&indexed_file.path) {
            Ok(b) => b,
            Err(e) => {
                warn!("Cannot re-read {:?}: {e}", indexed_file.path);
                continue;
            }
        };
        let content_hash = hex::encode(Sha256::digest(&raw));
        let size_bytes = i64::try_from(raw.len()).unwrap_or(i64::MAX);

        let rel_path = indexed_file
            .path
            .strip_prefix(local_path)
            .unwrap_or(&indexed_file.path)
            .to_string_lossy()
            .into_owned();

        // Idempotency: reuse the existing file id if content is unchanged,
        // but still upsert symbols so raw_text is always up-to-date.
        let file_id = if let Ok(Some(existing)) = get_file_by_path(pool, repo_id, &rel_path).await {
            if existing.content_hash == content_hash {
                file_id_map.insert(rel_path.clone(), existing.id);
                existing.id
            } else {
                let file_create = FileCreate {
                    repository_id: repo_id,
                    path: rel_path.clone(),
                    language: Some(lang_str(indexed_file.language).to_string()),
                    size_bytes,
                    content_hash,
                };
                match create_file(pool, &file_create).await {
                    Ok(r) => {
                        files_stored += 1;
                        file_id_map.insert(rel_path.clone(), r.id);
                        r.id
                    }
                    Err(e) => {
                        warn!("DB error storing file {:?}: {e}", indexed_file.path);
                        continue;
                    }
                }
            }
        } else {
            let file_create = FileCreate {
                repository_id: repo_id,
                path: rel_path.clone(),
                language: Some(lang_str(indexed_file.language).to_string()),
                size_bytes,
                content_hash,
            };
            match create_file(pool, &file_create).await {
                Ok(r) => {
                    files_stored += 1;
                    file_id_map.insert(rel_path.clone(), r.id);
                    r.id
                }
                Err(e) => {
                    warn!("DB error storing file {:?}: {e}", indexed_file.path);
                    continue;
                }
            }
        };

        // Always upsert symbols so raw_text stays fresh even on re-index.
        for sym in &indexed_file.symbols {
            let sym_create = SymbolCreate {
                file_id,
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
                raw_text: sym.raw_text.clone(),
            };
            match upsert_symbol(pool, &sym_create).await {
                Ok(s) => {
                    symbols_stored += 1;
                    symbol_id_map
                        .entry(sym.name.clone())
                        .or_default()
                        .push(s.id);
                }
                Err(e) => warn!("DB error upserting symbol '{}': {e}", sym.name),
            }
        }
    }

    (files_stored, symbols_stored, file_id_map, symbol_id_map)
}

/// Persist commits; returns `(count, sha → DB id)`.
async fn store_commits(
    pool: &PgPool,
    repo_id: Uuid,
    local_path: &Path,
) -> (usize, HashMap<String, Uuid>) {
    let commits = walk_commits(local_path, &WalkFilter::default()).unwrap_or_default();
    let mut commits_stored: usize = 0;
    let mut commit_id_map: HashMap<String, Uuid> = HashMap::new();

    for commit_info in &commits {
        match get_commit_by_sha(pool, repo_id, &commit_info.sha).await {
            Ok(Some(existing)) => {
                commit_id_map.insert(commit_info.sha.clone(), existing.id);
                continue;
            }
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
            Ok(c) => {
                commits_stored += 1;
                commit_id_map.insert(commit_info.sha.clone(), c.id);
            }
            Err(e) => warn!("DB error storing commit {}: {e}", &commit_info.sha[..8]),
        }
    }

    (commits_stored, commit_id_map)
}

/// For every commit, diff it and write the changed-file rows into `commit_files`.
async fn store_commit_files(
    pool: &PgPool,
    local_path: &Path,
    commit_id_map: &HashMap<String, Uuid>,
) -> usize {
    let mut stored: usize = 0;

    for (sha, &commit_db_id) in commit_id_map {
        let diff_files = match diff_commit(local_path, sha) {
            Ok(f) => f,
            Err(e) => {
                warn!("diff_commit {}: {e}", &sha[..8]);
                continue;
            }
        };

        for df in &diff_files {
            let cf = CommitFileCreate {
                commit_id: commit_db_id,
                file_path: df.file_path.clone(),
                status: format!("{:?}", df.status).to_lowercase(),
                additions: i32::try_from(df.additions).unwrap_or(i32::MAX),
                deletions: i32::try_from(df.deletions).unwrap_or(i32::MAX),
                old_path: df.old_path.clone(),
            };
            match create_commit_file(pool, &cf).await {
                Ok(_) => stored += 1,
                Err(e) => warn!("DB error storing commit_file for {}: {e}", &sha[..8]),
            }
        }
    }

    stored
}

/// Link symbols to the commits that touched their file.
///
/// For every commit→file entry we check whether any stored symbol lives in
/// that file; if so, we insert a `symbol_commits` row with the appropriate
/// `change_type`.
async fn store_symbol_commits(
    pool: &PgPool,
    local_path: &Path,
    file_id_map: &HashMap<String, Uuid>,
    symbol_id_map: &HashMap<String, Vec<Uuid>>,
    commit_id_map: &HashMap<String, Uuid>,
) -> usize {
    // Build a reverse map: file_db_id → [symbol_db_ids]
    // We need per-file symbol lists — requery from the symbol_id_map is
    // impractical, so we build file→symbols from the indexed data instead.
    // Since file_id_map maps rel_path→file_id and symbol_id_map maps
    // name→[sym_id], we need a per-file symbol list. Build it via a
    // separate query isn't available here, so we use the file_path diff
    // data and match rel_path directly.

    let mut stored: usize = 0;

    for (sha, &commit_db_id) in commit_id_map {
        let diff_files = match diff_commit(local_path, sha) {
            Ok(f) => f,
            Err(e) => {
                warn!("symbol_commits diff {}: {e}", &sha[..8]);
                continue;
            }
        };

        for df in &diff_files {
            let file_db_id = match file_id_map.get(&df.file_path) {
                Some(id) => *id,
                None => continue, // file not a supported language; skip
            };

            // Collect every symbol that belongs to this file.
            // symbol_id_map is keyed by name; we need to resolve by file.
            // Since we don't have a direct file→symbols map here, we use
            // pool query to list symbols for the file.
            let syms =
                match sqlx::query_as::<_, (Uuid,)>("SELECT id FROM symbols WHERE file_id = $1")
                    .bind(file_db_id)
                    .fetch_all(pool)
                    .await
                {
                    Ok(rows) => rows,
                    Err(e) => {
                        warn!("symbol_commits query symbols: {e}");
                        continue;
                    }
                };

            let change_type = match df.status {
                archaeologist_git::FileStatus::Added => "added",
                archaeologist_git::FileStatus::Deleted => "deleted",
                _ => "modified",
            };

            for (sym_id,) in syms {
                let sc = SymbolCommitCreate {
                    symbol_id: sym_id,
                    commit_id: commit_db_id,
                    change_type: change_type.to_string(),
                };
                match upsert_symbol_commit(pool, &sc).await {
                    Ok(_) => stored += 1,
                    Err(e) => warn!("symbol_commits upsert: {e}"),
                }
            }
        }
    }

    // Suppress unused-variable warning for symbol_id_map (kept in signature
    // for future direct-lookup optimisations).
    let _ = symbol_id_map;

    stored
}

/// Upsert branches and lightweight tags; returns `(branches, tags)` counts.
async fn store_refs(pool: &PgPool, repo_id: Uuid, local_path: &Path) -> (usize, usize) {
    let mut branches_stored: usize = 0;
    let mut tags_stored: usize = 0;

    // Branches
    match list_branches(local_path) {
        Ok(branches) => {
            for b in &branches {
                let bc = BranchCreate {
                    repository_id: repo_id,
                    name: b.name.clone(),
                    head_sha: b.head_sha.clone(),
                    is_default: b.is_default,
                };
                match upsert_branch(pool, &bc).await {
                    Ok(_) => branches_stored += 1,
                    Err(e) => warn!("DB error storing branch '{}': {e}", b.name),
                }
            }
        }
        Err(e) => warn!("list_branches: {e}"),
    }

    // Tags
    match list_tags(local_path) {
        Ok(tags) => {
            for t in &tags {
                let tc = TagCreate {
                    repository_id: repo_id,
                    name: t.name.clone(),
                    target_sha: t.target_sha.clone(),
                };
                match upsert_tag(pool, &tc).await {
                    Ok(_) => tags_stored += 1,
                    Err(e) => warn!("DB error storing tag '{}': {e}", t.name),
                }
            }
        }
        Err(e) => warn!("list_tags: {e}"),
    }

    (branches_stored, tags_stored)
}

/// Persist `symbol_dependencies` rows derived from the indexer's dependency
/// extraction.
///
/// Each dependency is attached to the *single* symbol whose source line-range
/// contains the dependency's line.  If no symbol spans that line (e.g. a
/// top-level import above all function definitions) we fall back to the symbol
/// with the smallest `line_start` in the file.  This avoids the previous
/// behaviour of fan-out where every dependency was duplicated for every symbol.
///
/// `depends_on_symbol_id` is set only when the dependency target is a plain
/// identifier (no dots, no parentheses) that exactly matches a symbol name
/// in the same repository.
async fn store_symbol_dependencies(
    pool: &PgPool,
    repo_id: Uuid,
    local_path: &Path,
    indexed: &[IndexedFile],
    file_id_map: &HashMap<String, Uuid>,
) -> usize {
    let mut stored: usize = 0;

    for indexed_file in indexed {
        if indexed_file.dependencies.is_empty() {
            continue;
        }

        let rel_path = indexed_file
            .path
            .strip_prefix(local_path)
            .unwrap_or(&indexed_file.path)
            .to_string_lossy()
            .into_owned();

        let file_db_id = match file_id_map.get(&rel_path) {
            Some(id) => *id,
            None => continue,
        };

        // Fetch symbols for this file with their line ranges.
        // Columns: (id, line_start, line_end)
        let syms: Vec<(Uuid, i32, i32)> = match sqlx::query_as::<_, (Uuid, i32, i32)>(
            "SELECT id, line_start, line_end FROM symbols WHERE file_id = $1 ORDER BY line_start",
        )
        .bind(file_db_id)
        .fetch_all(pool)
        .await
        {
            Ok(rows) => rows,
            Err(e) => {
                warn!("symbol_deps query symbols: {e}");
                continue;
            }
        };

        if syms.is_empty() {
            continue;
        }

        // Pre-resolve only simple identifiers (no dots/parens/spaces) against
        // known symbol names — these are the only ones that can match.
        // We prefer a symbol in the *same language* to avoid cross-language
        // false matches (e.g. Python's `User` resolving to TypeScript's `User`).
        let file_lang = lang_str(indexed_file.language);
        let mut resolved: HashMap<String, Uuid> = HashMap::new();
        for dep in &indexed_file.dependencies {
            let target = dep.target.trim();
            // Only attempt resolution for plain single identifiers.
            if target.contains('.') || target.contains('(') || target.contains(' ') {
                continue;
            }
            if resolved.contains_key(target) {
                continue;
            }
            // 1st choice: same language
            let row = sqlx::query_as::<_, (Uuid,)>(
                "SELECT id FROM symbols \
                 WHERE repository_id = $1 AND name = $2 AND language = $3 LIMIT 1",
            )
            .bind(repo_id)
            .bind(target)
            .bind(file_lang)
            .fetch_optional(pool)
            .await;

            if let Ok(Some((id,))) = row {
                resolved.insert(target.to_string(), id);
                continue;
            }

            // 2nd choice: any language in the same repo (cross-language dep)
            if let Ok(Some((id,))) = sqlx::query_as::<_, (Uuid,)>(
                "SELECT id FROM symbols WHERE repository_id = $1 AND name = $2 LIMIT 1",
            )
            .bind(repo_id)
            .bind(target)
            .fetch_optional(pool)
            .await
            {
                resolved.insert(target.to_string(), id);
            }
        }

        // The fallback symbol is the one with the smallest line_start.
        let fallback_sym_id = syms[0].0;

        for dep in &indexed_file.dependencies {
            // Find the innermost symbol that contains dep.line.
            let dep_line = i32::try_from(dep.line).unwrap_or(0);
            let sym_id = syms
                .iter()
                .filter(|(_, start, end)| *start <= dep_line && dep_line <= *end)
                // Among all spanning symbols, pick the one with the largest
                // line_start (most specific / innermost).
                .max_by_key(|(_, start, _)| *start)
                .map_or(fallback_sym_id, |(id, _, _)| *id);

            let dep_type = match dep.kind {
                archaeologist_indexer::DependencyKind::Import => "import",
                archaeologist_indexer::DependencyKind::Call => "call",
                archaeologist_indexer::DependencyKind::TraitImpl => "trait_impl",
            };

            let target = dep.target.trim();
            let depends_on = resolved.get(target).copied();

            let sd = SymbolDependencyCreate {
                symbol_id: sym_id,
                depends_on_symbol_id: depends_on,
                dependency_name: dep.target.clone(),
                dependency_type: dep_type.to_string(),
            };
            match create_symbol_dependency(pool, &sd).await {
                Ok(_) => stored += 1,
                Err(e) => warn!("symbol_dep insert: {e}"),
            }
        }
    }

    stored
}

// ── Path / repo helpers ───────────────────────────────────────────────────────

/// Returns `(local_path, canonical_url)`.
fn resolve_path(target: &str) -> Result<(PathBuf, String)> {
    let is_url = target.contains("://") || target.starts_with("git@");
    if is_url {
        let cache_dir = PathBuf::from("/tmp/archaeologist-cache");
        std::fs::create_dir_all(&cache_dir).context("create cache dir")?;

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

/// Fetch or create the repository DB record.
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
        SymbolKind::Function => SymbolType::Function,
        SymbolKind::Constructor => SymbolType::Constructor,
        SymbolKind::Method => SymbolType::Method,
        SymbolKind::Struct => SymbolType::Struct,
        SymbolKind::Enum => SymbolType::Enum,
        SymbolKind::Trait => SymbolType::Trait,
        SymbolKind::Impl => SymbolType::Impl,
        SymbolKind::Module => SymbolType::Module,
        SymbolKind::Class => SymbolType::Class,
        SymbolKind::Interface => SymbolType::Interface,
        SymbolKind::Type => SymbolType::Type,
    }
}
