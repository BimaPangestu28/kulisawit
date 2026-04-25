#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::process::Command;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use http_body_util::BodyExt;
use kulisawit_agent::MockAgent;
use kulisawit_core::{AgentAdapter, ColumnId, ProjectId};
use kulisawit_db::{
    columns, connect, migrate, project,
    project::NewProject,
    task::{self, NewTask},
};
use kulisawit_orchestrator::{AgentRegistry, Orchestrator, RuntimeConfig};
use kulisawit_server::{routes_for_testing, AppState};
use tempfile::tempdir;
use tower::ServiceExt;

fn init_git_repo(dir: &std::path::Path) {
    Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(dir)
        .status()
        .unwrap();
    std::fs::write(dir.join("README.md"), "# test\n").unwrap();
    Command::new("git")
        .args(["-c", "user.email=t@t", "-c", "user.name=t", "add", "."])
        .current_dir(dir)
        .status()
        .unwrap();
    Command::new("git")
        .args([
            "-c", "user.email=t@t", "-c", "user.name=t",
            "commit", "-m", "init",
        ])
        .current_dir(dir)
        .status()
        .unwrap();
}

async fn fresh_app_with_pool() -> (axum::Router, kulisawit_db::DbPool) {
    let pool = connect("sqlite::memory:").await.expect("pool");
    migrate(&pool).await.expect("mig");
    let mut registry = AgentRegistry::new();
    registry.register(Arc::new(MockAgent::default()) as Arc<dyn AgentAdapter>);
    let dir = tempdir().expect("tmp");
    init_git_repo(dir.path());
    let orch = Arc::new(Orchestrator::new(
        pool.clone(),
        registry,
        dir.path().to_path_buf(),
        dir.path().join("wt"),
        RuntimeConfig::default(),
    ));
    std::mem::forget(dir);
    (routes_for_testing(AppState { orch }), pool)
}

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

#[tokio::test]
async fn patch_task_with_title_only_updates_title() {
    let (app, pool) = fresh_app_with_pool().await;
    let project_id = project::create(&pool, NewProject {
        name: "p".into(), repo_path: "/tmp/p".into(),
    }).await.expect("project");
    let cols = columns::seed_defaults(&pool, &project_id).await.expect("cols");
    let task_id = task::create(&pool, NewTask {
        project_id: project_id.clone(),
        column_id: cols[0].clone(),
        title: "old".into(),
        description: Some("desc".into()),
        tags: vec![],
        linked_files: vec![],
    }).await.expect("task");

    let body = serde_json::json!({"title": "new"}).to_string();
    let uri = format!("/api/tasks/{}", task_id.as_str());
    let resp = app.oneshot(
        Request::builder()
            .method(Method::PATCH).uri(uri)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body)).unwrap(),
    ).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["title"], "new");
    assert_eq!(json["description"], "desc"); // preserved
}

#[tokio::test]
async fn patch_task_with_column_only_moves_task() {
    let (app, pool) = fresh_app_with_pool().await;
    let project_id = project::create(&pool, NewProject {
        name: "p".into(), repo_path: "/tmp/p".into(),
    }).await.expect("project");
    let cols = columns::seed_defaults(&pool, &project_id).await.expect("cols");
    let task_id = task::create(&pool, NewTask {
        project_id: project_id.clone(),
        column_id: cols[0].clone(),
        title: "t".into(), description: None,
        tags: vec![], linked_files: vec![],
    }).await.expect("task");

    let body = serde_json::json!({"column_id": cols[2].as_str()}).to_string();
    let uri = format!("/api/tasks/{}", task_id.as_str());
    let resp = app.oneshot(
        Request::builder().method(Method::PATCH).uri(uri)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body)).unwrap(),
    ).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["column_id"], cols[2].as_str());
}

#[tokio::test]
async fn patch_task_with_combined_fields_updates_both() {
    let (app, pool) = fresh_app_with_pool().await;
    let project_id = project::create(&pool, NewProject {
        name: "p".into(), repo_path: "/tmp/p".into(),
    }).await.expect("project");
    let cols = columns::seed_defaults(&pool, &project_id).await.expect("cols");
    let task_id = task::create(&pool, NewTask {
        project_id: project_id.clone(),
        column_id: cols[0].clone(),
        title: "old".into(), description: None,
        tags: vec![], linked_files: vec![],
    }).await.expect("task");

    let body = serde_json::json!({
        "title": "new", "description": "added", "column_id": cols[1].as_str()
    }).to_string();
    let uri = format!("/api/tasks/{}", task_id.as_str());
    let resp = app.oneshot(
        Request::builder().method(Method::PATCH).uri(uri)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body)).unwrap(),
    ).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["title"], "new");
    assert_eq!(json["description"], "added");
    assert_eq!(json["column_id"], cols[1].as_str());
}

