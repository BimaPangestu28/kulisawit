//! Shared HTTP handler state and server configuration.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use kulisawit_orchestrator::{Orchestrator, RuntimeConfig};

/// Declarative configuration for [`crate::serve`].
#[derive(Debug, Clone)]
pub struct ServeConfig {
    pub bind: SocketAddr,
    pub db_path: PathBuf,
    pub repo_root: PathBuf,
    pub worktree_root: PathBuf,
    pub runtime: RuntimeConfig,
}

/// State passed to every handler via `axum::extract::State`.
#[derive(Clone)]
pub struct AppState {
    pub orch: Arc<Orchestrator>,
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState").finish_non_exhaustive()
    }
}
