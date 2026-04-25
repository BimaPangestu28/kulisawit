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
