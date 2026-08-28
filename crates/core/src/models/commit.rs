use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Commit {
    pub id: Uuid,
    pub repository_id: Uuid,
    pub sha: String,
    pub author_name: Option<String>,
    pub author_email: Option<String>,
    pub author_date: DateTime<Utc>,
    pub committer_name: Option<String>,
    pub committer_email: Option<String>,
    pub committer_date: DateTime<Utc>,
    pub message: String,
    pub parent_shas: Vec<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitCreate {
    pub repository_id: Uuid,
    pub sha: String,
    pub author_name: Option<String>,
    pub author_email: Option<String>,
    pub author_date: DateTime<Utc>,
    pub committer_name: Option<String>,
    pub committer_email: Option<String>,
    pub committer_date: DateTime<Utc>,
    pub message: String,
    pub parent_shas: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CommitFile {
    pub id: Uuid,
    pub commit_id: Uuid,
    pub file_path: String,
    pub status: String,
    pub additions: i32,
    pub deletions: i32,
    pub old_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitFileCreate {
    pub commit_id: Uuid,
    pub file_path: String,
    pub status: String,
    pub additions: i32,
    pub deletions: i32,
    pub old_path: Option<String>,
}
