use axum::{
    extract::{rejection::QueryRejection, Query, State},
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use sqlx::PgPool;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use archaeologist_core::models::Symbol;
use archaeologist_search::symbol_search::{SymbolQuery, SymbolSearchResult as SearchResult};

use crate::{
    error::{ApiError, ApiResult},
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new().route("/search", get(search))
}

/// Query parameters for `GET /search`.
#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct SearchParams {
    /// Search term (fuzzy-matched against symbol names).
    pub q: String,
    /// Restrict to a single repository.
    pub repository_id: Option<Uuid>,
    /// Filter by symbol type (e.g. `function`, `class`).
    pub symbol_type: Option<String>,
    /// Filter by language (e.g. `rust`, `python`).
    pub language: Option<String>,
    /// Max results (default 20, max 200).
    #[serde(default = "default_limit")]
    pub limit: i64,
    /// Pagination offset (default 0).
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    20
}

/// Paginated search response.
#[derive(Debug, serde::Serialize, ToSchema)]
pub struct SearchResponse {
    pub items: Vec<Symbol>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

impl From<SearchResult> for SearchResponse {
    fn from(r: SearchResult) -> Self {
        Self {
            items: r.items,
            total: r.total,
            limit: r.limit,
            offset: r.offset,
        }
    }
}

/// GET /search?q=... — fuzzy symbol search.
#[utoipa::path(
    get,
    path = "/search",
    params(SearchParams),
    responses(
        (status = 200, description = "Search results", body = SearchResponse),
        (status = 400, description = "Missing query parameter", body = crate::error::ErrorBody)
    ),
    tag = "search"
)]
async fn search(
    State(pool): State<PgPool>,
    params: Result<Query<SearchParams>, QueryRejection>,
) -> ApiResult<Json<SearchResponse>> {
    let Query(params) =
        params.map_err(|_| ApiError::BadRequest("invalid or missing query parameters".into()))?;

    if params.q.trim().is_empty() {
        return Err(ApiError::BadRequest(
            "query parameter `q` must not be empty".into(),
        ));
    }

    let q = SymbolQuery::new(&params.q)
        .limit(params.limit)
        .offset(params.offset);

    let q = if let Some(repo_id) = params.repository_id {
        q.repo(repo_id)
    } else {
        q
    };
    let q = if let Some(ref st) = params.symbol_type {
        q.symbol_type(st.as_str())
    } else {
        q
    };
    let q = if let Some(ref lang) = params.language {
        q.language(lang.as_str())
    } else {
        q
    };

    let result = archaeologist_search::symbol_search::search_symbols(&pool, &q)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(SearchResponse::from(result)))
}
