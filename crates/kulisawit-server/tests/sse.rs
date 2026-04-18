#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use http_body_util::BodyExt;
use kulisawit_agent::MockAgent;
use kulisawit_core::{AgentAdapter, AgentEvent, RunStatus};
use kulisawit_db::{attempt as attempt_db, columns, connect, migrate, project, task};
use kulisawit_orchestrator::{AgentRegistry, Orchestrator, RuntimeConfig};
use kulisawit_server::{routes_for_testing, AppState};
use tempfile::tempdir;
use tower::ServiceExt;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sse_unknown_attempt_returns_404() {
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
                .uri("/api/attempts/01900000-0000-0000-0000-000000000000/events")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sse_terminal_attempt_emits_single_status_and_closes() {
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
            task_id,
            agent_id: "mock".into(),
            prompt_variant: None,
            worktree_path: "/tmp/wt".into(),
            branch_name: "b".into(),
        },
    )
    .await
    .expect("a");
    attempt_db::mark_running(&pool, &attempt_id)
        .await
        .expect("run");
    attempt_db::mark_terminal(&pool, &attempt_id, kulisawit_core::AttemptStatus::Completed)
        .await
        .expect("term");

    let orch = Arc::new(Orchestrator::new(
        pool,
        registry,
        dir.path().to_path_buf(),
        dir.path().join("wt"),
        RuntimeConfig::default(),
    ));
    std::mem::forget(dir);
    let app = routes_for_testing(AppState { orch });

    let uri = format!("/api/attempts/{}/events", attempt_id.as_str());
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
    assert_eq!(
        resp.headers()
            .get(axum::http::header::CONTENT_TYPE)
            .map(|v| v.to_str().unwrap_or("")),
        Some("text/event-stream")
    );
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let s = String::from_utf8_lossy(&body);
    // AttemptStatus::Completed -> RunStatus::Succeeded, serialized as "succeeded"
    assert!(
        s.contains("\"status\":\"Completed\"")
            || s.contains("\"status\":\"completed\"")
            || s.contains("\"status\":\"succeeded\""),
        "expected terminal status envelope, got:\n{s}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sse_live_attempt_streams_events() {
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
            task_id,
            agent_id: "mock".into(),
            prompt_variant: None,
            worktree_path: "/tmp/wt".into(),
            branch_name: "b".into(),
        },
    )
    .await
    .expect("a");
    attempt_db::mark_running(&pool, &attempt_id)
        .await
        .expect("run");

    let orch = Arc::new(Orchestrator::new(
        pool,
        registry,
        dir.path().to_path_buf(),
        dir.path().join("wt"),
        RuntimeConfig::default(),
    ));
    let broadcaster = orch.broadcaster().clone();
    let attempt_id_bg = attempt_id.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        broadcaster.send(
            &attempt_id_bg,
            AgentEvent::Stdout {
                text: "live-event".into(),
            },
        );
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        broadcaster.send(
            &attempt_id_bg,
            AgentEvent::Status {
                status: RunStatus::Succeeded,
                detail: None,
            },
        );
        broadcaster.close(&attempt_id_bg);
    });
    std::mem::forget(dir);
    let app = routes_for_testing(AppState { orch });

    let uri = format!("/api/attempts/{}/events", attempt_id.as_str());
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
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let s = String::from_utf8_lossy(&body);
    assert!(s.contains("live-event"), "expected stdout in stream:\n{s}");
    assert!(
        s.contains("Completed") || s.contains("completed") || s.contains("succeeded"),
        "expected terminal status in stream:\n{s}"
    );
}
