#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kulisawit_git::branch::commit_all_in_worktree;
use kulisawit_git::worktree::{create_worktree, CreateWorktreeRequest};
use std::process::Command;
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

#[tokio::test]
async fn commit_all_captures_added_file() {
    let base = tempdir().unwrap();
    init_repo(base.path());
    let outcome = create_worktree(CreateWorktreeRequest {
        repo_root: base.path().to_path_buf(),
        worktree_root: base.path().join(".kulisawit/worktrees"),
        attempt_short_id: "ab".into(),
        branch_name: "kulisawit/t/ab".into(),
        base_ref: "main".into(),
    })
    .await
    .unwrap();

    // agent "edits" a file in the worktree
    std::fs::write(outcome.worktree_path.join("NEW.txt"), "hello\n").unwrap();

    let summary = commit_all_in_worktree(&outcome.worktree_path, "kulisawit: attempt ab for test")
        .await
        .unwrap();
    assert!(summary.changed); // commit happened
    assert!(summary.message.starts_with("kulisawit: attempt ab"));
}

#[tokio::test]
async fn commit_all_no_op_when_clean() {
    let base = tempdir().unwrap();
    init_repo(base.path());
    let outcome = create_worktree(CreateWorktreeRequest {
        repo_root: base.path().to_path_buf(),
        worktree_root: base.path().join(".kulisawit/worktrees"),
        attempt_short_id: "cd".into(),
        branch_name: "kulisawit/t/cd".into(),
        base_ref: "main".into(),
    })
    .await
    .unwrap();
    let summary = commit_all_in_worktree(&outcome.worktree_path, "empty")
        .await
        .unwrap();
    assert!(!summary.changed);
}
