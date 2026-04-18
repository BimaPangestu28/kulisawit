//! `AppState` and `ServeConfig` — placeholders; Task 3.1.3 replaces.

use std::net::SocketAddr;
use std::path::PathBuf;

use kulisawit_orchestrator::RuntimeConfig;

#[derive(Debug, Clone)]
pub struct ServeConfig {
    pub bind: SocketAddr,
    pub db_path: PathBuf,
    pub repo_root: PathBuf,
    pub worktree_root: PathBuf,
    pub runtime: RuntimeConfig,
}

#[derive(Debug, Default)]
pub struct AppState;
