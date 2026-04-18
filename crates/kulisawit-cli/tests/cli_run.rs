#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::process::Command;
use tempfile::tempdir;

fn bin_path() -> std::path::PathBuf {
    env!("CARGO_BIN_EXE_kulisawit").into()
}

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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn run_subcommand_dispatches_two_mock_attempts_to_completion() {
    let base = tempdir().expect("tmp");
    init_repo(base.path());
    let db_path = base.path().join("kulisawit.sqlite");

    let pool = kulisawit_db::connect(db_path.to_str().expect("utf8"))
        .await
        .expect("pool");
    kulisawit_db::migrate(&pool).await.expect("mig");
    let project_id = kulisawit_db::project::create(
        &pool,
        kulisawit_db::project::NewProject {
            name: "K".into(),
            repo_path: base.path().display().to_string(),
        },
    )
    .await
    .expect("p");
    let cols = kulisawit_db::columns::seed_defaults(&pool, &project_id)
        .await
        .expect("c");
    let task_id = kulisawit_db::task::create(
        &pool,
        kulisawit_db::task::NewTask {
            project_id,
            column_id: cols[0].clone(),
            title: "cli smoke".into(),
            description: Some("d".into()),
            tags: vec![],
            linked_files: vec![],
        },
    )
    .await
    .expect("t");
    pool.close().await;

    let out = Command::new(bin_path())
        .args([
            "run",
            "--db",
            db_path.to_str().expect("utf8"),
            "--repo",
            base.path().to_str().expect("utf8"),
            "--task",
            task_id.as_str(),
            "--agent",
            "mock",
            "--batch",
            "2",
        ])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "run exit: {:?}\nstdout:\n{}\nstderr:\n{}",
        out.status,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let pool = kulisawit_db::connect(db_path.to_str().expect("utf8"))
        .await
        .expect("pool");
    let rows = kulisawit_db::attempt::list_for_task(&pool, &task_id)
        .await
        .expect("list");
    assert_eq!(rows.len(), 2, "expected 2 attempts, got {}", rows.len());
    for r in &rows {
        assert_eq!(r.status, kulisawit_core::AttemptStatus::Completed);
    }

    let stdout = String::from_utf8_lossy(&out.stdout);
    for r in &rows {
        assert!(
            stdout.contains(r.id.as_str()),
            "stdout missing {}; got:\n{stdout}",
            r.id
        );
    }
}
