//! Read-only git queries via libgit2.

use git2::{Repository, StatusOptions};
use std::path::Path;

use crate::error::GitResult;

pub fn head_commit_sha(repo_path: &Path) -> GitResult<String> {
    let repo = Repository::open(repo_path)?;
    let head = repo.head()?.peel_to_commit()?;
    Ok(head.id().to_string())
}

pub fn is_clean(repo_path: &Path) -> GitResult<bool> {
    let repo = Repository::open(repo_path)?;
    let mut opts = StatusOptions::new();
    opts.include_untracked(true).recurse_untracked_dirs(true);
    let statuses = repo.statuses(Some(&mut opts))?;
    Ok(statuses.is_empty())
}
