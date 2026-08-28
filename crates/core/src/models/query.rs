use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    pub query: String,
    pub repository_id: Option<Uuid>,
    pub symbol_type: Option<String>,
    pub language: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Explanation {
    pub symbol_name: String,
    pub purpose: String,
    pub origin: OriginInfo,
    pub history: Vec<CommitInfo>,
    pub dependencies: Vec<String>,
    pub dependents: Vec<String>,
    pub evidence: Vec<EvidenceInfo>,
    pub confidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OriginInfo {
    pub author: String,
    pub created_at: String,
    pub first_commit: String,
    pub commit_message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitInfo {
    pub sha: String,
    pub author: String,
    pub date: String,
    pub message: String,
    pub files_changed: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceInfo {
    pub evidence_type: String,
    pub content: String,
    pub source_ref: Option<String>,
    pub confidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactReport {
    pub symbol_name: String,
    pub direct_callers: Vec<String>,
    pub indirect_callers: Vec<String>,
    pub affected_tests: Vec<String>,
    pub risk_level: String,
    pub recommended_tests: Vec<String>,
}
