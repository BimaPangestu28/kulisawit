#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use http_body_util::BodyExt;
use kulisawit_agent::MockAgent;
use kulisawit_core::AgentAdapter;
use kulisawit_db::{
    columns, connect, migrate, project,
    project::NewProject,
    task,
    task::NewTask,
};
use kulisawit_orchestrator::{AgentRegistry, Orchestrator, RuntimeConfig};
use kulisawit_server::{routes_for_testing, AppState};
use tempfile::tempdir;
use tower::ServiceExt;

async fn fresh_app_with_pool() -> (axum::Router, kulisawit_db::DbPool) {
    let pool = connect("sqlite::memory:").await.expect("pool");
    migrate(&pool).await.expect("mig");
    let mut registry = AgentRegistry::new();
    registry.register(Arc::new(MockAgent::default()) as Arc<dyn AgentAdapter>);
    let dir = tempdir().expect("tmp");
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

#[tokio::test]
async fn get_board_returns_404_for_unknown_project() {
    let (app, _pool) = fresh_app_with_pool().await;
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/projects/01900000-0000-0000-0000-000000000000/board")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn get_board_returns_5_empty_columns_for_fresh_project() {
    let (app, pool) = fresh_app_with_pool().await;
    let project_id = project::create(&pool, NewProject {
        name: "demo".into(),
        repo_path: "/tmp/d".into(),
    }).await.expect("create");
    columns::seed_defaults(&pool, &project_id).await.expect("seed");
    let uri = format!("/api/projects/{}/board", project_id.as_str());
    let resp = app
        .oneshot(
            Request::builder().method(Method::GET).uri(uri).body(Body::empty()).unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["project"]["name"], "demo");
    let columns_arr = json["columns"].as_array().expect("columns");
    assert_eq!(columns_arr.len(), 5);
    let names: Vec<&str> = columns_arr.iter().map(|c| c["name"].as_str().unwrap()).collect();
    assert_eq!(names, vec!["Backlog", "Todo", "Doing", "Review", "Done"]);
    for col in columns_arr {
        assert_eq!(col["tasks"].as_array().unwrap().len(), 0);
    }
}

#[tokio::test]
async fn get_board_groups_tasks_into_correct_columns_in_position_order() {
    let (app, pool) = fresh_app_with_pool().await;
    let project_id = project::create(&pool, NewProject {
        name: "demo".into(),
        repo_path: "/tmp/d".into(),
    }).await.expect("create");
    let cols = columns::seed_defaults(&pool, &project_id).await.expect("seed");
    // 2 tasks in Backlog (col 0), 1 in Doing (col 2)
    for (col_idx, title) in [(0, "first"), (0, "second"), (2, "in_progress")] {
        task::create(&pool, NewTask {
            project_id: project_id.clone(),
            column_id: cols[col_idx].clone(),
            title: title.into(),
            description: None,
            tags: vec![],
            linked_files: vec![],
        }).await.expect("task");
    }
    let uri = format!("/api/projects/{}/board", project_id.as_str());
    let resp = app
        .oneshot(
            Request::builder().method(Method::GET).uri(uri).body(Body::empty()).unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let cols_arr = json["columns"].as_array().expect("columns");
    let backlog_tasks = cols_arr[0]["tasks"].as_array().expect("backlog tasks");
    assert_eq!(backlog_tasks.len(), 2);
    assert_eq!(backlog_tasks[0]["title"], "first");
    assert_eq!(backlog_tasks[1]["title"], "second");
    assert_eq!(cols_arr[1]["tasks"].as_array().unwrap().len(), 0);
    let doing_tasks = cols_arr[2]["tasks"].as_array().expect("doing tasks");
    assert_eq!(doing_tasks.len(), 1);
    assert_eq!(doing_tasks[0]["title"], "in_progress");
}
