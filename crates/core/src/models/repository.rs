use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Repository {
    pub id: Uuid,
    pub name: String,
    pub url: String,
    pub local_path: Option<String>,
    pub description: Option<String>,
    pub default_branch: String,
    pub indexed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryCreate {
    pub name: String,
    pub url: String,
    pub local_path: Option<String>,
    pub description: Option<String>,
    pub default_branch: Option<String>,
}
