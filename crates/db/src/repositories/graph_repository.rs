#![allow(clippy::missing_errors_doc)]

use archaeologus_core::models::{
    SymbolCommit, SymbolCommitCreate, SymbolDependency, SymbolDependencyCreate,
};
use sqlx::PgPool;
use uuid::Uuid;

// ── Symbol ↔ Commit links ─────────────────────────────────────────────────────

pub async fn upsert_symbol_commit(
    pool: &PgPool,
    sc: &SymbolCommitCreate,
) -> Result<SymbolCommit, sqlx::Error> {
    let record = sqlx::query_as::<_, SymbolCommit>(
        r"
        INSERT INTO symbol_commits (symbol_id, commit_id, change_type)
        VALUES ($1, $2, $3)
        ON CONFLICT (symbol_id, commit_id) DO UPDATE
            SET change_type = EXCLUDED.change_type
        RETURNING id, symbol_id, commit_id, change_type
        ",
    )
    .bind(sc.symbol_id)
    .bind(sc.commit_id)
    .bind(&sc.change_type)
    .fetch_one(pool)
    .await?;
    Ok(record)
}

pub async fn list_symbol_commits(
    pool: &PgPool,
    symbol_id: Uuid,
) -> Result<Vec<SymbolCommit>, sqlx::Error> {
    let records = sqlx::query_as::<_, SymbolCommit>(
        r"
        SELECT id, symbol_id, commit_id, change_type
        FROM symbol_commits
        WHERE symbol_id = $1
        ",
    )
    .bind(symbol_id)
    .fetch_all(pool)
    .await?;
    Ok(records)
}

// ── Symbol dependency graph ───────────────────────────────────────────────────

pub async fn create_symbol_dependency(
    pool: &PgPool,
    dep: &SymbolDependencyCreate,
) -> Result<SymbolDependency, sqlx::Error> {
    let record = sqlx::query_as::<_, SymbolDependency>(
        r"
        INSERT INTO symbol_dependencies (symbol_id, depends_on_symbol_id, dependency_name, dependency_type)
        VALUES ($1, $2, $3, $4)
        RETURNING id, symbol_id, depends_on_symbol_id, dependency_name, dependency_type
        ",
    )
    .bind(dep.symbol_id)
    .bind(dep.depends_on_symbol_id)
    .bind(&dep.dependency_name)
    .bind(&dep.dependency_type)
    .fetch_one(pool)
    .await?;
    Ok(record)
}

pub async fn list_symbol_dependencies(
    pool: &PgPool,
    symbol_id: Uuid,
) -> Result<Vec<SymbolDependency>, sqlx::Error> {
    let records = sqlx::query_as::<_, SymbolDependency>(
        r"
        SELECT id, symbol_id, depends_on_symbol_id, dependency_name, dependency_type
        FROM symbol_dependencies
        WHERE symbol_id = $1
        ",
    )
    .bind(symbol_id)
    .fetch_all(pool)
    .await?;
    Ok(records)
}
