use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct File {
    pub id: Uuid,
    pub repository_id: Uuid,
    pub path: String,
    pub language: Option<String>,
    pub size_bytes: i64,
    pub content_hash: String,
    pub indexed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileCreate {
    pub repository_id: Uuid,
    pub path: String,
    pub language: Option<String>,
    pub size_bytes: i64,
    pub content_hash: String,
}
