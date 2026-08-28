use archaeologist_core::models::{File, FileCreate};
use sqlx::PgPool;
use uuid::Uuid;

pub async fn create_file(pool: &PgPool, file: &FileCreate) -> Result<File, sqlx::Error> {
    let record = sqlx::query_as::<_, File>(
        r#"
        INSERT INTO files (repository_id, path, language, size_bytes, content_hash)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, repository_id, path, language, size_bytes, content_hash, indexed_at
        "#,
    )
    .bind(file.repository_id)
    .bind(&file.path)
    .bind(&file.language)
    .bind(file.size_bytes)
    .bind(&file.content_hash)
    .fetch_one(pool)
    .await?;
    Ok(record)
}

pub async fn get_file(pool: &PgPool, id: Uuid) -> Result<Option<File>, sqlx::Error> {
    let record = sqlx::query_as::<_, File>(
        r#"
        SELECT id, repository_id, path, language, size_bytes, content_hash, indexed_at
        FROM files
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(record)
}

pub async fn get_file_by_path(
    pool: &PgPool,
    repository_id: Uuid,
    path: &str,
) -> Result<Option<File>, sqlx::Error> {
    let record = sqlx::query_as::<_, File>(
        r#"
        SELECT id, repository_id, path, language, size_bytes, content_hash, indexed_at
        FROM files
        WHERE repository_id = $1 AND path = $2
        "#,
    )
    .bind(repository_id)
    .bind(path)
    .fetch_optional(pool)
    .await?;
    Ok(record)
}

pub async fn list_files(pool: &PgPool, repository_id: Uuid) -> Result<Vec<File>, sqlx::Error> {
    let records = sqlx::query_as::<_, File>(
        r#"
        SELECT id, repository_id, path, language, size_bytes, content_hash, indexed_at
        FROM files
        WHERE repository_id = $1
        ORDER BY path
        "#,
    )
    .bind(repository_id)
    .fetch_all(pool)
    .await?;
    Ok(records)
}

pub async fn delete_file(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        DELETE FROM files
        WHERE id = $1
        "#,
    )
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}
