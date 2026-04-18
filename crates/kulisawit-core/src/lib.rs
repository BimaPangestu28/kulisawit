//! Kulisawit domain types, adapter trait, orchestrator
//!
//! See the workspace root `README.md` and `docs/PRD.md` for the product brief.

pub mod adapter;
pub mod error;
pub mod ids;
pub mod status;

pub use adapter::{AgentAdapter, AgentError, AgentEvent, CheckResult, RunContext};
pub use error::{CoreError, CoreResult};
pub use ids::{AttemptId, ColumnId, ProjectId, TaskId};
pub use status::{AttemptStatus, RunStatus, UnknownAttemptStatus, VerificationStatus};
