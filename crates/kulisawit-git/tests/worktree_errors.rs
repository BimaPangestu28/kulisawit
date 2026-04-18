#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kulisawit_git::worktree::{create_worktree, CreateWorktreeRequest};
use std::process::Command;
use tempfile::tempdir;

fn init_repo(dir: &std::path::Path) {
    Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(dir)
        .status()
        .unwrap();
    std::fs::write(dir.join("README.md"), "x").unwrap();
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

#[tokio::test]
async fn create_worktree_over_existing_path_errors() {
    let base = tempdir().unwrap();
    init_repo(base.path());
    let req = CreateWorktreeRequest {
        repo_root: base.path().to_path_buf(),
        worktree_root: base.path().join(".kulisawit/worktrees"),
        attempt_short_id: "dup".into(),
        branch_name: "kulisawit/t/dup".into(),
        base_ref: "main".into(),
    };
    // First create succeeds.
    create_worktree(req.clone()).await.expect("first create");
    // Second create at the same path should fail.
    let err = create_worktree(req).await;
    assert!(
        err.is_err(),
        "expected Err when worktree path exists, got {err:?}"
    );
}

#[tokio::test]
async fn create_worktree_with_invalid_base_ref_errors() {
    let base = tempdir().unwrap();
    init_repo(base.path());
    let req = CreateWorktreeRequest {
        repo_root: base.path().to_path_buf(),
        worktree_root: base.path().join(".kulisawit/worktrees"),
        attempt_short_id: "bad".into(),
        branch_name: "kulisawit/t/bad".into(),
        base_ref: "nonexistent-ref".into(),
    };
    let err = create_worktree(req).await;
    assert!(
        err.is_err(),
        "expected Err for invalid base_ref, got {err:?}"
    );
}
