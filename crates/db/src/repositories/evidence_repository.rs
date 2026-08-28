use archaeologist_core::models::{Evidence, EvidenceCreate};
use sqlx::PgPool;
use uuid::Uuid;

pub async fn create_evidence(
    pool: &PgPool,
    evidence: &EvidenceCreate,
) -> Result<Evidence, sqlx::Error> {
    let record = sqlx::query_as::<_, Evidence>(
        r#"
        INSERT INTO evidence (repository_id, evidence_type, source_ref, content, confidence, symbol_id, commit_id, metadata)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING id, repository_id, evidence_type, source_ref, content, confidence, symbol_id, commit_id, metadata, created_at
        "#,
    )
    .bind(evidence.repository_id)
    .bind(evidence.evidence_type.to_string())
    .bind(&evidence.source_ref)
    .bind(&evidence.content)
    .bind(evidence.confidence.to_string())
    .bind(evidence.symbol_id)
    .bind(evidence.commit_id)
    .bind(&evidence.metadata)
    .fetch_one(pool)
    .await?;
    Ok(record)
}

pub async fn get_evidence_for_symbol(
    pool: &PgPool,
    symbol_id: Uuid,
) -> Result<Vec<Evidence>, sqlx::Error> {
    let records = sqlx::query_as::<_, Evidence>(
        r#"
        SELECT id, repository_id, evidence_type, source_ref, content, confidence, symbol_id, commit_id, metadata, created_at
        FROM evidence
        WHERE symbol_id = $1
        ORDER BY created_at DESC
        "#,
    )
    .bind(symbol_id)
    .fetch_all(pool)
    .await?;
    Ok(records)
}

pub async fn get_evidence_for_repository(
    pool: &PgPool,
    repository_id: Uuid,
) -> Result<Vec<Evidence>, sqlx::Error> {
    let records = sqlx::query_as::<_, Evidence>(
        r#"
        SELECT id, repository_id, evidence_type, source_ref, content, confidence, symbol_id, commit_id, metadata, created_at
        FROM evidence
        WHERE repository_id = $1
        ORDER BY created_at DESC
        "#,
    )
    .bind(repository_id)
    .fetch_all(pool)
    .await?;
    Ok(records)
}

pub async fn delete_evidence(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        DELETE FROM evidence
        WHERE id = $1
        "#,
    )
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}
