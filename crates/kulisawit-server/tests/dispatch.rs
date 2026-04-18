#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::process::Command;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use http_body_util::BodyExt;
use kulisawit_agent::MockAgent;
use kulisawit_core::{AgentAdapter, TaskId};
use kulisawit_db::{columns, connect, migrate, project, task};
use kulisawit_orchestrator::{AgentRegistry, Orchestrator, RuntimeConfig};
use kulisawit_server::{routes_for_testing, AppState};
use tempfile::tempdir;
use tower::ServiceExt;

fn init_repo(dir: &std::path::Path) {
    Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(dir)
        .status()
        .unwrap();
    std::fs::write(dir.join("README.md"), "# t\n").unwrap();
    Command::new("git")
        .args(["-c", "user.email=t@t", "-c", "user.name=t", "add", "."])
        .current_dir(dir)
        .status()
        .unwrap();
    Command::new("git")
        .args([
            "-c",
            "user.email=t@t",
            "-c",
            "user.name=t",
            "commit",
            "-m",
            "i",
        ])
        .current_dir(dir)
        .status()
        .unwrap();
}

async fn app_with_task() -> (axum::Router, TaskId) {
    let pool = connect("sqlite::memory:").await.expect("pool");
    migrate(&pool).await.expect("mig");
    let mut registry = AgentRegistry::new();
    registry.register(Arc::new(MockAgent::default()) as Arc<dyn AgentAdapter>);
    let dir = tempdir().expect("tmp");
    init_repo(dir.path());

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
    let task_id = task::create(
        &pool,
        task::NewTask {
            project_id,
            column_id: cols[0].clone(),
            title: "t".into(),
            description: None,
            tags: vec![],
            linked_files: vec![],
        },
    )
    .await
    .expect("t");

    let orch = Arc::new(Orchestrator::new(
        pool,
        registry,
        dir.path().to_path_buf(),
        dir.path().join("wt"),
        RuntimeConfig::default(),
    ));
    std::mem::forget(dir);
    (routes_for_testing(AppState { orch }), task_id)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn dispatch_unknown_task_returns_404() {
    let (app, _) = app_with_task().await;
    let body = r#"{"agent":"mock","batch":1}"#;
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/tasks/01900000-0000-0000-0000-000000000000/dispatch")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn dispatch_batch_zero_returns_400() {
    let (app, task_id) = app_with_task().await;
    let body = r#"{"agent":"mock","batch":0}"#;
    let uri = format!("/api/tasks/{}/dispatch", task_id.as_str());
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(&uri)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn dispatch_valid_returns_ids() {
    let (app, task_id) = app_with_task().await;
    let body = r#"{"agent":"mock","batch":2}"#;
    let uri = format!("/api/tasks/{}/dispatch", task_id.as_str());
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(&uri)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let ids = json["attempt_ids"].as_array().expect("ids array");
    assert_eq!(ids.len(), 2);
    for id in ids {
        assert!(id.is_string());
    }
}
