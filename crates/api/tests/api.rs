//! Integration tests for `archaeologus-api`.
//!
//! Uses `tower::ServiceExt` + `http-body-util` for in-process HTTP testing —
//! no separate test server process is needed.
//!
//! Tests that touch the DB are guarded by `skip_if_no_db!`.  They are silently
//! skipped when `DATABASE_URL` is not set or Postgres is unreachable.
//!
//! Run with:
//! ```text
//! DATABASE_URL=postgres://... cargo test -p archaeologus-api
//! ```

use archaeologus_api::{build_router, state::AppState};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use sqlx::PgPool;
use tower::ServiceExt;
use utoipa::OpenApi;
use uuid::Uuid;

// ── helpers ──────────────────────────────────────────────────────────────────

fn build_app(pool: PgPool) -> axum::Router {
    build_router(AppState::new(pool))
}

async fn try_pool() -> Option<PgPool> {
    let url = std::env::var("DATABASE_URL").ok()?;
    let pool = PgPool::connect(&url).await.ok()?;
    archaeologus_db::run_migrations(&pool).await.ok()?;
    Some(pool)
}

/// Parse a full response body as JSON.
async fn body_json(body: Body) -> serde_json::Value {
    let bytes = body.collect().await.expect("body collect").to_bytes();
    serde_json::from_slice(&bytes).expect("body is valid JSON")
}

/// Issue a GET request and return (status, body).
async fn get(app: &axum::Router, uri: &str) -> (StatusCode, serde_json::Value) {
    let req = Request::get(uri).body(Body::empty()).unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let json = body_json(resp.into_body()).await;
    (status, json)
}

/// Issue a POST request with a JSON body and return (status, body).
async fn post_json(
    app: &axum::Router,
    uri: &str,
    payload: serde_json::Value,
) -> (StatusCode, serde_json::Value) {
    let req = Request::post(uri)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&payload).unwrap()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let json = body_json(resp.into_body()).await;
    (status, json)
}

macro_rules! skip_if_no_db {
    ($pool:ident) => {
        let Some($pool) = try_pool().await else {
            eprintln!("Skipping test: no DATABASE_URL / db not reachable");
            return;
        };
    };
}

// ── Health ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn health_returns_ok() {
    let pool = PgPool::connect_lazy("postgres://localhost/non_existent_db_for_test")
        .expect("lazy pool creation failed");
    let app = build_app(pool);

    let (status, body) = get(&app, "/health").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
    assert!(body["version"].is_string());
}

// ── OpenAPI ───────────────────────────────────────────────────────────────────

#[test]
fn openapi_document_generates_without_panic() {
    let doc = archaeologus_api::openapi::ApiDoc::openapi();
    let json = serde_json::to_string(&doc).expect("spec must serialize");
    assert!(json.contains("AI Software Archaeologus API"));
}

#[tokio::test]
async fn openapi_spec_is_served() {
    let pool = PgPool::connect_lazy("postgres://localhost/non_existent_db_for_test")
        .expect("lazy pool creation failed");
    let app = build_app(pool);

    let (status, spec) = get(&app, "/api-docs/openapi.json").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(spec["info"]["title"], "AI Software Archaeologus API");
    assert!(spec["paths"].is_object());
}

#[tokio::test]
async fn swagger_ui_root_is_reachable() {
    let pool = PgPool::connect_lazy("postgres://localhost/non_existent_db_for_test")
        .expect("lazy pool creation failed");
    let app = build_app(pool);

    let req = Request::get("/swagger-ui/").body(Body::empty()).unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let status = resp.status();
    assert!(
        status.is_success() || status.is_redirection(),
        "unexpected Swagger UI status {status}"
    );
}

// ── Repositories ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn list_repositories_returns_array() {
    skip_if_no_db!(pool);
    let app = build_app(pool);

    let (status, body) = get(&app, "/repositories").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.is_array(), "expected JSON array, got: {body}");
}

#[tokio::test]
async fn create_and_get_repository() {
    skip_if_no_db!(pool);
    let app = build_app(pool);

    let unique = Uuid::new_v4();
    let payload = serde_json::json!({
        "name": format!("test-repo-{unique}"),
        "url":  format!("https://github.com/test/repo-{unique}"),
    });

    let (create_status, created) = post_json(&app, "/repositories", payload).await;
    assert_eq!(create_status, StatusCode::CREATED);
    let id = created["id"].as_str().expect("id must be present");

    let (get_status, fetched) = get(&app, &format!("/repositories/{id}")).await;
    assert_eq!(get_status, StatusCode::OK);
    assert_eq!(fetched["id"], created["id"]);
}

#[tokio::test]
async fn create_repository_empty_name_is_400() {
    skip_if_no_db!(pool);
    let app = build_app(pool);

    let payload = serde_json::json!({ "name": "", "url": "https://example.com" });
    let (status, body) = post_json(&app, "/repositories", payload).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].is_string());
}

#[tokio::test]
async fn get_unknown_repository_is_404() {
    skip_if_no_db!(pool);
    let app = build_app(pool);

    let (status, _) = get(&app, &format!("/repositories/{}", Uuid::new_v4())).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ── Symbols ───────────────────────────────────────────────────────────────────

#[tokio::test]
async fn get_unknown_symbol_is_404() {
    skip_if_no_db!(pool);
    let app = build_app(pool);

    let (status, _) = get(&app, &format!("/symbols/{}", Uuid::new_v4())).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn symbol_history_unknown_is_404() {
    skip_if_no_db!(pool);
    let app = build_app(pool);

    let (status, _) = get(&app, &format!("/symbols/{}/history", Uuid::new_v4())).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn symbol_impact_unknown_is_404() {
    skip_if_no_db!(pool);
    let app = build_app(pool);

    let (status, _) = get(&app, &format!("/symbols/{}/impact", Uuid::new_v4())).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn list_symbols_unknown_repo_is_404() {
    skip_if_no_db!(pool);
    let app = build_app(pool);

    let (status, _) = get(&app, &format!("/repositories/{}/symbols", Uuid::new_v4())).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ── Search ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn search_missing_q_returns_error() {
    skip_if_no_db!(pool);
    let app = build_app(pool);

    // Axum rejects a missing required query field with 400 (via rejection handler).
    let (status, _) = get(&app, "/search").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn search_returns_paginated_results() {
    skip_if_no_db!(pool);
    let app = build_app(pool);

    let (status, body) = get(&app, "/search?q=main&limit=5").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["items"].is_array());
    assert!(body["total"].is_number());
    assert_eq!(body["limit"], 5);
    assert_eq!(body["offset"], 0);
}

#[tokio::test]
async fn search_blank_q_is_400() {
    skip_if_no_db!(pool);
    let app = build_app(pool);

    // %20 = single space
    let (status, _) = get(&app, "/search?q=%20").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

// ── Evidence ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn evidence_no_filter_is_400() {
    skip_if_no_db!(pool);
    let app = build_app(pool);

    let (status, body) = get(&app, "/evidence").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].is_string());
}

#[tokio::test]
async fn evidence_unknown_symbol_returns_empty() {
    skip_if_no_db!(pool);
    let app = build_app(pool);

    let (status, body) = get(&app, &format!("/evidence?symbol_id={}", Uuid::new_v4())).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["items"].as_array().unwrap().len(), 0);
    assert_eq!(body["total"], 0);
}

#[tokio::test]
async fn evidence_unknown_repo_returns_empty() {
    skip_if_no_db!(pool);
    let app = build_app(pool);

    let (status, body) = get(&app, &format!("/evidence?repository_id={}", Uuid::new_v4())).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["items"].as_array().unwrap().len(), 0);
}
