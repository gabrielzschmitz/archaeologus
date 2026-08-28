use sqlx::PgPool;
use tracing::info;
use uuid::Uuid;

pub async fn search_code(
    pool: &PgPool,
    query: &str,
    repository_id: Option<Uuid>,
    language: Option<&str>,
) -> anyhow::Result<Vec<String>> {
    info!("Searching code: {}", query);
    Ok(vec![])
}
