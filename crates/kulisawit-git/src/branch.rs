//! Branch + commit operations scoped to a single worktree.

use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::process::Command;
use tracing::instrument;

use crate::error::{GitError, GitResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitSummary {
    pub changed: bool,
    pub message: String,
    pub commit_sha: Option<String>,
}

async fn git_in(worktree: &Path, args: &[&str]) -> GitResult<(i32, String, String)> {
    let out = Command::new("git")
        .args(args)
        .current_dir(worktree)
        .output()
        .await?;
    Ok((
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    ))
}

#[instrument(skip_all, fields(worktree = %worktree_path.display()))]
pub async fn commit_all_in_worktree(
    worktree_path: &Path,
    message: &str,
) -> GitResult<CommitSummary> {
    // Short-circuit if nothing changed.
    let (_code, status_out, _) = git_in(worktree_path, &["status", "--porcelain"]).await?;
    if status_out.trim().is_empty() {
        return Ok(CommitSummary {
            changed: false,
            message: message.to_owned(),
            commit_sha: None,
        });
    }

    let (code, _so, se) = git_in(worktree_path, &["add", "-A"]).await?;
    if code != 0 {
        return Err(GitError::Command {
            command: "git add -A".into(),
            status: code,
            stderr: se,
        });
    }

    let (code, _so, se) = git_in(
        worktree_path,
        &[
            "-c",
            "user.email=kulisawit@localhost",
            "-c",
            "user.name=Kulisawit Orchestrator",
            "commit",
            "-m",
            message,
        ],
    )
    .await?;
    if code != 0 {
        return Err(GitError::Command {
            command: "git commit".into(),
            status: code,
            stderr: se,
        });
    }

    let (_, sha, _) = git_in(worktree_path, &["rev-parse", "HEAD"]).await?;
    Ok(CommitSummary {
        changed: true,
        message: message.to_owned(),
        commit_sha: Some(sha.trim().to_owned()),
    })
}
