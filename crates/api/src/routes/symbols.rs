use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use sqlx::PgPool;
use uuid::Uuid;

use archaeologus_core::models::{Commit, Symbol, SymbolCommit, SymbolDependency};
use archaeologus_db::repositories::{commit_repository, graph_repository, symbol_repository};

use crate::{
    error::{ApiError, ApiResult},
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/repositories/{repo_id}/symbols",
            get(list_symbols_for_repo),
        )
        .route("/symbols/{id}", get(get_symbol))
        .route("/symbols/{id}/history", get(get_symbol_history))
        .route("/symbols/{id}/impact", get(get_symbol_impact))
}

// ── GET /repositories/:repo_id/symbols ───────────────────────────────────────

/// GET `/repositories/:repo_id/symbols` — list all symbols in a repository.
#[utoipa::path(
    get,
    path = "/repositories/{repo_id}/symbols",
    params(("repo_id" = Uuid, Path, description = "Repository UUID")),
    responses(
        (status = 200, description = "List of symbols", body = Vec<Symbol>),
        (status = 404, description = "Repository not found", body = crate::error::ErrorBody)
    ),
    tag = "symbols"
)]
async fn list_symbols_for_repo(
    State(pool): State<PgPool>,
    Path(repo_id): Path<Uuid>,
) -> ApiResult<Json<Vec<Symbol>>> {
    // Verify repository exists
    archaeologus_db::repositories::repo_repository::get_repository(&pool, repo_id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::NotFound(format!("repository {repo_id} not found")))?;

    let symbols = symbol_repository::list_symbols(&pool, repo_id)
        .await
        .map_err(ApiError::from)?;
    Ok(Json(symbols))
}

// ── GET /symbols/:id ─────────────────────────────────────────────────────────

/// GET /symbols/:id — retrieve a single symbol.
#[utoipa::path(
    get,
    path = "/symbols/{id}",
    params(("id" = Uuid, Path, description = "Symbol UUID")),
    responses(
        (status = 200, description = "Symbol found", body = Symbol),
        (status = 404, description = "Not found", body = crate::error::ErrorBody)
    ),
    tag = "symbols"
)]
async fn get_symbol(State(pool): State<PgPool>, Path(id): Path<Uuid>) -> ApiResult<Json<Symbol>> {
    let symbol = symbol_repository::get_symbol(&pool, id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::NotFound(format!("symbol {id} not found")))?;
    Ok(Json(symbol))
}

// ── GET /symbols/:id/history ─────────────────────────────────────────────────

/// Commit history for a symbol.
#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct SymbolHistoryResponse {
    pub symbol_id: Uuid,
    pub commits: Vec<Commit>,
    pub symbol_commits: Vec<SymbolCommit>,
}

/// GET /symbols/:id/history — commit history for a symbol.
#[utoipa::path(
    get,
    path = "/symbols/{id}/history",
    params(("id" = Uuid, Path, description = "Symbol UUID")),
    responses(
        (status = 200, description = "Commit history", body = SymbolHistoryResponse),
        (status = 404, description = "Symbol not found", body = crate::error::ErrorBody)
    ),
    tag = "symbols"
)]
async fn get_symbol_history(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<SymbolHistoryResponse>> {
    // Ensure symbol exists
    symbol_repository::get_symbol(&pool, id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::NotFound(format!("symbol {id} not found")))?;

    let symbol_commits = graph_repository::list_symbol_commits(&pool, id)
        .await
        .map_err(ApiError::from)?;

    // Fetch full commit records for each symbol_commit entry
    let mut commits = Vec::with_capacity(symbol_commits.len());
    for sc in &symbol_commits {
        if let Some(c) = commit_repository::get_commit(&pool, sc.commit_id)
            .await
            .map_err(ApiError::from)?
        {
            commits.push(c);
        }
    }

    Ok(Json(SymbolHistoryResponse {
        symbol_id: id,
        commits,
        symbol_commits,
    }))
}

// ── GET /symbols/:id/impact ──────────────────────────────────────────────────

/// Impact analysis for a symbol.
#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct SymbolImpactResponse {
    pub symbol_id: Uuid,
    /// Symbols this symbol depends on.
    pub dependencies: Vec<SymbolDependency>,
    /// Symbols that depend on this symbol (reverse edges).
    pub dependents: Vec<SymbolDependency>,
}

/// GET /symbols/:id/impact — impact analysis for a symbol.
#[utoipa::path(
    get,
    path = "/symbols/{id}/impact",
    params(("id" = Uuid, Path, description = "Symbol UUID")),
    responses(
        (status = 200, description = "Impact analysis", body = SymbolImpactResponse),
        (status = 404, description = "Symbol not found", body = crate::error::ErrorBody)
    ),
    tag = "symbols"
)]
async fn get_symbol_impact(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<SymbolImpactResponse>> {
    symbol_repository::get_symbol(&pool, id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::NotFound(format!("symbol {id} not found")))?;

    let dependencies = graph_repository::list_symbol_dependencies(&pool, id)
        .await
        .map_err(ApiError::from)?;

    // Reverse: find all deps where depends_on_symbol_id = id
    let dependents = sqlx::query_as::<_, SymbolDependency>(
        "SELECT id, symbol_id, depends_on_symbol_id, dependency_name, dependency_type
         FROM symbol_dependencies
         WHERE depends_on_symbol_id = $1",
    )
    .bind(id)
    .fetch_all(&pool)
    .await
    .map_err(ApiError::from)?;

    Ok(Json(SymbolImpactResponse {
        symbol_id: id,
        dependencies,
        dependents,
    }))
}
