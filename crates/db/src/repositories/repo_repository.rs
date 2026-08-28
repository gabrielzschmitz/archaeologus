use archaeologist_core::models::{Repository, RepositoryCreate};
use sqlx::PgPool;
use uuid::Uuid;

pub async fn create_repository(
    pool: &PgPool,
    repo: &RepositoryCreate,
) -> Result<Repository, sqlx::Error> {
    let record = sqlx::query_as::<_, Repository>(
        r#"
        INSERT INTO repositories (name, url, local_path, description, default_branch)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, name, url, local_path, description, default_branch, indexed_at, created_at, updated_at
        "#,
    )
    .bind(&repo.name)
    .bind(&repo.url)
    .bind(&repo.local_path)
    .bind(&repo.description)
    .bind(repo.default_branch.as_deref().unwrap_or("main"))
    .fetch_one(pool)
    .await?;
    Ok(record)
}

pub async fn get_repository(pool: &PgPool, id: Uuid) -> Result<Option<Repository>, sqlx::Error> {
    let record = sqlx::query_as::<_, Repository>(
        r#"
        SELECT id, name, url, local_path, description, default_branch, indexed_at, created_at, updated_at
        FROM repositories
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(record)
}

pub async fn get_repository_by_url(
    pool: &PgPool,
    url: &str,
) -> Result<Option<Repository>, sqlx::Error> {
    let record = sqlx::query_as::<_, Repository>(
        r#"
        SELECT id, name, url, local_path, description, default_branch, indexed_at, created_at, updated_at
        FROM repositories
        WHERE url = $1
        "#,
    )
    .bind(url)
    .fetch_optional(pool)
    .await?;
    Ok(record)
}

pub async fn list_repositories(pool: &PgPool) -> Result<Vec<Repository>, sqlx::Error> {
    let records = sqlx::query_as::<_, Repository>(
        r#"
        SELECT id, name, url, local_path, description, default_branch, indexed_at, created_at, updated_at
        FROM repositories
        ORDER BY created_at DESC
        "#,
    )
    .fetch_all(pool)
    .await?;
    Ok(records)
}

pub async fn update_repository_indexed(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE repositories
        SET indexed_at = NOW(), updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete_repository(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        DELETE FROM repositories
        WHERE id = $1
        "#,
    )
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}
