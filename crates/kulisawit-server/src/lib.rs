//! Kulisawit HTTP + SSE server.
//!
//! Public surface:
//! - [`ServeConfig`] — declarative configuration.
//! - [`serve`] — bind, wire an `Orchestrator`, run until shutdown.

pub mod error;
pub mod state;
pub mod wire;

mod routes;

pub use error::{ServerError, ServerResult};
pub use state::{AppState, ServeConfig};

use std::net::SocketAddr;

/// Bind the HTTP server and run until a shutdown signal.
///
/// The full implementation lands in Task 3.1.3; this stub lets downstream
/// tasks write tests that call `serve` without a hanging routine.
pub async fn serve(_config: ServeConfig) -> ServerResult<SocketAddr> {
    Err(ServerError::Internal("serve not yet implemented".into()))
}
