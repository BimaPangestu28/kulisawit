//! `kulisawit serve` — start the in-process HTTP + SSE server.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Args;

use kulisawit_orchestrator::RuntimeConfig;
use kulisawit_server::{serve, ServeConfig};

#[derive(Debug, Args)]
pub struct ServeArgs {
    /// Path to the SQLite database. Will be created if missing.
    #[arg(long)]
    pub db: PathBuf,
    /// Path to the git repository hosting dispatched tasks.
    #[arg(long)]
    pub repo: PathBuf,
    /// Worktree root; defaults to <repo>/.kulisawit/worktrees.
    #[arg(long)]
    pub worktree_root: Option<PathBuf>,
    /// Port to bind. Default 3000.
    #[arg(long, default_value_t = 3000)]
    pub port: u16,
}

pub async fn run(args: ServeArgs) -> Result<()> {
    let bind = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), args.port);
    let worktree_root = args
        .worktree_root
        .unwrap_or_else(|| args.repo.join(".kulisawit/worktrees"));
    let cfg = ServeConfig {
        bind,
        db_path: args.db,
        repo_root: args.repo,
        worktree_root,
        runtime: RuntimeConfig::default(),
    };
    let addr = serve(cfg).await.context("serve")?;
    tracing::info!(addr = %addr, "server exited");
    Ok(())
}
