//! MCP server — [`ArchaeologistServer`] exposes all archaeologist capabilities
//! as MCP tools via the rmcp crate.

use archaeologist_core::models::Symbol;
use archaeologist_db::repositories::{
    get_commit, get_evidence_for_symbol, list_symbol_commits, list_symbol_dependencies,
};
use archaeologist_evidence::{
    aggregate_evidence, collect_from_commits, collect_from_db, deduplicate_and_rank,
    explain_symbol as ev_explain_symbol,
};
use archaeologist_search::code_search::{search_code, CodeQuery};
use archaeologist_search::symbol_search::{search_symbols, SymbolQuery};
use rmcp::{
    handler::server::router::tool::ToolRouter, handler::server::wrapper::Parameters,
    model::ServerInfo, tool, tool_handler, tool_router, ServerHandler,
};
use schemars::JsonSchema;
use serde::Deserialize;
use sqlx::PgPool;
use std::collections::{HashMap, HashSet};
use tracing::info;
use uuid::Uuid;

// ── Tool input types ──────────────────────────────────────────────────────────
// Each tool gets its own `#[derive(Deserialize, JsonSchema)]` input struct so
// rmcp can auto-generate the JSON Schema for the tool definition.

#[derive(Debug, Deserialize, JsonSchema)]
pub struct IndexRepositoryInput {
    /// Git repository URL or local path to index.
    pub url: String,
    /// Branch to record (optional).
    pub branch: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchSymbolsInput {
    /// Search query (fuzzy-matched against symbol names).
    pub query: String,
    /// Restrict to a specific repository by UUID.
    pub repository_id: Option<uuid::Uuid>,
    /// Filter by symbol type (function, struct, class, …).
    pub symbol_type: Option<String>,
    /// Filter by language (rust, python, go, …).
    pub language: Option<String>,
    /// Maximum results (default 20).
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ExplainSymbolInput {
    /// Symbol name to explain.
    pub symbol_name: String,
    /// Narrow down by file path (optional).
    pub file_path: Option<String>,
    /// Narrow down by repository UUID (optional).
    pub repository_id: Option<uuid::Uuid>,
    /// Filter by language (optional).
    pub language: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetHistoryInput {
    /// Symbol name to get commit history for.
    pub symbol_name: String,
    /// Narrow down by repository UUID (optional).
    pub repository_id: Option<uuid::Uuid>,
    /// Filter by language (optional).
    pub language: Option<String>,
    /// Maximum number of commits to return (default 50).
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AnalyzeImpactInput {
    /// Symbol name to analyse the blast radius of.
    pub symbol_name: String,
    /// Narrow down by repository UUID (optional).
    pub repository_id: Option<uuid::Uuid>,
    /// Filter by language (optional).
    pub language: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetEvidenceInput {
    /// Symbol name to gather evidence for.
    pub symbol_name: String,
    /// Narrow down by repository UUID (optional).
    pub repository_id: Option<uuid::Uuid>,
    /// Filter by language (optional).
    pub language: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchCodeInput {
    /// Search query matched against symbol raw text and names.
    pub query: String,
    /// Narrow down by repository UUID (optional).
    pub repository_id: Option<uuid::Uuid>,
    /// Filter by language (optional).
    pub language: Option<String>,
    /// Maximum results (default 20).
    pub limit: Option<i64>,
}

// ── Server ────────────────────────────────────────────────────────────────────

/// The main MCP server.  Holds a database pool and exposes archaeologist
/// capabilities as MCP tools.
#[derive(Clone)]
pub struct ArchaeologistServer {
    pub pool: PgPool,
    tool_router: ToolRouter<Self>,
}

// Wire the `#[tool]` methods into the tool router.
#[allow(clippy::unused_async_trait_impl)]
#[tool_handler(router = self.tool_router)]
impl ServerHandler for ArchaeologistServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            rmcp::model::ServerCapabilities::builder()
                .enable_tools()
                .build(),
        )
        .with_instructions(
            "AI Software Archaeologist — answers 'why is the code like this?'. \
             Index repositories, search symbols, analyse history, understand code.",
        )
    }
}

// Define all MCP tools.
#[tool_router(router = tool_router)]
impl ArchaeologistServer {
    /// Create a new server instance.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            tool_router: Self::tool_router(),
        }
    }

    // ── index_repository ─────────────────────────────────────────────────────

    #[tool(
        description = "Index a git repository for analysis. Clones the repository if needed and indexes all source files. Returns indexing statistics."
    )]
    pub async fn index_repository(
        &self,
        params: Parameters<IndexRepositoryInput>,
    ) -> Result<String, rmcp::ErrorData> {
        let input = params.0;
        info!(url = %input.url, "MCP: index_repository");
        index_repository_impl(&self.pool, &input.url, input.branch.as_deref())
            .await
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))
    }

    // ── search_symbols ────────────────────────────────────────────────────────

    #[tool(
        description = "Search for symbols (functions, structs, enums, traits, classes, …) in indexed repositories."
    )]
    pub async fn search_symbols(
        &self,
        params: Parameters<SearchSymbolsInput>,
    ) -> Result<String, rmcp::ErrorData> {
        let input = params.0;

        let mut q = SymbolQuery::new(&input.query).limit(input.limit.unwrap_or(20));
        if let Some(repo_id) = input.repository_id {
            q = q.repo(repo_id);
        }
        if let Some(ref st) = input.symbol_type {
            q = q.symbol_type(st.as_str());
        }
        if let Some(ref lang) = input.language {
            q = q.language(lang.as_str());
        }

        let result = search_symbols(&self.pool, &q)
            .await
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;

        let output = serde_json::json!({
            "total": result.total,
            "limit": result.limit,
            "offset": result.offset,
            "symbols": result.items,
        });

        serde_json::to_string_pretty(&output)
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))
    }

    // ── explain_symbol ────────────────────────────────────────────────────────

    #[tool(description = "Explain a symbol's purpose, origin, author, history, and dependencies.")]
    pub async fn explain_symbol(
        &self,
        params: Parameters<ExplainSymbolInput>,
    ) -> Result<String, rmcp::ErrorData> {
        let input = params.0;

        let mut q = SymbolQuery::new(&input.symbol_name).limit(5);
        if let Some(repo_id) = input.repository_id {
            q = q.repo(repo_id);
        }
        if let Some(ref lang) = input.language {
            q = q.language(lang.as_str());
        }

        let result = search_symbols(&self.pool, &q)
            .await
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;

        if result.items.is_empty() {
            return Ok(format!(
                "No symbol matching '{name}' found. Index a repository first.",
                name = input.symbol_name
            ));
        }

        let mut explanations = Vec::new();

        for sym in &result.items {
            // Fetch commits.
            let sc_links = list_symbol_commits(&self.pool, sym.id)
                .await
                .unwrap_or_default();
            let mut commits = Vec::new();
            for link in &sc_links {
                if let Ok(Some(c)) = get_commit(&self.pool, link.commit_id).await {
                    commits.push(c);
                }
            }

            // Fetch DB evidence.
            let db_ev = get_evidence_for_symbol(&self.pool, sym.id)
                .await
                .unwrap_or_default();

            // Fetch dependencies.
            let deps = list_symbol_dependencies(&self.pool, sym.id)
                .await
                .unwrap_or_default();

            let evidence = aggregate_evidence(sym.id, Some(sym), &commits, &[], &db_ev);
            let explanation = ev_explain_symbol(&sym.name, &evidence);

            explanations.push(serde_json::json!({
                "symbol": {
                    "name": sym.name,
                    "type": sym.symbol_type,
                    "language": sym.language,
                    "file_id": sym.file_id,
                    "line_start": sym.line_start,
                    "line_end": sym.line_end,
                    "visibility": sym.visibility,
                    "doc_comment": sym.doc_comment,
                },
                "summary": explanation.summary,
                "confidence": explanation.confidence.to_string(),
                "citations": explanation.citations.iter().map(|c| serde_json::json!({
                    "source_type": c.source_type,
                    "content": c.content,
                    "source_ref": c.source_ref,
                    "score": c.score,
                })).collect::<Vec<_>>(),
                "commits": commits.iter().take(10).map(|c| serde_json::json!({
                    "sha": c.sha,
                    "author": c.author_name,
                    "date": c.author_date.to_rfc3339(),
                    "message": c.message.lines().next().unwrap_or("").trim(),
                })).collect::<Vec<_>>(),
                "dependencies": deps.iter().map(|d| serde_json::json!({
                    "name": d.dependency_name,
                    "type": d.dependency_type,
                })).collect::<Vec<_>>(),
            }));
        }

        serde_json::to_string_pretty(&explanations)
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))
    }

    // ── get_history ───────────────────────────────────────────────────────────

    #[tool(description = "Get commit history for a symbol — who changed it, when, and why.")]
    pub async fn get_history(
        &self,
        params: Parameters<GetHistoryInput>,
    ) -> Result<String, rmcp::ErrorData> {
        let input = params.0;

        let limit = usize::try_from(input.limit.unwrap_or(50)).unwrap_or(usize::MAX);
        let mut q = SymbolQuery::new(&input.symbol_name).limit(5);
        if let Some(repo_id) = input.repository_id {
            q = q.repo(repo_id);
        }
        if let Some(ref lang) = input.language {
            q = q.language(lang.as_str());
        }

        let result = search_symbols(&self.pool, &q)
            .await
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;

        if result.items.is_empty() {
            return Ok(format!(
                "No symbol matching '{name}' found.",
                name = input.symbol_name
            ));
        }

        let mut all_history = Vec::new();
        for sym in &result.items {
            let sc_links = list_symbol_commits(&self.pool, sym.id)
                .await
                .unwrap_or_default();
            let mut commits = Vec::new();
            for link in &sc_links {
                if let Ok(Some(c)) = get_commit(&self.pool, link.commit_id).await {
                    commits.push((c, link.change_type.clone()));
                }
            }
            // Sort newest first.
            commits.sort_by_key(|(c, _)| std::cmp::Reverse(c.author_date));

            let history: Vec<serde_json::Value> = commits
                .iter()
                .take(limit)
                .map(|(c, ct)| {
                    serde_json::json!({
                        "sha": &c.sha[..c.sha.len().min(8)],
                        "sha_full": c.sha,
                        "author": c.author_name,
                        "author_email": c.author_email,
                        "date": c.author_date.to_rfc3339(),
                        "message": c.message.lines().next().unwrap_or("").trim(),
                        "change_type": ct,
                    })
                })
                .collect();

            all_history.push(serde_json::json!({
                "symbol": {
                    "name": sym.name,
                    "type": sym.symbol_type,
                    "language": sym.language,
                },
                "commits": history,
                "total_commits": commits.len(),
            }));
        }

        serde_json::to_string_pretty(&all_history)
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))
    }

    // ── analyze_impact ────────────────────────────────────────────────────────

    #[tool(
        description = "Analyze the impact of changing a symbol — direct callers, indirect callers, tests, and risk level."
    )]
    pub async fn analyze_impact(
        &self,
        params: Parameters<AnalyzeImpactInput>,
    ) -> Result<String, rmcp::ErrorData> {
        let input = params.0;

        let mut q = SymbolQuery::new(&input.symbol_name).limit(5);
        if let Some(repo_id) = input.repository_id {
            q = q.repo(repo_id);
        }
        if let Some(ref lang) = input.language {
            q = q.language(lang.as_str());
        }

        let result = search_symbols(&self.pool, &q)
            .await
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;

        if result.items.is_empty() {
            return Ok(format!(
                "No symbol matching '{name}' found.",
                name = input.symbol_name
            ));
        }

        let mut reports = Vec::new();
        for sym in &result.items {
            let direct = self.find_callers(sym.id).await;

            let mut indirect: Vec<Symbol> = Vec::new();
            let mut seen: HashSet<Uuid> = direct.iter().map(|s| s.id).collect();
            seen.insert(sym.id);
            for caller in &direct {
                for s in self.find_callers(caller.id).await {
                    if seen.insert(s.id) {
                        indirect.push(s);
                    }
                }
            }

            let is_test = |s: &Symbol| -> bool {
                let n = s.name.to_lowercase();
                n.starts_with("test") || n.ends_with("_test") || n.contains("_test_")
            };

            let test_count = direct
                .iter()
                .chain(indirect.iter())
                .filter(|s| is_test(s))
                .count();
            let total = direct.len() + indirect.len();
            let risk = estimate_risk(total, test_count);

            reports.push(serde_json::json!({
                "symbol": {
                    "name": sym.name,
                    "type": sym.symbol_type,
                    "language": sym.language,
                },
                "direct_callers": direct.iter().map(|s| serde_json::json!({
                    "name": s.name, "type": s.symbol_type, "language": s.language,
                    "is_test": is_test(s),
                })).collect::<Vec<_>>(),
                "indirect_callers": indirect.iter().map(|s| serde_json::json!({
                    "name": s.name, "type": s.symbol_type, "language": s.language,
                    "is_test": is_test(s),
                })).collect::<Vec<_>>(),
                "test_count": test_count,
                "total_affected": total,
                "risk_level": risk,
            }));
        }

        serde_json::to_string_pretty(&reports)
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))
    }

    // ── get_evidence ──────────────────────────────────────────────────────────

    #[tool(
        description = "Get all evidence for why a piece of code exists — commits, blame, doc comments, and database records."
    )]
    pub async fn get_evidence(
        &self,
        params: Parameters<GetEvidenceInput>,
    ) -> Result<String, rmcp::ErrorData> {
        let input = params.0;

        let mut q = SymbolQuery::new(&input.symbol_name).limit(5);
        if let Some(repo_id) = input.repository_id {
            q = q.repo(repo_id);
        }
        if let Some(ref lang) = input.language {
            q = q.language(lang.as_str());
        }

        let result = search_symbols(&self.pool, &q)
            .await
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;

        if result.items.is_empty() {
            return Ok(format!(
                "No symbol matching '{name}' found.",
                name = input.symbol_name
            ));
        }

        let mut all_evidence = Vec::new();
        for sym in &result.items {
            let sc_links = list_symbol_commits(&self.pool, sym.id)
                .await
                .unwrap_or_default();
            let mut commits = Vec::new();
            for link in &sc_links {
                if let Ok(Some(c)) = get_commit(&self.pool, link.commit_id).await {
                    commits.push(c);
                }
            }
            let db_ev = get_evidence_for_symbol(&self.pool, sym.id)
                .await
                .unwrap_or_default();

            let mut items = collect_from_commits(&commits);
            items.extend(collect_from_db(&db_ev));
            let ranked = deduplicate_and_rank(items);

            all_evidence.push(serde_json::json!({
                "symbol": sym.name,
                "evidence": ranked.iter().map(|e| serde_json::json!({
                    "source": e.source.as_str(),
                    "content": e.content,
                    "source_ref": e.source_ref,
                    "weight": e.weight,
                })).collect::<Vec<_>>(),
            }));
        }

        serde_json::to_string_pretty(&all_evidence)
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))
    }

    // ── search_code ───────────────────────────────────────────────────────────

    #[tool(description = "Search code content across the codebase using fuzzy matching.")]
    pub async fn search_code(
        &self,
        params: Parameters<SearchCodeInput>,
    ) -> Result<String, rmcp::ErrorData> {
        let input = params.0;

        let mut q = CodeQuery::new(&input.query).limit(input.limit.unwrap_or(20));
        if let Some(repo_id) = input.repository_id {
            q = q.repo(repo_id);
        }
        if let Some(ref lang) = input.language {
            q = q.language(lang.as_str());
        }

        let result = search_code(&self.pool, &q)
            .await
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;

        let output = serde_json::json!({
            "total": result.total,
            "symbols": result.items,
        });

        serde_json::to_string_pretty(&output)
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))
    }

    // ── list_repositories ─────────────────────────────────────────────────────

    #[tool(description = "List all indexed repositories.")]
    pub async fn list_repositories(&self) -> Result<String, rmcp::ErrorData> {
        use archaeologist_db::repositories::list_repositories;

        let repos = list_repositories(&self.pool)
            .await
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;

        serde_json::to_string_pretty(&repos)
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))
    }
}

