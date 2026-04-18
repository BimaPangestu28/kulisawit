#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kulisawit_agent::{MockAgent, MockMode};
use kulisawit_core::{AgentAdapter, AttemptStatus};
use kulisawit_db::{attempt, columns, connect, migrate, project, task};
use kulisawit_orchestrator::{dispatch_batch, AgentRegistry, Orchestrator, RuntimeConfig};
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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn dispatch_batch_of_three_all_complete() {
    let base = tempdir().expect("tmp");
    init_repo(base.path());
    let pool = connect("sqlite::memory:").await.expect("pool");
    migrate(&pool).await.expect("mig");
    let mut registry = AgentRegistry::new();
    registry.register(Arc::new(MockAgent::new(MockMode::Succeed)) as Arc<dyn AgentAdapter>);
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
            title: "batch".into(),
            description: Some("d".into()),
            tags: vec![],
            linked_files: vec![],
        },
    )
    .await
    .expect("t");

    let ids = dispatch_batch(&orch, &task_id, "mock", 3, None)
        .await
        .expect("batch");
    assert_eq!(ids.len(), 3);
    let mut strs: Vec<String> = ids.iter().map(|a| a.as_str().to_owned()).collect();
    strs.sort();
    strs.dedup();
    assert_eq!(strs.len(), 3, "three distinct AttemptIds");

    for id in &ids {
        let row = attempt::get(orch.pool(), id)
            .await
            .expect("get")
            .expect("row");
        assert_eq!(row.status, AttemptStatus::Completed);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dispatch_batch_with_variants_len_mismatch_errors() {
    let base = tempdir().expect("tmp");
    init_repo(base.path());
    let pool = connect("sqlite::memory:").await.expect("pool");
    migrate(&pool).await.expect("mig");
    let mut registry = AgentRegistry::new();
    registry.register(Arc::new(MockAgent::default()) as Arc<dyn AgentAdapter>);
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
            title: "t".into(),
            description: None,
            tags: vec![],
            linked_files: vec![],
        },
    )
    .await
    .expect("t");

    let err = dispatch_batch(
        &orch,
        &task_id,
        "mock",
        2,
        Some(vec!["a".into(), "b".into(), "c".into()]),
    )
    .await
    .expect_err("should error");
    let msg = format!("{err}");
    assert!(msg.contains("variants length"), "got: {msg}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn dispatch_batch_spawned_returns_ids_before_agents_finish() {
    let base = tempdir().expect("tmp");
    init_repo(base.path());
    let pool = connect("sqlite::memory:").await.expect("pool");
    migrate(&pool).await.expect("mig");
    let mut registry = AgentRegistry::new();
    registry.register(Arc::new(MockAgent::new(MockMode::Slow)) as Arc<dyn AgentAdapter>);
    let orch = std::sync::Arc::new(Orchestrator::new(
        pool,
        registry,
        base.path().to_path_buf(),
        base.path().join(".kulisawit/worktrees"),
        RuntimeConfig::default(),
    ));

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
            title: "batch spawned".into(),
            description: None,
            tags: vec![],
            linked_files: vec![],
        },
    )
    .await
    .expect("t");

    let before = std::time::Instant::now();
    let ids = kulisawit_orchestrator::dispatch_batch_spawned(&orch, &task_id, "mock", 2, None)
        .await
        .expect("spawned");
    let elapsed_ms = before.elapsed().as_millis();
    assert_eq!(ids.len(), 2);
    assert!(
        elapsed_ms < 500,
        "dispatch_batch_spawned should return immediately, took {elapsed_ms}ms"
    );

    for id in &ids {
        let row = attempt::get(orch.pool(), id)
            .await
            .expect("get")
            .expect("row");
        assert!(
            matches!(
                row.status,
                kulisawit_core::AttemptStatus::Queued | kulisawit_core::AttemptStatus::Running
            ),
            "expected Queued/Running, got {:?}",
            row.status
        );
    }

    for id in &ids {
        for _ in 0..200 {
            let row = attempt::get(orch.pool(), id)
                .await
                .expect("get")
                .expect("row");
            if matches!(
                row.status,
                kulisawit_core::AttemptStatus::Completed
                    | kulisawit_core::AttemptStatus::Failed
                    | kulisawit_core::AttemptStatus::Cancelled
            ) {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    }
}
