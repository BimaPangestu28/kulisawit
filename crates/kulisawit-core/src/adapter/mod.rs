//! The `KuliAdapter` contract every agent integration implements.

use async_trait::async_trait;
use futures::stream::BoxStream;

mod event;
pub use event::{CheckResult, KuliEvent, RunContext};

use crate::error::CoreError;

/// Errors an adapter may return. Kept narrow on purpose — detail goes in the message.
#[derive(Debug, thiserror::Error)]
pub enum KuliError {
    #[error("adapter not ready: {0}")]
    NotReady(String),
    #[error("adapter failed: {0}")]
    Failed(String),
    #[error("cancelled")]
    Cancelled,
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

impl From<KuliError> for CoreError {
    fn from(value: KuliError) -> Self {
        CoreError::Adapter(value.to_string())
    }
}

#[async_trait]
pub trait KuliAdapter: Send + Sync + std::fmt::Debug {
    fn id(&self) -> &str;
    fn display_name(&self) -> &str;
    fn version(&self) -> &str;

    async fn check(&self) -> Result<CheckResult, KuliError>;

    async fn run(&self, ctx: RunContext) -> Result<BoxStream<'static, KuliEvent>, KuliError>;

    async fn cancel(&self, run_id: &str) -> Result<(), KuliError>;
}
