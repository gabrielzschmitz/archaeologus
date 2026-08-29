use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use sqlx::PgPool;
use utoipa::ToSchema;
use uuid::Uuid;

use archaeologus_core::models::{Repository, RepositoryCreate};
use archaeologus_db::repositories::repo_repository;

use crate::{
    error::{ApiError, ApiResult},
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/repositories",
            get(list_repositories).post(create_repository),
        )
        .route("/repositories/{id}", get(get_repository))
}

/// GET /repositories — list all repositories.
#[utoipa::path(
    get,
    path = "/repositories",
    responses(
        (status = 200, description = "List of repositories", body = Vec<Repository>)
    ),
    tag = "repositories"
)]
async fn list_repositories(State(pool): State<PgPool>) -> ApiResult<Json<Vec<Repository>>> {
    let repos = repo_repository::list_repositories(&pool)
        .await
        .map_err(ApiError::from)?;
    Ok(Json(repos))
}

/// POST /repositories — register a new repository.
#[utoipa::path(
    post,
    path = "/repositories",
    request_body = CreateRepositoryRequest,
    responses(
        (status = 201, description = "Repository created", body = Repository),
        (status = 400, description = "Bad request", body = crate::error::ErrorBody)
    ),
    tag = "repositories"
)]
async fn create_repository(
    State(pool): State<PgPool>,
    Json(body): Json<CreateRepositoryRequest>,
) -> ApiResult<(StatusCode, Json<Repository>)> {
    if body.name.trim().is_empty() {
        return Err(ApiError::BadRequest("name must not be empty".into()));
    }
    if body.url.trim().is_empty() {
        return Err(ApiError::BadRequest("url must not be empty".into()));
    }
    let create = RepositoryCreate {
        name: body.name,
        url: body.url,
        local_path: body.local_path,
        description: body.description,
        default_branch: body.default_branch,
    };
    let repo = repo_repository::create_repository(&pool, &create)
        .await
        .map_err(ApiError::from)?;
    Ok((StatusCode::CREATED, Json(repo)))
}

/// GET /repositories/:id — retrieve a single repository.
#[utoipa::path(
    get,
    path = "/repositories/{id}",
    params(("id" = Uuid, Path, description = "Repository UUID")),
    responses(
        (status = 200, description = "Repository found", body = Repository),
        (status = 404, description = "Not found", body = crate::error::ErrorBody)
    ),
    tag = "repositories"
)]
async fn get_repository(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Repository>> {
    let repo = repo_repository::get_repository(&pool, id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::NotFound(format!("repository {id} not found")))?;
    Ok(Json(repo))
}

// ── Request body ──────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateRepositoryRequest {
    pub name: String,
    pub url: String,
    pub local_path: Option<String>,
    pub description: Option<String>,
    pub default_branch: Option<String>,
}
