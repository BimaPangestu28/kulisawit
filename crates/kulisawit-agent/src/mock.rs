//! A deterministic adapter used for tests and developer smoke runs.

use async_trait::async_trait;
use futures::stream::{self, BoxStream};
use kulisawit_core::{
    adapter::{AgentAdapter, AgentError, AgentEvent, CheckResult, RunContext},
    status::RunStatus,
};
use std::time::Duration;

#[derive(Debug, Default, Clone)]
pub struct MockAgent;

#[async_trait]
impl AgentAdapter for MockAgent {
    fn id(&self) -> &str {
        "mock"
    }
    fn display_name(&self) -> &str {
        "Mock Agent"
    }
    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    async fn check(&self) -> Result<CheckResult, AgentError> {
        Ok(CheckResult {
            ok: true,
            message: Some("mock ready".into()),
            version: Some("0".into()),
        })
    }

    async fn run(&self, _ctx: RunContext) -> Result<BoxStream<'static, AgentEvent>, AgentError> {
        let scripted = vec![
            AgentEvent::Status {
                status: RunStatus::Starting,
                detail: None,
            },
            AgentEvent::Stdout {
                text: "Reading repo…".into(),
            },
            AgentEvent::ToolCall {
                name: "read_file".into(),
                input: serde_json::json!({ "path": "README.md" }),
            },
            AgentEvent::ToolResult {
                name: "read_file".into(),
                output: serde_json::json!({ "bytes": 128 }),
            },
            AgentEvent::Stdout {
                text: "Drafting change…".into(),
            },
            AgentEvent::FileEdit {
                path: "src/lib.rs".into(),
                diff: Some("@@ -1 +1,2 @@\n+// mock edit\n".into()),
            },
            AgentEvent::Status {
                status: RunStatus::Succeeded,
                detail: None,
            },
        ];
        let s = stream::unfold(scripted.into_iter(), |mut it| async move {
            let next = it.next()?;
            tokio::time::sleep(Duration::from_millis(5)).await;
            Some((next, it))
        });
        Ok(Box::pin(s))
    }

    async fn cancel(&self, _run_id: &str) -> Result<(), AgentError> {
        Ok(())
    }
}
