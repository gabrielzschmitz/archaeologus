use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceType {
    Commit,
    Blame,
    Diff,
    Dependency,
    DocComment,
    Test,
    AiAnalysis,
}

impl std::fmt::Display for EvidenceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Commit => write!(f, "commit"),
            Self::Blame => write!(f, "blame"),
            Self::Diff => write!(f, "diff"),
            Self::Dependency => write!(f, "dependency"),
            Self::DocComment => write!(f, "doc_comment"),
            Self::Test => write!(f, "test"),
            Self::AiAnalysis => write!(f, "ai_analysis"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    High,
    Medium,
    Low,
    Unknown,
}

impl std::fmt::Display for Confidence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::High => write!(f, "high"),
            Self::Medium => write!(f, "medium"),
            Self::Low => write!(f, "low"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Evidence {
    pub id: Uuid,
    pub repository_id: Uuid,
    pub evidence_type: String,
    pub source_ref: Option<String>,
    pub content: String,
    pub confidence: String,
    pub symbol_id: Option<Uuid>,
    pub commit_id: Option<Uuid>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceCreate {
    pub repository_id: Uuid,
    pub evidence_type: EvidenceType,
    pub source_ref: Option<String>,
    pub content: String,
    pub confidence: Confidence,
    pub symbol_id: Option<Uuid>,
    pub commit_id: Option<Uuid>,
    pub metadata: Option<serde_json::Value>,
}
