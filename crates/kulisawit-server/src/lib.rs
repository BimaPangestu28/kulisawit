//! Kulisawit HTTP + SSE server.
//!
//! Public surface:
//! - [`ServeConfig`] — declarative configuration.
//! - [`serve`] — bind, wire an `Orchestrator`, run until Ctrl-C.
//! - [`serve_with_shutdown`] — same, with a caller-supplied future that
//!   completes to signal graceful shutdown. Used by integration tests.
//! - [`serve_with_shutdown_ready`] — same, plus a oneshot that fires once the
//!   listener is bound. Tests use this to learn the ephemeral port.

pub mod error;
pub mod state;
pub mod wire;

mod routes;

#[cfg(feature = "embed-ui")]
pub mod assets;

pub use error::{ServerError, ServerResult};
pub use state::{AppState, ServeConfig};

use std::net::SocketAddr;
use std::sync::Arc;

use kulisawit_agent::MockAgent;
use kulisawit_core::AgentAdapter;
use kulisawit_db::{connect, migrate};
use kulisawit_orchestrator::{AgentRegistry, Orchestrator};

/// Same as [`serve`] but accepts an external shutdown future and emits the
/// bound `SocketAddr` via a `oneshot::Sender` once the listener is ready.
///
/// Tests use this to learn the ephemeral port (bind to 0, read the actual
/// port back). Production code calls [`serve`] which doesn't care.
pub async fn serve_with_shutdown_ready<S>(
    config: ServeConfig,
    shutdown: S,
    ready_tx: Option<tokio::sync::oneshot::Sender<SocketAddr>>,
) -> ServerResult<SocketAddr>
where
    S: std::future::Future<Output = ()> + Send + 'static,
{
    let db_str = config
        .db_path
        .to_str()
        .ok_or_else(|| ServerError::InvalidInput("db_path is not valid UTF-8".into()))?
        .to_owned();
    let pool = connect(&db_str).await?;
    migrate(&pool).await?;

    let mut registry = AgentRegistry::new();
    registry.register(Arc::new(MockAgent::default()) as Arc<dyn AgentAdapter>);

    let orch = Arc::new(Orchestrator::new(
        pool,
        registry,
        config.repo_root.clone(),
        config.worktree_root.clone(),
        config.runtime.clone(),
    ));

    let state = AppState { orch };
    let app = routes::router(state);

    let listener = tokio::net::TcpListener::bind(config.bind).await?;
    let local_addr = listener.local_addr()?;
    tracing::info!(addr = %local_addr, "kulisawit server listening");
    if let Some(tx) = ready_tx {
        let _ = tx.send(local_addr);
    }

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await?;

    Ok(local_addr)
}

pub async fn serve_with_shutdown<S>(config: ServeConfig, shutdown: S) -> ServerResult<SocketAddr>
where
    S: std::future::Future<Output = ()> + Send + 'static,
{
    serve_with_shutdown_ready(config, shutdown, None).await
}

/// Bind the HTTP server and run until Ctrl-C.
pub async fn serve(config: ServeConfig) -> ServerResult<SocketAddr> {
    serve_with_shutdown(config, shutdown_signal()).await
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("shutdown signal received");
}

/// Construct the router for integration tests. Bypasses the bind/listen steps
/// of [`serve`] so a test can exercise handlers with `tower::ServiceExt::oneshot`.
pub fn routes_for_testing(state: AppState) -> axum::Router {
    routes::router(state)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod lib_tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use std::time::Duration;
    use tempfile::tempdir;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn serve_binds_and_returns_address_then_graceful_shuts_down() {
        let dir = tempdir().expect("tmp");
        let cfg = ServeConfig {
            bind: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0),
            db_path: dir.path().join("k.sqlite"),
            repo_root: dir.path().to_path_buf(),
            worktree_root: dir.path().join("wt"),
            runtime: kulisawit_orchestrator::RuntimeConfig::default(),
        };

        let shutdown = std::sync::Arc::new(tokio::sync::Notify::new());
        let shutdown_clone = shutdown.clone();
        let handle = tokio::spawn(async move {
            serve_with_shutdown(cfg, async move { shutdown_clone.notified().await }).await
        });

        tokio::time::sleep(Duration::from_millis(100)).await;
        shutdown.notify_one();

        let result = tokio::time::timeout(Duration::from_secs(5), handle)
            .await
            .expect("shutdown within 5s")
            .expect("join");
        result.expect("serve ok");
    }
}