// ── Private helpers ───────────────────────────────────────────────────────────

impl ArchaeologistServer {
    /// Find all symbols that have a direct dependency on `target_id`.
    async fn find_callers(&self, target_id: uuid::Uuid) -> Vec<archaeologist_core::models::Symbol> {
        sqlx::query_as(
            "SELECT s.id, s.file_id, s.repository_id, s.name, s.symbol_type, s.language,
                    s.line_start, s.line_end, s.col_start, s.col_end,
                    s.visibility, s.doc_comment, s.raw_text, s.created_at
             FROM symbol_dependencies sd
             JOIN symbols s ON s.id = sd.symbol_id
             WHERE sd.depends_on_symbol_id = $1",
        )
        .bind(target_id)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default()
    }
}

// ── Risk estimate ─────────────────────────────────────────────────────────────

fn estimate_risk(total_callers: usize, test_count: usize) -> &'static str {
    match (total_callers, test_count) {
        (0, _) => "LOW",
        (1..=5, t) if t > 0 => "LOW",
        (1..=5, _) => "MEDIUM",
        (6..=20, t) if t > 0 => "MEDIUM",
        (6..=20, _) => "HIGH",
        (_, t) if t > 0 => "HIGH",
        _ => "CRITICAL",
    }
}

// ── Index repository (full pipeline) ─────────────────────────────────────────

