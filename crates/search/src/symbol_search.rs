use sqlx::PgPool;
use tracing::info;
use uuid::Uuid;

pub async fn search_symbols(
    _pool: &PgPool,
    query: &str,
    _repository_id: Option<Uuid>,
    _symbol_type: Option<&str>,
    _language: Option<&str>,
) -> anyhow::Result<Vec<String>> {
    info!("Searching symbols: {}", query);
    Ok(vec![])
}
