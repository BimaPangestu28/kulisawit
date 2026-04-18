//! The `AgentAdapter` contract every agent integration implements.

use async_trait::async_trait;
use futures::stream::BoxStream;

mod event;
pub use event::{AgentEvent, CheckResult, RunContext};

use crate::error::CoreError;

/// Errors an adapter may return. Kept narrow on purpose — detail goes in the message.
#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("adapter not ready: {0}")]
    NotReady(String),
    #[error("adapter failed: {0}")]
    Failed(String),
    #[error("cancelled")]
    Cancelled,
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

impl From<AgentError> for CoreError {
    fn from(value: AgentError) -> Self {
        CoreError::Adapter(value.to_string())
    }
}

#[async_trait]
pub trait AgentAdapter: Send + Sync + std::fmt::Debug {
    fn id(&self) -> &str;
    fn display_name(&self) -> &str;
    fn version(&self) -> &str;

    async fn check(&self) -> Result<CheckResult, AgentError>;

    async fn run(&self, ctx: RunContext) -> Result<BoxStream<'static, AgentEvent>, AgentError>;

    async fn cancel(&self, run_id: &str) -> Result<(), AgentError>;
}