async fn index_repository_impl(
    pool: &PgPool,
    target: &str,
    branch: Option<&str>,
) -> anyhow::Result<String> {
    use anyhow::Context;
    use archaeologist_db::repositories::update_repository_indexed;
    use archaeologist_indexer::index_directory;

    let (local_path, canonical_url) = resolve_local_path(target)?;

    let repo_id = upsert_repository(pool, &canonical_url, &local_path, branch).await?;

    let indexed = index_directory(&local_path, |_, _| {}).context("index directory")?;

    let (files_stored, symbols_stored, file_id_map) =
        store_files_and_symbols(pool, repo_id, &local_path, &indexed).await?;

    let (commits_stored, commit_id_map) = store_commits(pool, repo_id, &local_path).await?;

    let commit_files_stored = store_commit_files(pool, &local_path, &commit_id_map).await?;

    let symbol_commits_stored =
        store_symbol_commits(pool, &local_path, &commit_id_map, &file_id_map).await?;

    let (branches_stored, tags_stored) = store_refs(pool, repo_id, &local_path).await?;

    let deps_stored =
        store_dependencies(pool, repo_id, &local_path, &indexed, &file_id_map).await?;

    update_repository_indexed(pool, repo_id).await.ok();

    Ok(format!(
        "Indexed '{canonical_url}': {files_stored} files, {symbols_stored} symbols, \
         {commits_stored} commits, {commit_files_stored} commit_files, \
         {symbol_commits_stored} symbol_commits, {branches_stored} branches, \
         {tags_stored} tags, {deps_stored} dependencies"
    ))
}

