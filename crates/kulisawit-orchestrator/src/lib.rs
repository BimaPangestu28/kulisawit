//! Kulisawit orchestrator: dispatches agent attempts into isolated worktrees.
//!
//! Public surface:
//! - [`Orchestrator`] — owns shared state (DB pool, agent registry, broadcaster,
//!   semaphore, cancel flags) and exposes `dispatch_single_attempt`,
//!   `dispatch_batch`, and `cancel_attempt`.
//! - [`AgentRegistry`] — keyed lookup of `AgentAdapter` implementations.
//! - [`EventBroadcaster`] — per-attempt `tokio::sync::broadcast` channels for
//!   SSE fanout.
//! - [`RuntimeConfig`] — declarative runtime knobs loaded from
//!   `peta-kebun.toml`.
//! - [`prompt::compose_prompt`] — deterministic prompt composer from a
//!   `Task` row.

pub mod broadcaster;
pub mod config;
pub mod dispatch;
pub mod error;
pub mod orchestrator;
pub mod prompt;
pub mod registry;

pub use broadcaster::EventBroadcaster;
pub use config::RuntimeConfig;
pub use dispatch::{dispatch_batch, dispatch_single_attempt};
pub use error::{OrchestratorError, OrchestratorResult};
pub use orchestrator::Orchestrator;
pub use registry::AgentRegistry;
