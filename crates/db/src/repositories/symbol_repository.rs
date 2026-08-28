use archaeologist_core::models::{Symbol, SymbolCreate};
use sqlx::PgPool;
use uuid::Uuid;

pub async fn create_symbol(pool: &PgPool, symbol: &SymbolCreate) -> Result<Symbol, sqlx::Error> {
    let record = sqlx::query_as::<_, Symbol>(
        r#"
        INSERT INTO symbols (file_id, repository_id, name, symbol_type, language, line_start, line_end, col_start, col_end, visibility, doc_comment, raw_text)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        RETURNING id, file_id, repository_id, name, symbol_type, language, line_start, line_end, col_start, col_end, visibility, doc_comment, raw_text, created_at
        "#,
    )
    .bind(symbol.file_id)
    .bind(symbol.repository_id)
    .bind(&symbol.name)
    .bind(symbol.symbol_type.to_string())
    .bind(&symbol.language)
    .bind(symbol.line_start)
    .bind(symbol.line_end)
    .bind(symbol.col_start)
    .bind(symbol.col_end)
    .bind(&symbol.visibility)
    .bind(&symbol.doc_comment)
    .bind(&symbol.raw_text)
    .fetch_one(pool)
    .await?;
    Ok(record)
}

pub async fn get_symbol(pool: &PgPool, id: Uuid) -> Result<Option<Symbol>, sqlx::Error> {
    let record = sqlx::query_as::<_, Symbol>(
        r#"
        SELECT id, file_id, repository_id, name, symbol_type, language, line_start, line_end, col_start, col_end, visibility, doc_comment, raw_text, created_at
        FROM symbols
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(record)
}

pub async fn search_symbols(
    pool: &PgPool,
    query: &str,
    repository_id: Option<Uuid>,
    symbol_type: Option<&str>,
    language: Option<&str>,
) -> Result<Vec<Symbol>, sqlx::Error> {
    let records = sqlx::query_as::<_, Symbol>(
        r#"
        SELECT id, file_id, repository_id, name, symbol_type, language, line_start, line_end, col_start, col_end, visibility, doc_comment, raw_text, created_at
        FROM symbols
        WHERE name ILIKE '%' || $1 || '%'
        AND ($2::UUID IS NULL OR repository_id = $2)
        AND ($3::TEXT IS NULL OR symbol_type = $3)
        AND ($4::TEXT IS NULL OR language = $4)
        ORDER BY name
        LIMIT 100
        "#,
    )
    .bind(query)
    .bind(repository_id)
    .bind(symbol_type)
    .bind(language)
    .fetch_all(pool)
    .await?;
    Ok(records)
}

pub async fn list_symbols(pool: &PgPool, repository_id: Uuid) -> Result<Vec<Symbol>, sqlx::Error> {
    let records = sqlx::query_as::<_, Symbol>(
        r#"
        SELECT id, file_id, repository_id, name, symbol_type, language, line_start, line_end, col_start, col_end, visibility, doc_comment, raw_text, created_at
        FROM symbols
        WHERE repository_id = $1
        ORDER BY name
        "#,
    )
    .bind(repository_id)
    .fetch_all(pool)
    .await?;
    Ok(records)
}

pub async fn delete_symbol(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        DELETE FROM symbols
        WHERE id = $1
        "#,
    )
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}