fn resolve_local_path(target: &str) -> anyhow::Result<(std::path::PathBuf, String)> {
    use anyhow::Context;
    use archaeologist_git::CloneOptions;

    let is_url = target.contains("://") || target.starts_with("git@");
    let (local_path, canonical_url) = if is_url {
        let cache_dir = std::path::PathBuf::from("/tmp/archaeologist-cache");
        std::fs::create_dir_all(&cache_dir).context("create cache dir")?;
        let slug = target
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or("repo")
            .trim_end_matches(".git");
        let dest = cache_dir.join(slug);
        if !dest.join(".git").exists() {
            archaeologist_git::clone_repository(target, &dest, CloneOptions::default())
                .with_context(|| format!("clone {target}"))?;
        }
        (dest, target.to_string())
    } else {
        let path = std::path::PathBuf::from(target);
        let abs = path
            .canonicalize()
            .with_context(|| format!("resolve path '{target}'"))?;
        let url = format!("file://{}", abs.display());
        (abs, url)
    };
    Ok((local_path, canonical_url))
}

async fn upsert_repository(
    pool: &PgPool,
    canonical_url: &str,
    local_path: &std::path::Path,
    branch: Option<&str>,
) -> anyhow::Result<Uuid> {
    use anyhow::Context;
    use archaeologist_core::models::RepositoryCreate;
    use archaeologist_db::repositories::{create_repository, get_repository_by_url};

    let repo = if let Some(existing) = get_repository_by_url(pool, canonical_url)
        .await
        .context("look up repository")?
    {
        existing
    } else {
        let name = canonical_url
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or("unknown")
            .trim_end_matches(".git")
            .to_string();
        let create = RepositoryCreate {
            name,
            url: canonical_url.to_string(),
            local_path: Some(local_path.to_string_lossy().into_owned()),
            description: None,
            default_branch: branch.map(str::to_string),
        };
        create_repository(pool, &create)
            .await
            .context("create repository")?
    };
    Ok(repo.id)
}

