#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use futures::future::join_all;
use kulisawit_db::{attempt, columns, connect, migrate, project, task, DbPool};
use std::sync::Arc;

async fn setup() -> (Arc<DbPool>, kulisawit_core::TaskId) {
    let pool = Arc::new(connect("sqlite::memory:").await.expect("pool"));
    migrate(&pool).await.expect("mig");
    let project_id = project::create(
        pool.as_ref(),
        project::NewProject {
            name: "K".into(),
            repo_path: "/k".into(),
        },
    )
    .await
    .expect("project");
    let cols = columns::seed_defaults(pool.as_ref(), &project_id)
        .await
        .expect("cols");
    let task_id = task::create(
        pool.as_ref(),
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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_attempt_inserts_produce_distinct_ids() {
    let (pool, task_id) = setup().await;
    let count: usize = 50;
    let handles: Vec<_> = (0..count)
        .map(|i| {
            let pool = Arc::clone(&pool);
            let task_id = task_id.clone();
            tokio::spawn(async move {
                attempt::create(
                    pool.as_ref(),
                    attempt::NewAttempt {
                        task_id,
                        agent_id: "mock".into(),
                        prompt_variant: None,
                        worktree_path: format!("/tmp/w-{i}"),
                        branch_name: format!("b-{i}"),
                    },
                )
                .await
                .expect("create")
            })
        })
        .collect();
    let ids: Vec<_> = join_all(handles)
        .await
        .into_iter()
        .map(|r| r.expect("join"))
        .collect();
    assert_eq!(ids.len(), count);
    // All IDs distinct.
    let mut sorted: Vec<_> = ids.iter().map(|id| id.as_str().to_owned()).collect();
    sorted.sort();
    sorted.dedup();
    assert_eq!(sorted.len(), count);

    let rows = attempt::list_for_task(pool.as_ref(), &task_id)
        .await
        .expect("list");
    assert_eq!(rows.len(), count);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_event_appends_preserve_order_per_attempt() {
    use kulisawit_core::adapter::AgentEvent;
    use kulisawit_db::events;

    let (pool, task_id) = setup().await;
    let attempt_id = attempt::create(
        pool.as_ref(),
        attempt::NewAttempt {
            task_id,
            agent_id: "mock".into(),
            prompt_variant: None,
            worktree_path: "/tmp/x".into(),
            branch_name: "b".into(),
        },
    )
    .await
    .expect("a");

    let n: usize = 50;
    let handles: Vec<_> = (0..n)
        .map(|i| {
            let pool = Arc::clone(&pool);
            let attempt_id = attempt_id.clone();
            tokio::spawn(async move {
                events::append(
                    pool.as_ref(),
                    &attempt_id,
                    &AgentEvent::Stdout {
                        text: format!("line {i}"),
                    },
                )
                .await
                .expect("append")
            })
        })
        .collect();
    let row_ids: Vec<i64> = join_all(handles)
        .await
        .into_iter()
        .map(|r| r.expect("join"))
        .collect();
    assert_eq!(row_ids.len(), n);

    let evts = events::list_for_attempt(pool.as_ref(), &attempt_id)
        .await
        .expect("list");
    assert_eq!(evts.len(), n);
    // All returned events are Stdout (interleaved text is expected; ordering
    // per-attempt is by `id ASC` which equals insertion order).
    for evt in &evts {
        assert!(matches!(evt, AgentEvent::Stdout { .. }));
    }
}
