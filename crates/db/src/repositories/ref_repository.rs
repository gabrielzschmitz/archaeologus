#![allow(clippy::missing_errors_doc)]

use archaeologist_core::models::{Branch, BranchCreate, Tag, TagCreate};
use sqlx::PgPool;
use uuid::Uuid;

// ── Branches ─────────────────────────────────────────────────────────────────

pub async fn upsert_branch(pool: &PgPool, branch: &BranchCreate) -> Result<Branch, sqlx::Error> {
    let record = sqlx::query_as::<_, Branch>(
        r"
        INSERT INTO branches (repository_id, name, head_sha, is_default)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (repository_id, name) DO UPDATE
            SET head_sha = EXCLUDED.head_sha,
                is_default = EXCLUDED.is_default
        RETURNING id, repository_id, name, head_sha, is_default, created_at
        ",
    )
    .bind(branch.repository_id)
    .bind(&branch.name)
    .bind(&branch.head_sha)
    .bind(branch.is_default)
    .fetch_one(pool)
    .await?;
    Ok(record)
}

pub async fn list_branches(pool: &PgPool, repository_id: Uuid) -> Result<Vec<Branch>, sqlx::Error> {
    let records = sqlx::query_as::<_, Branch>(
        r"
        SELECT id, repository_id, name, head_sha, is_default, created_at
        FROM branches
        WHERE repository_id = $1
        ORDER BY name
        ",
    )
    .bind(repository_id)
    .fetch_all(pool)
    .await?;
    Ok(records)
}

// ── Tags ──────────────────────────────────────────────────────────────────────

pub async fn upsert_tag(pool: &PgPool, tag: &TagCreate) -> Result<Tag, sqlx::Error> {
    let record = sqlx::query_as::<_, Tag>(
        r"
        INSERT INTO tags (repository_id, name, target_sha)
        VALUES ($1, $2, $3)
        ON CONFLICT (repository_id, name) DO UPDATE
            SET target_sha = EXCLUDED.target_sha
        RETURNING id, repository_id, name, target_sha, created_at
        ",
    )
    .bind(tag.repository_id)
    .bind(&tag.name)
    .bind(&tag.target_sha)
    .fetch_one(pool)
    .await?;
    Ok(record)
}

pub async fn list_tags(pool: &PgPool, repository_id: Uuid) -> Result<Vec<Tag>, sqlx::Error> {
    let records = sqlx::query_as::<_, Tag>(
        r"
        SELECT id, repository_id, name, target_sha, created_at
        FROM tags
        WHERE repository_id = $1
        ORDER BY name
        ",
    )
    .bind(repository_id)
    .fetch_all(pool)
    .await?;
    Ok(records)
}
