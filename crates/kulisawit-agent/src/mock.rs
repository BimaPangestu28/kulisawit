//! A deterministic adapter used for tests and developer smoke runs.

use async_trait::async_trait;
use futures::stream::{self, BoxStream};
use kulisawit_core::{
    adapter::{AgentAdapter, AgentError, AgentEvent, CheckResult, RunContext},
    status::RunStatus,
};
use std::time::Duration;

/// Controls MockAgent behaviour. Useful for exercising orchestrator code paths.
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq)]
pub enum MockMode {
    /// Emit a scripted happy-path sequence ending in `RunStatus::Succeeded`.
    #[default]
    Succeed,
    /// Emit a short scripted sequence ending in `RunStatus::Failed`.
    Fail,
    /// Emit a short scripted sequence ending in `RunStatus::Cancelled`.
    Cancel,
    /// Emit an event every 100ms for 10 seconds (for cancellation-mid-run tests).
    Slow,
}

#[derive(Debug, Default, Clone)]
pub struct MockAgent {
    mode: MockMode,
}

impl MockAgent {
    pub fn new(mode: MockMode) -> Self {
        Self { mode }
    }

    pub fn failing() -> Self {
        Self::new(MockMode::Fail)
    }

    pub fn cancelling() -> Self {
        Self::new(MockMode::Cancel)
    }

    pub fn slow() -> Self {
        Self::new(MockMode::Slow)
    }

    fn scripted_events(&self) -> Vec<AgentEvent> {
        match self.mode {
            MockMode::Succeed => vec![
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
            ],
            MockMode::Fail => vec![
                AgentEvent::Status {
                    status: RunStatus::Starting,
                    detail: None,
                },
                AgentEvent::Stdout {
                    text: "Attempting…".into(),
                },
                AgentEvent::Stderr {
                    text: "simulated failure".into(),
                },
                AgentEvent::Status {
                    status: RunStatus::Failed,
                    detail: Some("mock failure".into()),
                },
            ],
            MockMode::Cancel => vec![
                AgentEvent::Status {
                    status: RunStatus::Starting,
                    detail: None,
                },
                AgentEvent::Stdout {
                    text: "Attempting…".into(),
                },
                AgentEvent::Status {
                    status: RunStatus::Cancelled,
                    detail: Some("mock cancelled".into()),
                },
            ],
            MockMode::Slow => {
                // 100 tick events (tick_N stdout), roughly 10s if the stream
                // consumer waits the 100ms delay between emits.
                let mut v = Vec::with_capacity(101);
                v.push(AgentEvent::Status {
                    status: RunStatus::Starting,
                    detail: None,
                });
                for i in 0..100 {
                    v.push(AgentEvent::Stdout {
                        text: format!("tick {i}"),
                    });
                }
                v.push(AgentEvent::Status {
                    status: RunStatus::Succeeded,
                    detail: None,
                });
                v
            }
        }
    }

    fn emit_delay(&self) -> Duration {
        match self.mode {
            MockMode::Slow => Duration::from_millis(100),
            _ => Duration::from_millis(5),
        }
    }
}

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
        let events = self.scripted_events();
        let delay = self.emit_delay();
        let s = stream::unfold(events.into_iter(), move |mut it| async move {
            let next = it.next()?;
            tokio::time::sleep(delay).await;
            Some((next, it))
        });
        Ok(Box::pin(s))
    }

    async fn cancel(&self, _run_id: &str) -> Result<(), AgentError> {
        Ok(())
    }
}
