#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kulisawit_agent::{MockAgent, MockMode};
use kulisawit_core::{AgentAdapter, AttemptId, AttemptStatus};
use kulisawit_db::{attempt, columns, connect, migrate, project, task};
use kulisawit_orchestrator::{dispatch_single_attempt, AgentRegistry, Orchestrator, RuntimeConfig};
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;
use tempfile::tempdir;

fn init_repo(dir: &std::path::Path) {
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
            "-c",
            "user.email=t@t",
            "-c",
            "user.name=t",
            "commit",
            "-m",
            "init",
        ])
        .current_dir(dir)
        .status()
        .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cancel_attempt_on_slow_mock_terminates_as_cancelled() {
    let base = tempdir().expect("tmp");
    init_repo(base.path());
    let pool = connect("sqlite::memory:").await.expect("pool");
    migrate(&pool).await.expect("mig");
    let mut registry = AgentRegistry::new();
    registry.register(Arc::new(MockAgent::new(MockMode::Slow)) as Arc<dyn AgentAdapter>);
    let orch = Orchestrator::new(
        pool,
        registry,
        base.path().to_path_buf(),
        base.path().join(".kulisawit/worktrees"),
        RuntimeConfig::default(),
    );

    let project_id = project::create(
        orch.pool(),
        project::NewProject {
            name: "K".into(),
            repo_path: base.path().display().to_string(),
        },
    )
    .await
    .expect("p");
    let cols = columns::seed_defaults(orch.pool(), &project_id)
        .await
        .expect("c");
    let task_id = task::create(
        orch.pool(),
        task::NewTask {
            project_id,
            column_id: cols[0].clone(),
            title: "slow".into(),
            description: None,
            tags: vec![],
            linked_files: vec![],
        },
    )
    .await
    .expect("t");

    let orch = Arc::new(orch);

    let orch_bg = Arc::clone(&orch);
    let task_id_bg = task_id.clone();
    let dispatch_handle =
        tokio::spawn(
            async move { dispatch_single_attempt(&orch_bg, &task_id_bg, "mock", None).await },
        );

    let attempt_id = poll_for_attempt(&orch, &task_id).await;

    tokio::time::sleep(Duration::from_millis(200)).await;
    orch.cancel_attempt(&attempt_id).await.expect("cancel");

    let result = dispatch_handle.await.expect("join").expect("dispatch ok");
    assert_eq!(result, attempt_id);

    let row = attempt::get(orch.pool(), &attempt_id)
        .await
        .expect("get")
        .expect("row");
    assert_eq!(row.status, AttemptStatus::Cancelled);
    assert!(row.started_at.is_some());
    assert!(row.completed_at.is_some());
}

async fn poll_for_attempt(orch: &Orchestrator, task_id: &kulisawit_core::TaskId) -> AttemptId {
    for _ in 0..100 {
        let rows = attempt::list_for_task(orch.pool(), task_id)
            .await
            .expect("list");
        if let Some(first) = rows.into_iter().next() {
            return first.id;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("attempt never appeared in DB");
}
