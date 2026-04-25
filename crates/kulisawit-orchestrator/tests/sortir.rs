#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;
use kulisawit_orchestrator::sortir::{run_checks, load_config, ConfigError};
use tempfile::tempdir;

async fn write_config(dir: &Path, contents: &str) {
    let kulisawit_dir = dir.join(".kulisawit");
    tokio::fs::create_dir_all(&kulisawit_dir).await.expect("mkdir");
    tokio::fs::write(kulisawit_dir.join("sortir.toml"), contents)
        .await
        .expect("write");
}

#[tokio::test]
async fn load_config_returns_none_when_file_absent() {
    let dir = tempdir().expect("tmp");
    let result = load_config(dir.path()).await.expect("load");
    assert!(result.is_none(), "expected None when sortir.toml absent");
}

#[tokio::test]
async fn load_config_returns_invalid_when_toml_malformed() {
    let dir = tempdir().expect("tmp");
    write_config(dir.path(), "this is not valid toml = =").await;
    let err = load_config(dir.path()).await.expect_err("expected ConfigError");
    match err {
        ConfigError::Parse(_) => {}
        ConfigError::Io(_) => panic!("expected Parse, got Io"),
    }
}

#[tokio::test]
async fn run_checks_all_pass_returns_passed_with_aggregated_output() {
    let dir = tempdir().expect("tmp");
    write_config(
        dir.path(),
        r#"
[[checks]]
name = "smoke-true"
command = ["true"]
timeout_secs = 5
"#,
    ).await;
    let config = load_config(dir.path()).await.expect("load").expect("some");
    let (status, output) = run_checks(&config, dir.path()).await;
    assert_eq!(status, kulisawit_core::VerificationStatus::Passed);
    assert!(output.contains("=== smoke-true (passed"), "got: {output}");
}

#[tokio::test]
async fn run_checks_one_failing_marks_failed_but_runs_all() {
    let dir = tempdir().expect("tmp");
    write_config(
        dir.path(),
        r#"
[[checks]]
name = "first"
command = ["true"]
timeout_secs = 5

[[checks]]
name = "boom"
command = ["false"]
timeout_secs = 5

[[checks]]
name = "third"
command = ["true"]
timeout_secs = 5
"#,
    ).await;
    let config = load_config(dir.path()).await.expect("load").expect("some");
    let (status, output) = run_checks(&config, dir.path()).await;
    assert_eq!(status, kulisawit_core::VerificationStatus::Failed);
    assert!(output.contains("=== first ("), "missing first block");
    assert!(output.contains("=== boom ("), "missing boom block");
    assert!(output.contains("=== third ("), "missing third block (rest must still run)");
}

#[tokio::test]
async fn run_checks_timeout_marks_failed_with_marker() {
    let dir = tempdir().expect("tmp");
    write_config(
        dir.path(),
        r#"
[[checks]]
name = "stuck"
command = ["sleep", "10"]
timeout_secs = 1
"#,
    ).await;
    let config = load_config(dir.path()).await.expect("load").expect("some");
    let (status, output) = run_checks(&config, dir.path()).await;
    assert_eq!(status, kulisawit_core::VerificationStatus::Failed);
    assert!(output.contains("TIMEOUT"), "expected TIMEOUT marker, got: {output}");
}

use std::sync::Arc;
use std::time::Duration as StdDuration;
use kulisawit_agent::MockAgent;
use kulisawit_core::AgentAdapter;
use kulisawit_db::{columns, connect, migrate, project, project::NewProject, task, task::NewTask, attempt as attempt_db};
use kulisawit_orchestrator::{dispatch_batch_spawned, AgentRegistry, Orchestrator, RuntimeConfig};
use std::process::Command;

#[tokio::test]
async fn dispatch_with_succeeded_agent_triggers_sortir() {
    let pool = connect("sqlite::memory:").await.expect("pool");
    migrate(&pool).await.expect("mig");
    let mut registry = AgentRegistry::new();
    registry.register(Arc::new(MockAgent::default()) as Arc<dyn AgentAdapter>);

    let dir = tempfile::tempdir().expect("tmp");
    let repo_path = dir.path().to_path_buf();
    Command::new("git").arg("init").arg("-q").arg(&repo_path).output().expect("git init");
    std::fs::write(repo_path.join("README"), "init").expect("write");
    Command::new("git").current_dir(&repo_path).args(["add", "README"]).output().expect("add");
    Command::new("git").current_dir(&repo_path)
        .args(["-c", "user.email=a@b", "-c", "user.name=t", "commit", "-qm", "init"])
        .output().expect("commit");

    // No sortir.toml → expect Skipped
    let orch = Arc::new(Orchestrator::new(
        pool.clone(), registry, repo_path.clone(), dir.path().join("wt"), RuntimeConfig::default(),
    ));
    let project_id = project::create(&pool, NewProject {
        name: "p".into(), repo_path: repo_path.display().to_string(),
    }).await.expect("project");
    let cols = columns::seed_defaults(&pool, &project_id).await.expect("cols");
    let task_id = task::create(&pool, NewTask {
        project_id: project_id.clone(),
        column_id: cols[0].clone(),
        title: "t".into(), description: None,
        tags: vec![], linked_files: vec![],
    }).await.expect("task");

    let attempt_ids = dispatch_batch_spawned(&orch, &task_id, "mock", 1, None)
        .await.expect("dispatch");
    assert_eq!(attempt_ids.len(), 1);
    let attempt_id = attempt_ids[0].clone();

    // Wait for sortir to finish (best-effort poll)
    let mut verification: Option<kulisawit_core::VerificationStatus> = None;
    for _ in 0..50 {
        tokio::time::sleep(StdDuration::from_millis(100)).await;
        if let Some(a) = attempt_db::get(&pool, &attempt_id).await.expect("get") {
            if let Some(v) = a.verification_status {
                verification = Some(v);
                break;
            }
        }
    }
    assert_eq!(verification, Some(kulisawit_core::VerificationStatus::Skipped));
}
