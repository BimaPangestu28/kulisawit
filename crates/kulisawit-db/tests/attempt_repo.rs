#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kulisawit_core::{AttemptStatus, TaskId};
use kulisawit_db::{attempt, columns, connect, migrate, project, task};

async fn setup_task() -> (kulisawit_db::DbPool, TaskId) {
    let pool = connect("sqlite::memory:").await.expect("pool");
    migrate(&pool).await.expect("mig");
    let project_id = project::create(
        &pool,
        project::NewProject {
            name: "K".into(),
            repo_path: "/k".into(),
        },
    )
    .await
    .expect("project");
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
    (pool, task_id)
}

#[tokio::test]
async fn create_attempt_defaults_to_queued() {
    let (pool, task_id) = setup_task().await;
    let id = attempt::create(
        &pool,
        attempt::NewAttempt {
            task_id,
            agent_id: "mock".into(),
            prompt_variant: None,
            worktree_path: "/tmp/worktree-1".into(),
            branch_name: "kulisawit/l1/b1".into(),
        },
    )
    .await
    .expect("create");
    let b = attempt::get(&pool, &id).await.expect("ok").expect("row");
    assert_eq!(b.status, AttemptStatus::Queued);
    assert!(b.started_at.is_none());
    assert!(b.completed_at.is_none());
}

#[tokio::test]
async fn transition_queued_to_running_sets_started_at() {
    let (pool, task_id) = setup_task().await;
    let id = attempt::create(
        &pool,
        attempt::NewAttempt {
            task_id,
            agent_id: "mock".into(),
            prompt_variant: None,
            worktree_path: "/tmp/x".into(),
            branch_name: "kulisawit/x".into(),
        },
    )
    .await
    .expect("c");
    attempt::mark_running(&pool, &id).await.expect("running");
    let b = attempt::get(&pool, &id).await.expect("ok").expect("row");
    assert_eq!(b.status, AttemptStatus::Running);
    assert!(b.started_at.is_some());
}

#[tokio::test]
async fn mark_terminal_sets_completed_at() {
    let (pool, task_id) = setup_task().await;
    let id = attempt::create(
        &pool,
        attempt::NewAttempt {
            task_id,
            agent_id: "mock".into(),
            prompt_variant: None,
            worktree_path: "/tmp/y".into(),
            branch_name: "kulisawit/y".into(),
        },
    )
    .await
    .expect("c");
    attempt::mark_running(&pool, &id).await.expect("r");
    attempt::mark_terminal(&pool, &id, AttemptStatus::Completed)
        .await
        .expect("done");
    let b = attempt::get(&pool, &id).await.expect("ok").expect("row");
    assert_eq!(b.status, AttemptStatus::Completed);
    assert!(b.completed_at.is_some());
}

#[tokio::test]
async fn list_for_task_returns_all() {
    let (pool, task_id) = setup_task().await;
    for i in 0..3 {
        attempt::create(
            &pool,
            attempt::NewAttempt {
                task_id: task_id.clone(),
                agent_id: "mock".into(),
                prompt_variant: None,
                worktree_path: format!("/tmp/{i}"),
                branch_name: format!("b-{i}"),
            },
        )
        .await
        .expect("c");
    }
    let rows = attempt::list_for_task(&pool, &task_id).await.expect("list");
    assert_eq!(rows.len(), 3);
}
