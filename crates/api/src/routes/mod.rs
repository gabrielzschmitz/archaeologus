use axum::Router;
use crate::state::AppState;

pub mod evidence;
pub mod health;
pub mod repositories;
pub mod search;
pub mod symbols;

/// Build the versioned API router.
pub fn api_router() -> Router<AppState> {
    Router::new()
        .merge(health::router())
        .merge(repositories::router())
        .merge(symbols::router())
        .merge(search::router())
        .merge(evidence::router())
}
