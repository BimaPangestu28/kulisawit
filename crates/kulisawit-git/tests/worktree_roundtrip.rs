#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kulisawit_git::worktree::{create_worktree, remove_worktree, CreateWorktreeRequest};
use std::process::Command;
use tempfile::tempdir;

fn init_repo_with_commit(dir: &std::path::Path) {
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
async fn create_and_remove_worktree_roundtrip() {
    let base = tempdir().expect("tmp");
    init_repo_with_commit(base.path());
    let worktree_root = base.path().join(".kulisawit/worktrees");
    let req = CreateWorktreeRequest {
        repo_root: base.path().to_path_buf(),
        worktree_root,
        attempt_short_id: "abc123".into(),
        branch_name: "kulisawit/lx/abc123".into(),
        base_ref: "main".into(),
    };
    let outcome = create_worktree(req.clone()).await.expect("create");
    assert!(outcome.worktree_path.exists());
    assert!(outcome.worktree_path.join("README.md").exists());

    remove_worktree(&req.repo_root, &outcome.worktree_path)
        .await
        .expect("remove");
    assert!(!outcome.worktree_path.exists());
}
