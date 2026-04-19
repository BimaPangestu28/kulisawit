#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use futures::StreamExt;
use kulisawit_db::{columns, connect, migrate, project, task};
use kulisawit_orchestrator::RuntimeConfig;
use kulisawit_server::{serve_with_shutdown_ready, ServeConfig};
use tempfile::tempdir;

fn init_repo(dir: &std::path::Path) {
    Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(dir)
        .status()
        .unwrap();
    std::fs::write(dir.join("README.md"), "# t\n").unwrap();
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
            "i",
        ])
        .current_dir(dir)
        .status()
        .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn end_to_end_dispatch_and_sse() {
    let dir = tempdir().expect("tmp");
    init_repo(dir.path());
    let db_path = dir.path().join("k.sqlite");

    let pool = connect(db_path.to_str().expect("utf8"))
        .await
        .expect("pool");
    migrate(&pool).await.expect("mig");
    let project_id = project::create(
        &pool,
        project::NewProject {
            name: "E2E".into(),
            repo_path: dir.path().display().to_string(),
        },
    )
    .await
    .expect("p");
    let cols = columns::seed_defaults(&pool, &project_id).await.expect("c");
    let task_id = task::create(
        &pool,
        task::NewTask {
            project_id,
            column_id: cols[0].clone(),
            title: "e2e".into(),
            description: None,
            tags: vec![],
            linked_files: vec![],
        },
    )
    .await
    .expect("t");
    pool.close().await;

    let shutdown = Arc::new(tokio::sync::Notify::new());
    let shutdown_clone = shutdown.clone();
    let cfg = ServeConfig {
        bind: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0),
        db_path: db_path.clone(),
        repo_root: dir.path().to_path_buf(),
        worktree_root: dir.path().join(".kulisawit/worktrees"),
        runtime: RuntimeConfig::default(),
    };
    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
    let server_handle = tokio::spawn(async move {
        serve_with_shutdown_ready(
            cfg,
            async move { shutdown_clone.notified().await },
            Some(ready_tx),
        )
        .await
    });
    let addr = tokio::time::timeout(Duration::from_secs(3), ready_rx)
        .await
        .expect("server ready")
        .expect("addr");

    let client = reqwest::Client::builder().build().expect("client");
    let base = format!("http://{addr}");

    let resp = client
        .post(format!("{base}/api/tasks/{}/dispatch", task_id.as_str()))
        .json(&serde_json::json!({"agent":"mock","batch":2}))
        .send()
        .await
        .expect("dispatch");
    assert_eq!(resp.status(), reqwest::StatusCode::OK);
    let body: serde_json::Value = resp.json().await.expect("json");
    let ids: Vec<String> = body["attempt_ids"]
        .as_array()
        .expect("ids")
        .iter()
        .map(|v| v.as_str().unwrap().to_owned())
        .collect();
    assert_eq!(ids.len(), 2);

    // Wait for both attempts to reach a terminal state via SSE before polling GET.
    // We must drain the stream until Ok(None) — the broadcaster closes *after*
    // mark_terminal, so stream-end guarantees the DB write is committed.
    for (i, attempt_id) in ids.iter().enumerate() {
        let url = format!("{base}/api/attempts/{attempt_id}/events");
        let resp = client.get(&url).send().await.expect("sse");
        assert_eq!(resp.status(), reqwest::StatusCode::OK);
        if i == 0 {
            assert_eq!(
                resp.headers()
                    .get(reqwest::header::CONTENT_TYPE)
                    .map(|v| v.to_str().unwrap_or("")),
                Some("text/event-stream")
            );
        }

        let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
        let mut bytes_stream = resp.bytes_stream();
        let mut collected = String::new();
        let mut stream_closed = false;
        while tokio::time::Instant::now() < deadline {
            match tokio::time::timeout(Duration::from_millis(500), bytes_stream.next()).await {
                Ok(Some(Ok(chunk))) => {
                    collected.push_str(&String::from_utf8_lossy(&chunk));
                }
                Ok(Some(Err(e))) => panic!("stream error: {e}"),
                Ok(None) => {
                    stream_closed = true;
                    break;
                }
                Err(_) => {
                    // timeout waiting for next chunk — check if we already saw terminal
                    if collected.contains("\"succeeded\"") {
                        // give the server a moment to close the channel
                        tokio::time::sleep(Duration::from_millis(50)).await;
                    }
                    continue;
                }
            }
        }
        assert!(
            collected.contains("\"succeeded\""),
            "SSE for attempt {i} did not see terminal Succeeded status within 10s:\n{collected}"
        );
        let _ = stream_closed; // we wait for close as a side-effect of draining above
    }

    for id in &ids {
        let resp = client
            .get(format!("{base}/api/attempts/{id}"))
            .send()
            .await
            .expect("get attempt");
        assert_eq!(resp.status(), reqwest::StatusCode::OK);
        let body: serde_json::Value = resp.json().await.expect("json");
        assert_eq!(body["status"], "completed");
    }

    shutdown.notify_one();
    tokio::time::timeout(Duration::from_secs(5), server_handle)
        .await
        .expect("drain within 5s")
        .expect("join")
        .expect("serve ok");
}
