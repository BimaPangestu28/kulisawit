#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use http_body_util::BodyExt;
use kulisawit_agent::MockAgent;
use kulisawit_core::AgentAdapter;
use kulisawit_db::{connect, migrate};
use kulisawit_orchestrator::{AgentRegistry, Orchestrator, RuntimeConfig};
use kulisawit_server::{routes_for_testing, AppState};
use tempfile::tempdir;
use tower::ServiceExt;

async fn fresh_app() -> axum::Router {
    let pool = connect("sqlite::memory:").await.expect("pool");
    migrate(&pool).await.expect("mig");
    let mut registry = AgentRegistry::new();
    registry.register(Arc::new(MockAgent::default()) as Arc<dyn AgentAdapter>);
    let dir = tempdir().expect("tmp");
    let orch = Arc::new(Orchestrator::new(
        pool,
        registry,
        dir.path().to_path_buf(),
        dir.path().join("wt"),
        RuntimeConfig::default(),
    ));
    std::mem::forget(dir);
    routes_for_testing(AppState { orch })
}

#[tokio::test]
async fn post_projects_with_valid_body_returns_200_and_inserts_with_seeded_columns() {
    let app = fresh_app().await;
    let body = r#"{"name":"Demo","repo_path":"/tmp/demo"}"#;
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/projects")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["name"], "Demo");
    assert_eq!(json["repo_path"], "/tmp/demo");
    assert!(json["id"].is_string());
    assert!(json["created_at"].is_i64());
    let column_ids = json["column_ids"].as_array().expect("column_ids array");
    assert_eq!(column_ids.len(), 5, "expected 5 default columns auto-seeded");
    for col in column_ids {
        assert!(col.is_string(), "column id should be string");
    }
}

#[tokio::test]
async fn post_projects_with_missing_name_returns_400() {
    let app = fresh_app().await;
    let body = r#"{"repo_path":"/tmp/demo"}"#;
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/projects")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(
        resp.status().is_client_error(),
        "expected 4xx, got {}",
        resp.status()
    );
}

#[tokio::test]
async fn get_project_by_unknown_id_returns_404() {
    let app = fresh_app().await;
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/projects/01900000-0000-0000-0000-000000000000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn get_projects_returns_empty_array_when_none() {
    let app = fresh_app().await;
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/projects")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(json.is_array(), "expected JSON array");
    assert_eq!(json.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn get_projects_returns_inserted_projects_in_creation_order() {
    let app = fresh_app().await;
    // Insert two projects via the create endpoint
    for name in ["one", "two"] {
        let body = format!(r#"{{"name":"{}","repo_path":"/tmp/{}"}}"#, name, name);
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/projects")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/projects")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let arr = json.as_array().expect("array");
    assert_eq!(arr.len(), 2);
    // project::list returns ORDER BY created_at DESC, so most-recent first
    assert_eq!(arr[0]["name"], "two");
    assert_eq!(arr[1]["name"], "one");
    // Listing returns column_ids as empty Vec per spec contract
    assert_eq!(arr[0]["column_ids"].as_array().unwrap().len(), 0);
    assert_eq!(arr[1]["column_ids"].as_array().unwrap().len(), 0);
}
