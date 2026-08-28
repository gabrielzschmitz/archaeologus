use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Branch {
    pub id: Uuid,
    pub repository_id: Uuid,
    pub name: String,
    pub head_sha: String,
    pub is_default: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchCreate {
    pub repository_id: Uuid,
    pub name: String,
    pub head_sha: String,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Tag {
    pub id: Uuid,
    pub repository_id: Uuid,
    pub name: String,
    pub target_sha: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagCreate {
    pub repository_id: Uuid,
    pub name: String,
    pub target_sha: String,
}
