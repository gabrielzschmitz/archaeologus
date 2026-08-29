use axum::{
    extract::{Query, State},
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use sqlx::PgPool;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use archaeologus_core::models::Evidence;
use archaeologus_db::repositories::evidence_repository;

use crate::{
    error::{ApiError, ApiResult},
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new().route("/evidence", get(get_evidence))
}

/// Query parameters for `GET /evidence`.
#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct EvidenceParams {
    /// Filter by symbol UUID.
    pub symbol_id: Option<Uuid>,
    /// Filter by repository UUID.
    pub repository_id: Option<Uuid>,
}

/// GET `/evidence?symbol_id=...` — retrieve evidence records.
#[derive(Debug, serde::Serialize, ToSchema)]
pub struct EvidenceResponse {
    pub items: Vec<Evidence>,
    pub total: usize,
}

/// GET /evidence — query evidence filtered by symbol or repository.
#[utoipa::path(
    get,
    path = "/evidence",
    params(EvidenceParams),
    responses(
        (status = 200, description = "Evidence records", body = EvidenceResponse),
        (status = 400, description = "No filter provided", body = crate::error::ErrorBody)
    ),
    tag = "evidence"
)]
async fn get_evidence(
    State(pool): State<PgPool>,
    Query(params): Query<EvidenceParams>,
) -> ApiResult<Json<EvidenceResponse>> {
    let items: Vec<Evidence> = match (params.symbol_id, params.repository_id) {
        (Some(symbol_id), _) => evidence_repository::get_evidence_for_symbol(&pool, symbol_id)
            .await
            .map_err(ApiError::from)?,
        (None, Some(repo_id)) => evidence_repository::get_evidence_for_repository(&pool, repo_id)
            .await
            .map_err(ApiError::from)?,
        (None, None) => {
            return Err(ApiError::BadRequest(
                "at least one of `symbol_id` or `repository_id` must be provided".into(),
            ));
        }
    };
    let total = items.len();
    Ok(Json(EvidenceResponse { items, total }))
}
