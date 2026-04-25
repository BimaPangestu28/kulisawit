#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kulisawit_agent::{MockAgent, MockMode};
use kulisawit_core::{AgentAdapter, AttemptStatus};
use kulisawit_db::{attempt, columns, connect, events, migrate, project, task};
use kulisawit_orchestrator::{dispatch_single_attempt, AgentRegistry, Orchestrator, RuntimeConfig};
use std::process::Command;
use std::sync::Arc;
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

async fn build_orch(repo_dir: &std::path::Path, mode: MockMode) -> Arc<Orchestrator> {
    let pool = connect("sqlite::memory:").await.expect("pool");
    migrate(&pool).await.expect("mig");
    let mut registry = AgentRegistry::new();
    registry.register(Arc::new(MockAgent::new(mode)) as Arc<dyn AgentAdapter>);
    Arc::new(Orchestrator::new(
        pool,
        registry,
        repo_dir.to_path_buf(),
        repo_dir.join(".kulisawit/worktrees"),
        RuntimeConfig::default(),
    ))
}

async fn seed_task(orch: &Orchestrator) -> kulisawit_core::TaskId {
    let project_id = project::create(
        orch.pool(),
        project::NewProject {
            name: "K".into(),
            repo_path: orch.repo_root().display().to_string(),
        },
    )
    .await
    .expect("p");
    let cols = columns::seed_defaults(orch.pool(), &project_id)
        .await
        .expect("c");
    task::create(
        orch.pool(),
        task::NewTask {
            project_id,
            column_id: cols[0].clone(),
            title: "smoke".into(),
            description: Some("smoke test".into()),
            tags: vec![],
            linked_files: vec![],
        },
    )
    .await
    .expect("t")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dispatch_succeed_records_completed_attempt_and_events() {
    let base = tempdir().expect("tmp");
    init_repo(base.path());
    let orch = build_orch(base.path(), MockMode::Succeed).await;
    let task_id = seed_task(&orch).await;

    let attempt_id = dispatch_single_attempt(&orch, &task_id, "mock", None)
        .await
        .expect("dispatch");

    let a = attempt::get(orch.pool(), &attempt_id)
        .await
        .expect("get")
        .expect("row");
    assert_eq!(a.status, AttemptStatus::Completed);
    assert!(a.started_at.is_some());
    assert!(a.completed_at.is_some());

    let evts = events::list_for_attempt(orch.pool(), &attempt_id)
        .await
        .expect("events");
    assert!(
        evts.len() >= 5,
        "expected >= 5 events from MockAgent, got {}",
        evts.len()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dispatch_failing_mock_records_failed_attempt() {
    let base = tempdir().expect("tmp");
    init_repo(base.path());
    let orch = build_orch(base.path(), MockMode::Fail).await;
    let task_id = seed_task(&orch).await;

    let attempt_id = dispatch_single_attempt(&orch, &task_id, "mock", None)
        .await
        .expect("dispatch");

    let a = attempt::get(orch.pool(), &attempt_id)
        .await
        .expect("get")
        .expect("row");
    assert_eq!(a.status, AttemptStatus::Failed);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dispatch_with_missing_task_returns_invalid() {
    let base = tempdir().expect("tmp");
    init_repo(base.path());
    let orch = build_orch(base.path(), MockMode::Succeed).await;
    let bogus = kulisawit_core::TaskId::new();
    let err = dispatch_single_attempt(&orch, &bogus, "mock", None)
        .await
        .expect_err("should fail");
    let msg = format!("{err}");
    assert!(msg.contains("task not found"), "got: {msg}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dispatch_with_unknown_agent_returns_invalid() {
    let base = tempdir().expect("tmp");
    init_repo(base.path());
    let orch = build_orch(base.path(), MockMode::Succeed).await;
    let task_id = seed_task(&orch).await;
    let err = dispatch_single_attempt(&orch, &task_id, "not-registered", None)
        .await
        .expect_err("should fail");
    let msg = format!("{err}");
    assert!(msg.contains("agent not registered"), "got: {msg}");
}