async fn store_commit_files(
    pool: &PgPool,
    local_path: &std::path::Path,
    commit_id_map: &HashMap<String, Uuid>,
) -> anyhow::Result<usize> {
    use archaeologist_core::models::CommitFileCreate;
    use archaeologist_db::repositories::create_commit_file;
    use archaeologist_git::diff_commit;

    let mut commit_files_stored: usize = 0;
    for (sha, &cid) in commit_id_map {
        let dfs = diff_commit(local_path, sha).unwrap_or_default();
        for df in &dfs {
            let cf = CommitFileCreate {
                commit_id: cid,
                file_path: df.file_path.clone(),
                status: format!("{:?}", df.status).to_lowercase(),
                additions: i32::try_from(df.additions).unwrap_or(i32::MAX),
                deletions: i32::try_from(df.deletions).unwrap_or(i32::MAX),
                old_path: df.old_path.clone(),
            };
            if create_commit_file(pool, &cf).await.is_ok() {
                commit_files_stored += 1;
            }
        }
    }
    Ok(commit_files_stored)
}

async fn store_files_and_symbols(
    pool: &PgPool,
    repo_id: Uuid,
    local_path: &std::path::Path,
    indexed: &[archaeologist_indexer::IndexedFile],
) -> anyhow::Result<(usize, usize, HashMap<String, Uuid>)> {
    use archaeologist_core::models::{FileCreate, SymbolCreate};
    use archaeologist_db::repositories::{create_file, create_symbol, get_file_by_path};
    use sha2::{Digest, Sha256};
    use tracing::warn;

    let mut files_stored: usize = 0;
    let mut symbols_stored: usize = 0;
    let mut file_id_map: HashMap<String, Uuid> = HashMap::new();

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

        if let Ok(Some(existing)) = get_file_by_path(pool, repo_id, &rel_path).await {
            if existing.content_hash == content_hash {
                file_id_map.insert(rel_path, existing.id);
                continue;
            }
        }

        let file_rec = match create_file(
            pool,
            &FileCreate {
                repository_id: repo_id,
                path: rel_path.clone(),
                language: Some(indexed_file.language.as_str().to_string()),
                size_bytes,
                content_hash,
            },
        )
        .await
        {
            Ok(r) => r,
            Err(e) => {
                warn!("DB error storing file {:?}: {e}", indexed_file.path);
                continue;
            }
        };
        files_stored += 1;
        file_id_map.insert(rel_path, file_rec.id);

        for sym in &indexed_file.symbols {
            let sym_create = SymbolCreate {
                file_id: file_rec.id,
                repository_id: repo_id,
                name: sym.name.clone(),
                symbol_type: indexer_kind_to_core(&sym.kind),
                language: indexed_file.language.as_str().to_string(),
                line_start: i32::try_from(sym.line_start).unwrap_or(0),
                line_end: i32::try_from(sym.line_end).unwrap_or(0),
                col_start: i32::try_from(sym.col_start).unwrap_or(0),
                col_end: i32::try_from(sym.col_end).unwrap_or(0),
                visibility: sym.visibility.clone(),
                doc_comment: sym.doc_comment.clone(),
                raw_text: sym.name.clone(),
            };
            match create_symbol(pool, &sym_create).await {
                Ok(_) => symbols_stored += 1,
                Err(e) => warn!("DB error storing symbol '{}': {e}", sym.name),
            }
        }
    }

    Ok((files_stored, symbols_stored, file_id_map))
}

