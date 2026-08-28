#![allow(clippy::missing_errors_doc)]

use sqlx::PgPool;
use tracing::info;

pub async fn run_migrations(pool: &PgPool) -> anyhow::Result<()> {
    info!("Running database migrations...");
    sqlx::migrate!("../../migrations").run(pool).await?;
    info!("Migrations completed successfully");
    Ok(())
}
