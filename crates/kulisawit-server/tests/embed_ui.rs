#![cfg(feature = "embed-ui")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
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
async fn serve_returns_index_html_for_root_when_embed_feature_enabled() {
    let app = fresh_app().await;
    let resp = app
        .oneshot(
            Request::builder().method(Method::GET).uri("/").body(Body::empty()).unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let body = std::str::from_utf8(&bytes).unwrap();
    assert!(body.contains("<!doctype html"), "expected index.html body, got: {}", body);
}

#[tokio::test]
async fn serve_returns_index_html_for_unknown_route_spa_fallback() {
    let app = fresh_app().await;
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/some/spa/route")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let body = std::str::from_utf8(&bytes).unwrap();
    assert!(body.contains("<!doctype html"));
}
