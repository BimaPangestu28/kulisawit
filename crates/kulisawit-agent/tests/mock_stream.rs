#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::default_constructed_unit_structs
)]

use futures::StreamExt;
use kulisawit_agent::MockAgent;
use kulisawit_core::adapter::{AgentAdapter, AgentEvent, RunContext};
use std::collections::HashMap;
use std::path::PathBuf;

fn ctx() -> RunContext {
    RunContext {
        run_id: "run-1".into(),
        worktree_path: PathBuf::from("/tmp/unused"),
        prompt: "do it".into(),
        prompt_variant: None,
        env: HashMap::new(),
    }
}

#[tokio::test]
async fn mock_check_reports_ok() {
    let k = MockAgent::default();
    let res = k.check().await.expect("check");
    assert!(res.ok);
}

#[tokio::test]
async fn mock_run_emits_scripted_sequence_ending_in_status_succeeded() {
    let k = MockAgent::default();
    let mut stream = k.run(ctx()).await.expect("run");
    let mut events = vec![];
    while let Some(evt) = stream.next().await {
        events.push(evt);
    }
    assert!(!events.is_empty());
    match events.last().expect("at least one") {
        AgentEvent::Status { status, .. } => assert!(matches!(
            status,
            kulisawit_core::status::RunStatus::Succeeded
        )),
        other => panic!("expected terminal Status event, got {other:?}"),
    }
    // Contains at least one tool_call and one file_edit.
    assert!(events
        .iter()
        .any(|e| matches!(e, AgentEvent::ToolCall { .. })));
    assert!(events
        .iter()
        .any(|e| matches!(e, AgentEvent::FileEdit { .. })));
}

#[tokio::test]
async fn mock_id_and_display_name_are_stable() {
    let k = MockAgent::default();
    assert_eq!(k.id(), "mock");
    assert_eq!(k.display_name(), "Mock Agent");
}

#[tokio::test]
async fn mock_failing_ends_with_status_failed() {
    let k = MockAgent::failing();
    let mut stream = k.run(ctx()).await.expect("run");
    let mut events = vec![];
    while let Some(evt) = stream.next().await {
        events.push(evt);
    }
    match events.last().expect("at least one") {
        AgentEvent::Status { status, .. } => {
            assert!(matches!(status, kulisawit_core::status::RunStatus::Failed))
        }
        other => panic!("expected terminal Failed status, got {other:?}"),
    }
}

#[tokio::test]
async fn mock_cancelling_ends_with_status_cancelled() {
    let k = MockAgent::cancelling();
    let mut stream = k.run(ctx()).await.expect("run");
    let mut events = vec![];
    while let Some(evt) = stream.next().await {
        events.push(evt);
    }
    match events.last().expect("at least one") {
        AgentEvent::Status { status, .. } => assert!(matches!(
            status,
            kulisawit_core::status::RunStatus::Cancelled
        )),
        other => panic!("expected terminal Cancelled status, got {other:?}"),
    }
}
