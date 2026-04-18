//! Sanity check: all *_at columns live in millisecond range (13 digits),
//! not seconds (10 digits). Catches regressions in the unit convention.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kulisawit_core::{ColumnId, ProjectId};
use kulisawit_db::{attempt, columns, connect, migrate, project, task};

/// November 2023 in unix ms = 1_700_000_000_000. Any value above this threshold
/// is necessarily milliseconds (not seconds); seconds would be ~1.7e9, ms ~1.7e12.
const MILLIS_FLOOR: i64 = 1_700_000_000_000;

async fn setup() -> (kulisawit_db::DbPool, ProjectId, ColumnId) {
    let pool = connect("sqlite::memory:").await.expect("pool");
    migrate(&pool).await.expect("mig");
    let project_id = project::create(
        &pool,
        project::NewProject {
            name: "T".into(),
            repo_path: "/t".into(),
        },
    )
    .await
    .expect("project");
    let col_ids = columns::seed_defaults(&pool, &project_id)
        .await
        .expect("cols");
    (pool, project_id, col_ids[0].clone())
}

#[tokio::test]
async fn project_created_at_is_millis() {
    let pool = connect("sqlite::memory:").await.expect("pool");
    migrate(&pool).await.expect("mig");
    let id = project::create(
        &pool,
        project::NewProject {
            name: "X".into(),
            repo_path: "/x".into(),
        },
    )
    .await
    .expect("create");
    let p = project::get(&pool, &id).await.expect("ok").expect("row");
    assert!(
        p.created_at > MILLIS_FLOOR,
        "created_at={} not in ms range",
        p.created_at
    );
}

#[tokio::test]
async fn task_created_and_updated_at_are_millis() {
    let (pool, project_id, col_id) = setup().await;
    let id = task::create(
        &pool,
        task::NewTask {
            project_id,
            column_id: col_id,
            title: "t".into(),
            description: None,
            tags: vec![],
            linked_files: vec![],
        },
    )
    .await
    .expect("create");
    let l = task::get(&pool, &id).await.expect("ok").expect("row");
    assert!(l.created_at > MILLIS_FLOOR);
    assert!(l.updated_at > MILLIS_FLOOR);
}

#[tokio::test]
async fn attempt_timestamps_are_millis() {
    let (pool, project_id, col_id) = setup().await;
    let task_id = task::create(
        &pool,
        task::NewTask {
            project_id,
            column_id: col_id,
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
    attempt::mark_running(&pool, &attempt_id).await.expect("r");
    attempt::mark_terminal(&pool, &attempt_id, kulisawit_core::AttemptStatus::Completed)
        .await
        .expect("done");
    let a = attempt::get(&pool, &attempt_id)
        .await
        .expect("ok")
        .expect("row");
    assert!(a.started_at.unwrap() > MILLIS_FLOOR);
    assert!(a.completed_at.unwrap() > MILLIS_FLOOR);
}
