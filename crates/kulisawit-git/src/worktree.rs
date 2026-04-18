//! Managing isolated git worktrees per attempt.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::process::Command;
use tracing::instrument;

use crate::error::{GitError, GitResult};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateWorktreeRequest {
    pub repo_root: PathBuf,
    pub worktree_root: PathBuf,
    pub attempt_short_id: String,
    pub branch_name: String,
    pub base_ref: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateWorktreeOutcome {
    pub worktree_path: PathBuf,
    pub branch_name: String,
}

async fn run_git(repo_root: &Path, args: &[&str]) -> GitResult<String> {
    let out = Command::new("git")
        .args(args)
        .current_dir(repo_root)
        .output()
        .await?;
    if !out.status.success() {
        return Err(GitError::Command {
            command: format!("git {}", args.join(" ")),
            status: out.status.code().unwrap_or(-1),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        });
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

#[instrument(skip(req), fields(attempt = %req.attempt_short_id, branch = %req.branch_name))]
pub async fn create_worktree(req: CreateWorktreeRequest) -> GitResult<CreateWorktreeOutcome> {
    tokio::fs::create_dir_all(&req.worktree_root).await?;
    let worktree_path = req
        .worktree_root
        .join(format!("attempt-{}", req.attempt_short_id));
    if worktree_path.exists() {
        return Err(GitError::Invalid(format!(
            "worktree path already exists: {}",
            worktree_path.display()
        )));
    }
    let worktree_str = worktree_path.to_string_lossy();
    run_git(
        &req.repo_root,
        &[
            "worktree",
            "add",
            "-b",
            &req.branch_name,
            &worktree_str,
            &req.base_ref,
        ],
    )
    .await?;
    Ok(CreateWorktreeOutcome {
        worktree_path,
        branch_name: req.branch_name,
    })
}

#[instrument(skip(repo_root), fields(worktree = %worktree_path.display()))]
pub async fn remove_worktree(repo_root: &Path, worktree_path: &Path) -> GitResult<()> {
    let worktree_str = worktree_path.to_string_lossy();
    run_git(repo_root, &["worktree", "remove", "--force", &worktree_str]).await?;
    Ok(())
}
