use sqlx::PgPool;
use tracing::info;
use uuid::Uuid;

pub async fn search_code(
    _pool: &PgPool,
    query: &str,
    _repository_id: Option<Uuid>,
    _language: Option<&str>,
) -> anyhow::Result<Vec<String>> {
    info!("Searching code: {}", query);
    Ok(vec![])
}
