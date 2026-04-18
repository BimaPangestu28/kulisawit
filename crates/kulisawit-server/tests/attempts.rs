#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use http_body_util::BodyExt;
use kulisawit_agent::MockAgent;
use kulisawit_core::AgentAdapter;
use kulisawit_db::{attempt as attempt_db, columns, connect, migrate, project, task};
use kulisawit_orchestrator::{AgentRegistry, Orchestrator, RuntimeConfig};
use kulisawit_server::{routes_for_testing, AppState};
use tempfile::tempdir;
use tower::ServiceExt;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_attempt_unknown_returns_404() {
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
    let app = routes_for_testing(AppState { orch });

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/attempts/01900000-0000-0000-0000-000000000000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_attempt_existing_returns_200_with_expected_shape() {
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
    let attempt_id = attempt_db::create(
        &pool,
        attempt_db::NewAttempt {
            task_id: task_id.clone(),
            agent_id: "mock".into(),
            prompt_variant: None,
            worktree_path: "/tmp/wt".into(),
            branch_name: "kulisawit/a/b".into(),
        },
    )
    .await
    .expect("a");

    let orch = Arc::new(Orchestrator::new(
        pool,
        registry,
        dir.path().to_path_buf(),
        dir.path().join("wt"),
        RuntimeConfig::default(),
    ));
    std::mem::forget(dir);
    let app = routes_for_testing(AppState { orch });

    let uri = format!("/api/attempts/{}", attempt_id.as_str());
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(&uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["id"], attempt_id.as_str());
    assert_eq!(json["agent_id"], "mock");
    assert_eq!(json["status"], "queued");
    assert_eq!(json["branch_name"], "kulisawit/a/b");
    assert!(!json
        .as_object()
        .unwrap()
        .contains_key("verification_status"));
}
