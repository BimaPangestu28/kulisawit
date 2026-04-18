#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kulisawit_core::AttemptStatus;
use kulisawit_db::{attempt, columns, connect, migrate, project, task};

async fn setup_attempt() -> (kulisawit_db::DbPool, kulisawit_core::AttemptId) {
    let pool = connect("sqlite::memory:").await.expect("pool");
    migrate(&pool).await.expect("mig");
    let project_id = project::create(
        &pool,
        project::NewProject {
            name: "P".into(),
            repo_path: "/p".into(),
        },
    )
    .await
    .expect("p");
    let cols = columns::seed_defaults(&pool, &project_id)
        .await
        .expect("cols");
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
    .expect("task");
    let attempt_id = attempt::create(
        &pool,
        attempt::NewAttempt {
            task_id,
            agent_id: "mock".into(),
            prompt_variant: None,
            worktree_path: "/x".into(),
            branch_name: "b".into(),
        },
    )
    .await
    .expect("a");
    (pool, attempt_id)
}

#[tokio::test]
async fn queued_to_failed_directly() {
    let (pool, attempt_id) = setup_attempt().await;
    attempt::mark_terminal(&pool, &attempt_id, AttemptStatus::Failed)
        .await
        .expect("fail");
    let a = attempt::get(&pool, &attempt_id)
        .await
        .expect("ok")
        .expect("row");
    assert_eq!(a.status, AttemptStatus::Failed);
    assert!(a.completed_at.is_some());
}

#[tokio::test]
async fn running_to_failed() {
    let (pool, attempt_id) = setup_attempt().await;
    attempt::mark_running(&pool, &attempt_id).await.expect("r");
    attempt::mark_terminal(&pool, &attempt_id, AttemptStatus::Failed)
        .await
        .expect("fail");
    let a = attempt::get(&pool, &attempt_id)
        .await
        .expect("ok")
        .expect("row");
    assert_eq!(a.status, AttemptStatus::Failed);
    assert!(a.started_at.is_some());
    assert!(a.completed_at.is_some());
}

#[tokio::test]
async fn mark_terminal_rejects_non_terminal_status() {
    let (pool, attempt_id) = setup_attempt().await;
    let err = attempt::mark_terminal(&pool, &attempt_id, AttemptStatus::Running).await;
    assert!(
        err.is_err(),
        "expected Err for non-terminal mark_terminal, got {err:?}"
    );
}
