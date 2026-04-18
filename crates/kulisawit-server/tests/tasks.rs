#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use http_body_util::BodyExt;
use kulisawit_agent::MockAgent;
use kulisawit_core::{AgentAdapter, ColumnId, ProjectId};
use kulisawit_db::{columns, connect, migrate, project};
use kulisawit_orchestrator::{AgentRegistry, Orchestrator, RuntimeConfig};
use kulisawit_server::{routes_for_testing, AppState};
use tempfile::tempdir;
use tower::ServiceExt;

async fn app_with_project() -> (axum::Router, ProjectId, ColumnId) {
    let pool = connect("sqlite::memory:").await.expect("pool");
    migrate(&pool).await.expect("mig");
    let mut registry = AgentRegistry::new();
    registry.register(Arc::new(MockAgent::default()) as Arc<dyn AgentAdapter>);
    let dir = tempdir().expect("tmp");

    let project_id = project::create(
        &pool,
        project::NewProject {
            name: "P".into(),
            repo_path: dir.path().display().to_string(),
        },
    )
    .await
    .expect("p");
    let cols = columns::seed_defaults(&pool, &project_id).await.expect("c");
    let column_id = cols[0].clone();

    let orch = Arc::new(Orchestrator::new(
        pool,
        registry,
        dir.path().to_path_buf(),
        dir.path().join("wt"),
        RuntimeConfig::default(),
    ));
    std::mem::forget(dir);
    (routes_for_testing(AppState { orch }), project_id, column_id)
}

#[tokio::test]
async fn post_tasks_with_valid_body_returns_200() {
    let (app, project_id, column_id) = app_with_project().await;
    let body = serde_json::json!({
        "project_id": project_id.as_str(),
        "column_id": column_id.as_str(),
        "title": "my task"
    })
    .to_string();
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/tasks")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn post_tasks_with_bogus_project_id_returns_400() {
    let (app, _, column_id) = app_with_project().await;
    let body = serde_json::json!({
        "project_id": "does-not-exist",
        "column_id": column_id.as_str(),
        "title": "my task"
    })
    .to_string();
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/tasks")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let body = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(body.contains("project not found"), "got: {body}");
}

#[tokio::test]
async fn get_task_by_unknown_id_returns_404() {
    let (app, _, _) = app_with_project().await;
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/tasks/01900000-0000-0000-0000-000000000000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
