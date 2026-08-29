//! `archaeologus-api` — Axum HTTP server with `OpenAPI` / Swagger UI.
//!
//! # Quick start
//! ```no_run
//! use archaeologus_api::serve;
//! use sqlx::PgPool;
//!
//! # async fn example(pool: PgPool) -> anyhow::Result<()> {
//! serve(pool, "0.0.0.0:3000").await
//! # }
//! ```

pub mod error;
pub mod openapi;
pub mod routes;
pub mod state;

use anyhow::Context;
use axum::Router;
use state::AppState;
use tokio::net::TcpListener;

/// Build and bind the Axum application, then serve it on `addr`.
///
/// # Errors
/// Returns an error if the TCP listener cannot bind or if Axum fails to serve.
pub async fn serve(pool: sqlx::PgPool, addr: &str) -> anyhow::Result<()> {
    let state = AppState::new(pool);
    let app = build_router(state);
    let listener = TcpListener::bind(addr)
        .await
        .with_context(|| format!("failed to bind to {addr}"))?;
    tracing::info!("API server listening on {addr}");
    axum::serve(listener, app)
        .await
        .context("axum server error")?;
    Ok(())
}

/// Construct the full [`Router`] (useful in tests).
pub fn build_router(state: AppState) -> Router {
    use tower_http::cors::CorsLayer;
    use tower_http::trace::TraceLayer;

    let api = routes::api_router();
    let swagger = openapi::swagger_router();

    Router::new()
        .merge(api)
        .merge(swagger)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