async fn store_commits(
    pool: &PgPool,
    repo_id: Uuid,
    local_path: &std::path::Path,
) -> anyhow::Result<(usize, HashMap<String, Uuid>)> {
    use std::collections::HashMap;

    use archaeologist_core::models::CommitCreate;
    use archaeologist_db::repositories::{create_commit, get_commit_by_sha};
    use archaeologist_git::{walk_commits, WalkFilter};
    use tracing::warn;
    use uuid::Uuid;

    let commits = walk_commits(local_path, &WalkFilter::default()).unwrap_or_default();
    let mut commits_stored: usize = 0;
    let mut commit_id_map: HashMap<String, Uuid> = HashMap::new();
    for ci in &commits {
        match get_commit_by_sha(pool, repo_id, &ci.sha).await {
            Ok(Some(e)) => {
                commit_id_map.insert(ci.sha.clone(), e.id);
                continue;
            }
            Ok(None) => {}
            Err(e) => {
                warn!("DB error checking commit {}: {e}", &ci.sha[..8]);
                continue;
            }
        }
        let cc = CommitCreate {
            repository_id: repo_id,
            sha: ci.sha.clone(),
            author_name: Some(ci.author_name.clone()),
            author_email: Some(ci.author_email.clone()),
            author_date: ci.author_date,
            committer_name: Some(ci.committer_name.clone()),
            committer_email: Some(ci.committer_email.clone()),
            committer_date: ci.committer_date,
            message: ci.message.clone(),
            parent_shas: ci.parent_shas.clone(),
        };
        match create_commit(pool, &cc).await {
            Ok(c) => {
                commits_stored += 1;
                commit_id_map.insert(ci.sha.clone(), c.id);
            }
            Err(e) => warn!("DB error storing commit {}: {e}", &ci.sha[..8]),
        }
    }

    Ok((commits_stored, commit_id_map))
}

