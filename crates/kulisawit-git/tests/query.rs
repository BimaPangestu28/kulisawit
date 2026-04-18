#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kulisawit_git::query::{head_commit_sha, is_clean};
use std::process::Command;
use tempfile::tempdir;

fn init(dir: &std::path::Path) {
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

#[test]
fn head_and_clean_on_fresh_repo() {
    let t = tempdir().unwrap();
    init(t.path());
    let sha = head_commit_sha(t.path()).unwrap();
    assert_eq!(sha.len(), 40);
    assert!(is_clean(t.path()).unwrap());
}

#[test]
fn dirty_when_untracked_file_added() {
    let t = tempdir().unwrap();
    init(t.path());
    std::fs::write(t.path().join("new.txt"), "hi").unwrap();
    assert!(!is_clean(t.path()).unwrap());
}
