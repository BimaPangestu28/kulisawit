#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kulisawit_core::adapter::AgentEvent;
use kulisawit_db::{attempt, columns, connect, events, migrate, project, task};

async fn setup() -> (kulisawit_db::DbPool, kulisawit_core::AttemptId) {
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
    .expect("k");
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
    .expect("l");
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
    .expect("b");
    (pool, attempt_id)
}

#[tokio::test]
async fn append_and_read_event_stream_in_order() {
    let (pool, attempt_id) = setup().await;
    events::append(
        &pool,
        &attempt_id,
        &AgentEvent::Stdout { text: "one".into() },
    )
    .await
    .unwrap();
    events::append(
        &pool,
        &attempt_id,
        &AgentEvent::Stdout { text: "two".into() },
    )
    .await
    .unwrap();
    events::append(
        &pool,
        &attempt_id,
        &AgentEvent::FileEdit {
            path: "a.rs".into(),
            diff: None,
        },
    )
    .await
    .unwrap();

    let evts = events::list_for_attempt(&pool, &attempt_id).await.unwrap();
    assert_eq!(evts.len(), 3);
    match &evts[0] {
        AgentEvent::Stdout { text } => assert_eq!(text, "one"),
        other => panic!("unexpected first: {other:?}"),
    }
    match &evts[2] {
        AgentEvent::FileEdit { path, .. } => assert_eq!(path, "a.rs"),
        other => panic!("unexpected third: {other:?}"),
    }
}