async fn store_symbol_commits(
    pool: &PgPool,
    local_path: &std::path::Path,
    commit_id_map: &HashMap<String, Uuid>,
    file_id_map: &HashMap<String, Uuid>,
) -> anyhow::Result<usize> {
    use archaeologist_core::models::SymbolCommitCreate;
    use archaeologist_db::repositories::upsert_symbol_commit;
    use archaeologist_git::diff_commit;

    let mut symbol_commits_stored: usize = 0;
    for (sha, &cid) in commit_id_map {
        let dfs = diff_commit(local_path, sha).unwrap_or_default();
        for df in &dfs {
            let file_db_id = match file_id_map.get(&df.file_path) {
                Some(id) => *id,
                None => continue,
            };
            let syms: Vec<(Uuid,)> = sqlx::query_as("SELECT id FROM symbols WHERE file_id = $1")
                .bind(file_db_id)
                .fetch_all(pool)
                .await
                .unwrap_or_default();
            let ct = match df.status {
                archaeologist_git::FileStatus::Added => "added",
                archaeologist_git::FileStatus::Deleted => "deleted",
                _ => "modified",
            };
            for (sid,) in syms {
                let sc = SymbolCommitCreate {
                    symbol_id: sid,
                    commit_id: cid,
                    change_type: ct.to_string(),
                };
                if upsert_symbol_commit(pool, &sc).await.is_ok() {
                    symbol_commits_stored += 1;
                }
            }
        }
    }
    Ok(symbol_commits_stored)
}

async fn store_refs(
    pool: &PgPool,
    repo_id: Uuid,
    local_path: &std::path::Path,
) -> anyhow::Result<(usize, usize)> {
    use archaeologist_core::models::{BranchCreate, TagCreate};
    use archaeologist_db::repositories::{upsert_branch, upsert_tag};
    use archaeologist_git::{list_branches, list_tags};

    let mut branches_stored: usize = 0;
    if let Ok(branches) = list_branches(local_path) {
        for b in &branches {
            let bc = BranchCreate {
                repository_id: repo_id,
                name: b.name.clone(),
                head_sha: b.head_sha.clone(),
                is_default: b.is_default,
            };
            if upsert_branch(pool, &bc).await.is_ok() {
                branches_stored += 1;
            }
        }
    }
    let mut tags_stored: usize = 0;
    if let Ok(tags) = list_tags(local_path) {
        for t in &tags {
            let tc = TagCreate {
                repository_id: repo_id,
                name: t.name.clone(),
                target_sha: t.target_sha.clone(),
            };
            if upsert_tag(pool, &tc).await.is_ok() {
                tags_stored += 1;
            }
        }
    }
    Ok((branches_stored, tags_stored))
}

