#![allow(clippy::missing_errors_doc)]

use archaeologus_core::models::{Commit, CommitCreate, CommitFile, CommitFileCreate};
use sqlx::PgPool;
use uuid::Uuid;

pub async fn create_commit(pool: &PgPool, commit: &CommitCreate) -> Result<Commit, sqlx::Error> {
    let record = sqlx::query_as::<_, Commit>(
        r"
        INSERT INTO commits (repository_id, sha, author_name, author_email, author_date, committer_name, committer_email, committer_date, message, parent_shas)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        RETURNING id, repository_id, sha, author_name, author_email, author_date, committer_name, committer_email, committer_date, message, parent_shas, created_at
        ",
    )
    .bind(commit.repository_id)
    .bind(&commit.sha)
    .bind(&commit.author_name)
    .bind(&commit.author_email)
    .bind(commit.author_date)
    .bind(&commit.committer_name)
    .bind(&commit.committer_email)
    .bind(commit.committer_date)
    .bind(&commit.message)
    .bind(&commit.parent_shas)
    .fetch_one(pool)
    .await?;
    Ok(record)
}

pub async fn get_commit(pool: &PgPool, id: Uuid) -> Result<Option<Commit>, sqlx::Error> {
    let record = sqlx::query_as::<_, Commit>(
        r"
        SELECT id, repository_id, sha, author_name, author_email, author_date, committer_name, committer_email, committer_date, message, parent_shas, created_at
        FROM commits
        WHERE id = $1
        ",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(record)
}

pub async fn get_commit_by_sha(
    pool: &PgPool,
    repository_id: Uuid,
    sha: &str,
) -> Result<Option<Commit>, sqlx::Error> {
    let record = sqlx::query_as::<_, Commit>(
        r"
        SELECT id, repository_id, sha, author_name, author_email, author_date, committer_name, committer_email, committer_date, message, parent_shas, created_at
        FROM commits
        WHERE repository_id = $1 AND sha = $2
        ",
    )
    .bind(repository_id)
    .bind(sha)
    .fetch_optional(pool)
    .await?;
    Ok(record)
}

pub async fn list_commits(
    pool: &PgPool,
    repository_id: Uuid,
    limit: i64,
) -> Result<Vec<Commit>, sqlx::Error> {
    let records = sqlx::query_as::<_, Commit>(
        r"
        SELECT id, repository_id, sha, author_name, author_email, author_date, committer_name, committer_email, committer_date, message, parent_shas, created_at
        FROM commits
        WHERE repository_id = $1
        ORDER BY author_date DESC
        LIMIT $2
        ",
    )
    .bind(repository_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(records)
}

pub async fn create_commit_file(
    pool: &PgPool,
    file: &CommitFileCreate,
) -> Result<CommitFile, sqlx::Error> {
    let record = sqlx::query_as::<_, CommitFile>(
        r"
        INSERT INTO commit_files (commit_id, file_path, status, additions, deletions, old_path)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id, commit_id, file_path, status, additions, deletions, old_path
        ",
    )
    .bind(file.commit_id)
    .bind(&file.file_path)
    .bind(&file.status)
    .bind(file.additions)
    .bind(file.deletions)
    .bind(&file.old_path)
    .fetch_one(pool)
    .await?;
    Ok(record)
}

pub async fn get_commit_files(
    pool: &PgPool,
    commit_id: Uuid,
) -> Result<Vec<CommitFile>, sqlx::Error> {
    let records = sqlx::query_as::<_, CommitFile>(
        r"
        SELECT id, commit_id, file_path, status, additions, deletions, old_path
        FROM commit_files
        WHERE commit_id = $1
        ORDER BY file_path
        ",
    )
    .bind(commit_id)
    .fetch_all(pool)
    .await?;
    Ok(records)
}
