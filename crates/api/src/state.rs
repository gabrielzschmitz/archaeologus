use axum::extract::FromRef;
use sqlx::PgPool;

/// Shared application state threaded through every handler.
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
}

impl AppState {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl FromRef<AppState> for PgPool {
    fn from_ref(state: &AppState) -> PgPool {
        state.pool.clone()
    }
}
