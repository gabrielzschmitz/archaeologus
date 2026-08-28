#![allow(clippy::missing_errors_doc)]

use sqlx::postgres::{PgPool, PgPoolOptions};
use tracing::info;

pub async fn create_pool(database_url: &str) -> anyhow::Result<PgPool> {
    info!("Connecting to database...");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await?;
    info!("Database connected");
    Ok(pool)
}

pub async fn run_migrations(pool: &PgPool) -> anyhow::Result<()> {
    info!("Running migrations...");
    sqlx::migrate!("../../migrations").run(pool).await?;
    info!("Migrations complete");
    Ok(())
}
