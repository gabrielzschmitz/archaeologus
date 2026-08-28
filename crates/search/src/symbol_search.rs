use sqlx::PgPool;
use tracing::info;
use uuid::Uuid;

pub async fn search_symbols(
    pool: &PgPool,
    query: &str,
    repository_id: Option<Uuid>,
    symbol_type: Option<&str>,
    language: Option<&str>,
) -> anyhow::Result<Vec<String>> {
    info!("Searching symbols: {}", query);
    Ok(vec![])
}
