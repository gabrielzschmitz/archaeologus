use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use utoipa::ToSchema;

/// Unified API error type that renders as a JSON body.
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<sqlx::Error> for ApiError {
    fn from(e: sqlx::Error) -> Self {
        match e {
            sqlx::Error::RowNotFound => Self::NotFound("record not found".into()),
            other => Self::Internal(other.to_string()),
        }
    }
}

impl From<archaeologist_core::AppError> for ApiError {
    fn from(e: archaeologist_core::AppError) -> Self {
        match e {
            archaeologist_core::AppError::NotFound(msg) => Self::NotFound(msg),
            archaeologist_core::AppError::Validation(msg) => Self::BadRequest(msg),
            other => Self::Internal(other.to_string()),
        }
    }
}

/// JSON body returned on error.
#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorBody {
    pub error: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            Self::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            Self::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            Self::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
        };
        (status, Json(ErrorBody { error: message })).into_response()
    }
}

pub type ApiResult<T> = Result<T, ApiError>;
