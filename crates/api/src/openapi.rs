use axum::Router;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::{
    error::ErrorBody,
    routes::{
        evidence::EvidenceResponse,
        health::HealthResponse,
        repositories::CreateRepositoryRequest,
        search::SearchResponse,
        symbols::{SymbolHistoryResponse, SymbolImpactResponse},
    },
    state::AppState,
};

use archaeologus_core::models::{
    Commit, CommitFile, Evidence, Repository, Symbol, SymbolCommit, SymbolDependency,
};

/// The top-level `OpenAPI` document for archaeologus-api.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "AI Software Archaeologus API",
        description = "REST API for the AI Software Archaeologus — answers 'why is the code like this?'",
        version = "0.1.0",
        license(name = "MIT"),
    ),
    paths(
        crate::routes::health::health_check,
        crate::routes::repositories::list_repositories,
        crate::routes::repositories::create_repository,
        crate::routes::repositories::get_repository,
        crate::routes::symbols::list_symbols_for_repo,
        crate::routes::symbols::get_symbol,
        crate::routes::symbols::get_symbol_history,
        crate::routes::symbols::get_symbol_impact,
        crate::routes::search::search,
        crate::routes::evidence::get_evidence,
    ),
    components(
        schemas(
            ErrorBody,
            HealthResponse,
            Repository,
            CreateRepositoryRequest,
            Symbol,
            SymbolHistoryResponse,
            SymbolImpactResponse,
            SymbolCommit,
            SymbolDependency,
            Commit,
            CommitFile,
            Evidence,
            SearchResponse,
            EvidenceResponse,
        )
    ),
    tags(
        (name = "health",       description = "Liveness probes"),
        (name = "repositories", description = "Repository management"),
        (name = "symbols",      description = "Symbol lookup, history, and impact"),
        (name = "search",       description = "Fuzzy symbol search"),
        (name = "evidence",     description = "Evidence records"),
    )
)]
pub struct ApiDoc;

/// Mount the Swagger UI at `/swagger-ui` and expose the raw spec at `/api-docs/openapi.json`.
pub fn swagger_router() -> Router<AppState> {
    SwaggerUi::new("/swagger-ui")
        .url("/api-docs/openapi.json", ApiDoc::openapi())
        .into()
}
