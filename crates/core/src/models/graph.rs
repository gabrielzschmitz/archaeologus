use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct SymbolCommit {
    pub id: Uuid,
    pub symbol_id: Uuid,
    pub commit_id: Uuid,
    pub change_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct SymbolCommitCreate {
    pub symbol_id: Uuid,
    pub commit_id: Uuid,
    /// e.g. "added", "modified", "deleted"
    pub change_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct SymbolDependency {
    pub id: Uuid,
    pub symbol_id: Uuid,
    pub depends_on_symbol_id: Option<Uuid>,
    pub dependency_name: String,
    pub dependency_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct SymbolDependencyCreate {
    pub symbol_id: Uuid,
    pub depends_on_symbol_id: Option<Uuid>,
    pub dependency_name: String,
    /// e.g. "import", "call", "`trait_impl`"
    pub dependency_type: String,
}