#[tokio::test]
async fn patch_unknown_task_returns_404() {
    let (app, _pool) = fresh_app_with_pool().await;
    let body = r#"{"title":"x"}"#;
    let resp = app.oneshot(
        Request::builder().method(Method::PATCH)
            .uri("/api/tasks/01900000-0000-0000-0000-000000000000")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body)).unwrap(),
    ).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn patch_task_with_empty_body_returns_400() {
    let (app, pool) = fresh_app_with_pool().await;
    let project_id = project::create(&pool, NewProject {
        name: "p".into(), repo_path: "/tmp/p".into(),
    }).await.expect("project");
    let cols = columns::seed_defaults(&pool, &project_id).await.expect("cols");
    let task_id = task::create(&pool, NewTask {
        project_id: project_id.clone(),
        column_id: cols[0].clone(),
        title: "t".into(), description: None,
        tags: vec![], linked_files: vec![],
    }).await.expect("task");

    let body = "{}";
    let uri = format!("/api/tasks/{}", task_id.as_str());
    let resp = app.oneshot(
        Request::builder().method(Method::PATCH).uri(uri)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body)).unwrap(),
    ).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn get_task_attempts_returns_empty_array_for_no_dispatch() {
    let (app, pool) = fresh_app_with_pool().await;
    let project_id = project::create(&pool, NewProject {
        name: "p".into(), repo_path: "/tmp/p".into(),
    }).await.expect("project");
    let cols = columns::seed_defaults(&pool, &project_id).await.expect("cols");
    let task_id = task::create(&pool, NewTask {
        project_id: project_id.clone(),
        column_id: cols[0].clone(),
        title: "t".into(), description: None,
        tags: vec![], linked_files: vec![],
    }).await.expect("task");

    let uri = format!("/api/tasks/{}/attempts", task_id.as_str());
    let resp = app.oneshot(
        Request::builder().method(Method::GET).uri(uri).body(Body::empty()).unwrap(),
    ).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(json.is_array());
    assert_eq!(json.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn get_task_attempts_returns_dispatched_attempts() {
    let (app, pool) = fresh_app_with_pool().await;
    let project_id = project::create(&pool, NewProject {
        name: "p".into(), repo_path: "/tmp/p".into(),
    }).await.expect("project");
    let cols = columns::seed_defaults(&pool, &project_id).await.expect("cols");
    let task_id = task::create(&pool, NewTask {
        project_id: project_id.clone(),
        column_id: cols[0].clone(),
        title: "t".into(), description: None,
        tags: vec![], linked_files: vec![],
    }).await.expect("task");

    // Dispatch 2 attempts via the existing endpoint
    let body = r#"{"agent":"mock","batch":2}"#;
    let dispatch_uri = format!("/api/tasks/{}/dispatch", task_id.as_str());
    let resp = app.clone().oneshot(
        Request::builder().method(Method::POST).uri(dispatch_uri)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body)).unwrap(),
    ).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let uri = format!("/api/tasks/{}/attempts", task_id.as_str());
    let resp = app.oneshot(
        Request::builder().method(Method::GET).uri(uri).body(Body::empty()).unwrap(),
    ).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let arr = json.as_array().expect("array");
    assert_eq!(arr.len(), 2);
    for att in arr {
        assert_eq!(att["task_id"], task_id.as_str());
        assert_eq!(att["agent_id"], "mock");
    }
}

#[tokio::test]
async fn patch_task_with_tags_and_linked_files_persists() {
    let (app, pool) = fresh_app_with_pool().await;
    let project_id = project::create(&pool, NewProject {
        name: "p".into(), repo_path: "/tmp/p".into(),
    }).await.expect("project");
    let cols = columns::seed_defaults(&pool, &project_id).await.expect("cols");
    let task_id = task::create(&pool, NewTask {
        project_id: project_id.clone(),
        column_id: cols[0].clone(),
        title: "t".into(),
        description: None,
        tags: vec![],
        linked_files: vec![],
    }).await.expect("task");

    let body = serde_json::json!({
        "tags": ["refactor", "perf"],
        "linked_files": ["src/parser.rs"]
    }).to_string();
    let uri = format!("/api/tasks/{}", task_id.as_str());
    let resp = app.oneshot(
        Request::builder()
            .method(Method::PATCH).uri(uri)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body)).unwrap(),
    ).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let tags = json["tags"].as_array().expect("tags array");
    assert_eq!(tags.len(), 2);
    assert_eq!(tags[0], "refactor");
    assert_eq!(tags[1], "perf");
    let files = json["linked_files"].as_array().expect("files array");
    assert_eq!(files.len(), 1);
    assert_eq!(files[0], "src/parser.rs");
}