async fn store_dependencies(
    pool: &PgPool,
    repo_id: Uuid,
    local_path: &std::path::Path,
    indexed: &[archaeologist_indexer::IndexedFile],
    file_id_map: &HashMap<String, Uuid>,
) -> anyhow::Result<usize> {
    use archaeologist_core::models::SymbolDependencyCreate;
    use archaeologist_db::repositories::create_symbol_dependency;

    let mut deps_stored: usize = 0;
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
        let syms: Vec<(Uuid, i32, i32)> = sqlx::query_as(
            "SELECT id, line_start, line_end FROM symbols \
             WHERE file_id = $1 ORDER BY line_start",
        )
        .bind(file_db_id)
        .fetch_all(pool)
        .await
        .unwrap_or_default();

        if syms.is_empty() {
            continue;
        }
        let fallback_sym = syms[0].0;
        let file_lang = indexed_file.language.as_str();

        for dep in &indexed_file.dependencies {
            let dep_line = i32::try_from(dep.line).unwrap_or(0);
            let sym_id = syms
                .iter()
                .filter(|(_, s, e)| *s <= dep_line && dep_line <= *e)
                .max_by_key(|(_, s, _)| *s)
                .map_or(fallback_sym, |(id, _, _)| *id);

            let depends_on = {
                let target = dep.target.trim();
                if !target.contains('.') && !target.contains('(') && !target.contains(' ') {
                    sqlx::query_as::<_, (Uuid,)>(
                        "SELECT id FROM symbols \
                         WHERE repository_id = $1 AND name = $2 AND language = $3 LIMIT 1",
                    )
                    .bind(repo_id)
                    .bind(target)
                    .bind(file_lang)
                    .fetch_optional(pool)
                    .await
                    .ok()
                    .flatten()
                    .map(|(id,)| id)
                } else {
                    None
                }
            };

            let dep_type = match dep.kind {
                archaeologist_indexer::DependencyKind::Import => "import",
                archaeologist_indexer::DependencyKind::Call => "call",
                archaeologist_indexer::DependencyKind::TraitImpl => "trait_impl",
            };
            let sd = SymbolDependencyCreate {
                symbol_id: sym_id,
                depends_on_symbol_id: depends_on,
                dependency_name: dep.target.clone(),
                dependency_type: dep_type.to_string(),
            };
            if create_symbol_dependency(pool, &sd).await.is_ok() {
                deps_stored += 1;
            }
        }
    }
    Ok(deps_stored)
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

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify the MCP server reports tool capabilities.
    #[test]
    fn server_info_has_tools_capability() {
        // We cannot instantiate a server without a real DB pool, but we can
        // verify the method signature compiles and that the tool router
        // is initialised by checking the tool count via the static router builder.
        // This test exercises the compilation of all tool definitions.
        let info = {
            // Create a dummy pool-less check by verifying the generated router
            // contains the expected tool names.
            let routes = ArchaeologistServer::tool_router();
            assert!(
                routes
                    .list_all()
                    .iter()
                    .any(|t| t.name == "index_repository"),
                "index_repository tool missing"
            );
            assert!(
                routes.list_all().iter().any(|t| t.name == "search_symbols"),
                "search_symbols tool missing"
            );
            assert!(
                routes.list_all().iter().any(|t| t.name == "explain_symbol"),
                "explain_symbol tool missing"
            );
            assert!(
                routes.list_all().iter().any(|t| t.name == "get_history"),
                "get_history tool missing"
            );
            assert!(
                routes.list_all().iter().any(|t| t.name == "analyze_impact"),
                "analyze_impact tool missing"
            );
            assert!(
                routes.list_all().iter().any(|t| t.name == "get_evidence"),
                "get_evidence tool missing"
            );
            assert!(
                routes.list_all().iter().any(|t| t.name == "search_code"),
                "search_code tool missing"
            );
            assert!(
                routes
                    .list_all()
                    .iter()
                    .any(|t| t.name == "list_repositories"),
                "list_repositories tool missing"
            );
            routes.list_all().len()
        };
        assert_eq!(info, 8, "expected exactly 8 MCP tools");
    }

    #[test]
    fn estimate_risk_no_callers() {
        assert_eq!(estimate_risk(0, 0), "LOW");
    }

    #[test]
    fn estimate_risk_critical() {
        assert_eq!(estimate_risk(100, 0), "CRITICAL");
    }

    #[test]
    fn estimate_risk_high_with_tests() {
        assert_eq!(estimate_risk(100, 5), "HIGH");
    }
}
