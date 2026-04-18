# Kulisawit Phase 3.1 — Server + SSE Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an in-process HTTP server with SSE event streaming to Kulisawit, exposing a minimal REST surface so a browser-based UI (Phase 3.2) can create projects/tasks, dispatch attempts, and consume live agent event streams.

**Architecture:** A new `kulisawit-server` library crate owns an `Arc<Orchestrator>` and composes `axum::Router` routes grouped by resource. A new `kulisawit serve` CLI subcommand starts the server in-process; the existing `kulisawit run` headless entry point is unchanged. The orchestrator gains one new public function (`dispatch_batch_spawned`) so the server can return `AttemptId`s immediately and let the agent runs proceed in background `tokio::spawn` tasks. SSE streams are fanned out via the existing `EventBroadcaster` added in Phase 2.1.3.

**Tech Stack:** Rust 1.86, Tokio multi-thread, `axum 0.7` (features `macros`, `json`, `tokio`), `tower-http 0.6` (trace layer only; no CORS in 3.1), `reqwest 0.12` for end-to-end tests, `serde` JSON with `snake_case` fields, graceful shutdown via `axum::serve(...).with_graceful_shutdown`.

**Prerequisites:** Phase 2 complete (tag `phase-2` at `3475031`). Current HEAD `992cc20` with the Phase 3.1 design spec at `docs/superpowers/specs/2026-04-18-kulisawit-phase-3.1-server-sse-design.md`. Rust 1.86, 87 tests passing baseline.

---

## Conventions

This plan inherits every convention from Phase 1 and Phase 2 plans:
- Conventional Commits; no `Co-Authored-By:` trailers in any commit (repo standing rule).
- `thiserror` at library boundaries; `anyhow` only in the CLI binary.
- `tracing::instrument(skip(state))` on handlers that take `AppState`.
- Workspace `[lints]` denies `unwrap_used` / `expect_used` / `panic`; tests use module-level or file-level `#[allow(...)]`.
- English generic identifiers everywhere in code; Kulisawit domain vocabulary reserved for user-facing strings (none introduced in Phase 3.1).
- `sqlx::query!` offline metadata committed when new SQL is added (no new SQL expected in 3.1 — Phase 3.1 reuses existing `kulisawit-db` functions).
- Imports from `kulisawit-core` use crate-root paths (`kulisawit_core::AgentEvent`, not `kulisawit_core::adapter::AgentEvent`) — adapter types were promoted in Task 2.0.1.

---

## File Structure (end of Phase 3.1)

```
kulisawit/
├── Cargo.toml                                  # + reqwest (dev-dep workspace entry)
├── crates/
│   ├── kulisawit-orchestrator/                 # (existing)
│   │   └── src/
│   │       └── dispatch.rs                     # modified: adds dispatch_batch_spawned + helper
│   ├── kulisawit-server/                       # (currently stub — fleshed out)
│   │   ├── Cargo.toml                          # full rewrite with deps
│   │   ├── src/
│   │   │   ├── lib.rs                          # serve() entry point
│   │   │   ├── state.rs                        # AppState { orch: Arc<Orchestrator> }
│   │   │   ├── error.rs                        # ServerError + IntoResponse
│   │   │   ├── wire.rs                         # serde DTOs
│   │   │   └── routes/
│   │   │       ├── mod.rs                      # composes Router
│   │   │       ├── projects.rs
│   │   │       ├── tasks.rs
│   │   │       └── attempts.rs                 # attempt GET + SSE
│   │   └── tests/
│   │       ├── projects.rs                     # router-level tests
│   │       ├── tasks.rs                        # router-level tests
│   │       ├── dispatch.rs                     # router-level tests
│   │       ├── attempts.rs                     # router-level tests
│   │       ├── sse.rs                          # SSE lifecycle tests
│   │       └── e2e.rs                          # reqwest end-to-end
│   └── kulisawit-cli/                          # (existing)
│       ├── Cargo.toml                          # + kulisawit-server dep
│       └── src/
│           ├── main.rs                         # adds `Serve(ServeArgs)` variant to Command enum
│           └── commands/
│               ├── mod.rs                      # + pub mod serve;
│               └── serve.rs                    # new
```

---

## Exit Criteria (tag `phase-3.1`)

Before tagging `phase-3.1`, all of the following must hold on a clean checkout:

- `cargo test --workspace --locked` → 100% pass. Target ~98 tests (87 baseline + 11 new).
- `cargo clippy --workspace --all-targets --locked -- -D warnings` → zero warnings.
- `cargo fmt --check` → clean.
- `cargo build --workspace --all-targets --locked` → clean.
- Phase 2 tests unchanged: `cargo test -p kulisawit-cli --test cli_run --locked` still green (no regression).
- Manual smoke: `kulisawit serve --db /tmp/k.sqlite --repo <repo>` starts cleanly, `curl -N http://127.0.0.1:3000/api/attempts/<id>/events` shows live events after dispatch, Ctrl-C drains within 5 s.

---

## Task list (13 tasks)

- Task 3.1.1 — Scaffold `kulisawit-server` crate (manifest + module stubs + workspace deps).
- Task 3.1.2 — `ServerError` with `IntoResponse`.
- Task 3.1.3 — `AppState` + `ServeConfig` + `serve()` entry point.
- Task 3.1.4 — Wire DTOs (`wire.rs`).
- Task 3.1.5 — `POST /api/projects` + `GET /api/projects/:id`.
- Task 3.1.6 — `POST /api/tasks` + `GET /api/tasks/:id` with pre-validation.
- Task 3.1.7 — `dispatch_batch_spawned` in orchestrator.
- Task 3.1.8 — `POST /api/tasks/:id/dispatch`.
- Task 3.1.9 — `GET /api/attempts/:id`.
- Task 3.1.10 — `GET /api/attempts/:id/events` (SSE).
- Task 3.1.11 — CLI `serve` subcommand.
- Task 3.1.12 — End-to-end smoke test with `reqwest`.
- Task 3.1.13 — Tag `phase-3.1`.

---

### Task 3.1.1: Scaffold `kulisawit-server` crate

**Files:**
- Modify: `Cargo.toml` (workspace root — add `reqwest` to `[workspace.dependencies]`)
- Modify: `crates/kulisawit-server/Cargo.toml` (full rewrite)
- Modify: `crates/kulisawit-server/src/lib.rs`
- Create: `crates/kulisawit-server/src/state.rs`
- Create: `crates/kulisawit-server/src/error.rs`
- Create: `crates/kulisawit-server/src/wire.rs`
- Create: `crates/kulisawit-server/src/routes/mod.rs`
- Create: `crates/kulisawit-server/src/routes/projects.rs`
- Create: `crates/kulisawit-server/src/routes/tasks.rs`
- Create: `crates/kulisawit-server/src/routes/attempts.rs`

- [ ] **Step 1: Add `reqwest` to workspace dependencies**

In root `Cargo.toml` under `[workspace.dependencies]`, add after the existing `axum` block:

```toml
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls", "stream"] }
```

The `default-features = false` drop avoids pulling openssl; `rustls-tls` keeps us pure-Rust. `stream` enables `.bytes_stream()` for SSE consumption.

- [ ] **Step 2: Rewrite `crates/kulisawit-server/Cargo.toml`**

Replace the entire file with:

```toml
[package]
name = "kulisawit-server"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
authors.workspace = true
description = "Axum HTTP + SSE server for Kulisawit"

[lib]

[dependencies]
anyhow.workspace = true
axum.workspace = true
chrono.workspace = true
futures.workspace = true
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true
tokio = { workspace = true, features = ["full"] }
tower.workspace = true
tower-http.workspace = true
tracing.workspace = true
kulisawit-core.workspace = true
kulisawit-db.workspace = true
kulisawit-agent.workspace = true
kulisawit-orchestrator.workspace = true

[dev-dependencies]
reqwest.workspace = true
tempfile.workspace = true

[lints]
workspace = true
```

- [ ] **Step 3: Scaffold module stubs**

Replace `crates/kulisawit-server/src/lib.rs`:

```rust
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
```

Create `crates/kulisawit-server/src/error.rs`:

```rust
//! `ServerError` — placeholder; Task 3.1.2 replaces.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ServerError {
    #[error("internal: {0}")]
    Internal(String),
}

pub type ServerResult<T> = Result<T, ServerError>;
```

Create `crates/kulisawit-server/src/state.rs`:

```rust
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
```

Create `crates/kulisawit-server/src/wire.rs`:

```rust
//! JSON wire DTOs — placeholder; Task 3.1.4 replaces.
```

Create `crates/kulisawit-server/src/routes/mod.rs`:

```rust
//! Router composition — placeholder; later tasks flesh out.

pub mod attempts;
pub mod projects;
pub mod tasks;
```

Create `crates/kulisawit-server/src/routes/projects.rs`:

```rust
//! `/api/projects` — implemented in Task 3.1.5.
```

Create `crates/kulisawit-server/src/routes/tasks.rs`:

```rust
//! `/api/tasks` + dispatch — implemented in Tasks 3.1.6 and 3.1.8.
```

Create `crates/kulisawit-server/src/routes/attempts.rs`:

```rust
//! `/api/attempts` + SSE — implemented in Tasks 3.1.9 and 3.1.10.
```

- [ ] **Step 4: Verify build**

Run:

```bash
cargo build -p kulisawit-server --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --check
```

Expected: all clean.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml crates/kulisawit-server/
git commit -m "feat(server): scaffold kulisawit-server crate"
```

---

### Task 3.1.2: `ServerError` with `IntoResponse`

**Files:**
- Modify: `crates/kulisawit-server/src/error.rs`

- [ ] **Step 1: Write the failing test (inline)**

Append to the existing `error.rs` content (keep the placeholder enum above; tests compile against the final enum in Step 3):

```rust
#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use axum::response::IntoResponse;
    use http_body_util::BodyExt;

    async fn body_str(resp: axum::response::Response) -> String {
        let bytes = resp.into_body().collect().await.expect("body").to_bytes();
        String::from_utf8(bytes.to_vec()).expect("utf8")
    }

    #[tokio::test]
    async fn not_found_maps_to_404_with_json_body() {
        let err = ServerError::NotFound {
            entity: "task",
            id: "abc".into(),
        };
        let resp = err.into_response();
        assert_eq!(resp.status(), axum::http::StatusCode::NOT_FOUND);
        let body = body_str(resp).await;
        assert!(body.contains("\"not_found\""));
        assert!(body.contains("\"task\""));
        assert!(body.contains("\"abc\""));
    }

    #[tokio::test]
    async fn invalid_input_maps_to_400() {
        let err = ServerError::InvalidInput("bad batch".into());
        assert_eq!(err.into_response().status(), axum::http::StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn internal_maps_to_500_without_leaking_detail() {
        let err = ServerError::Internal("secret db url leak".into());
        let resp = err.into_response();
        assert_eq!(resp.status(), axum::http::StatusCode::INTERNAL_SERVER_ERROR);
        let body = body_str(resp).await;
        assert!(!body.contains("secret"), "500 body must not leak detail: {body}");
        assert!(body.contains("\"internal\""));
    }
}
```

Also add `http-body-util` to `crates/kulisawit-server/Cargo.toml` under `[dev-dependencies]`:

```toml
[dev-dependencies]
http-body-util = "0.1"
reqwest.workspace = true
tempfile.workspace = true
```

- [ ] **Step 2: Run to verify it fails**

```bash
cargo test -p kulisawit-server --lib error::tests --locked
```

Expected: compile error — the placeholder `ServerError` only has `Internal`, no `NotFound` or `InvalidInput`, and no `IntoResponse` impl.

- [ ] **Step 3: Implement**

Replace the contents of `crates/kulisawit-server/src/error.rs`:

```rust
//! Server-level error type and HTTP mapping.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;
use thiserror::Error;

use kulisawit_db::DbError;
use kulisawit_orchestrator::OrchestratorError;

#[derive(Debug, Error)]
pub enum ServerError {
    #[error("not found: {entity} {id}")]
    NotFound { entity: &'static str, id: String },

    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("internal: {0}")]
    Internal(String),
}

pub type ServerResult<T> = Result<T, ServerError>;

impl From<DbError> for ServerError {
    fn from(e: DbError) -> Self {
        Self::Internal(format!("db: {e}"))
    }
}

impl From<OrchestratorError> for ServerError {
    fn from(e: OrchestratorError) -> Self {
        Self::Internal(format!("orchestrator: {e}"))
    }
}

impl From<std::io::Error> for ServerError {
    fn from(e: std::io::Error) -> Self {
        Self::Internal(format!("io: {e}"))
    }
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        match self {
            ServerError::NotFound { entity, id } => (
                StatusCode::NOT_FOUND,
                Json(json!({
                    "error": "not_found",
                    "entity": entity,
                    "id": id,
                })),
            )
                .into_response(),
            ServerError::InvalidInput(message) => (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "invalid_input",
                    "message": message,
                })),
            )
                .into_response(),
            ServerError::Conflict(message) => (
                StatusCode::CONFLICT,
                Json(json!({
                    "error": "conflict",
                    "message": message,
                })),
            )
                .into_response(),
            ServerError::Internal(detail) => {
                tracing::error!(detail, "server internal error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": "internal" })),
                )
                    .into_response()
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use axum::response::IntoResponse;
    use http_body_util::BodyExt;

    async fn body_str(resp: axum::response::Response) -> String {
        let bytes = resp.into_body().collect().await.expect("body").to_bytes();
        String::from_utf8(bytes.to_vec()).expect("utf8")
    }

    #[tokio::test]
    async fn not_found_maps_to_404_with_json_body() {
        let err = ServerError::NotFound {
            entity: "task",
            id: "abc".into(),
        };
        let resp = err.into_response();
        assert_eq!(resp.status(), axum::http::StatusCode::NOT_FOUND);
        let body = body_str(resp).await;
        assert!(body.contains("\"not_found\""));
        assert!(body.contains("\"task\""));
        assert!(body.contains("\"abc\""));
    }

    #[tokio::test]
    async fn invalid_input_maps_to_400() {
        let err = ServerError::InvalidInput("bad batch".into());
        assert_eq!(err.into_response().status(), axum::http::StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn internal_maps_to_500_without_leaking_detail() {
        let err = ServerError::Internal("secret db url leak".into());
        let resp = err.into_response();
        assert_eq!(resp.status(), axum::http::StatusCode::INTERNAL_SERVER_ERROR);
        let body = body_str(resp).await;
        assert!(!body.contains("secret"), "500 body must not leak detail: {body}");
        assert!(body.contains("\"internal\""));
    }
}
```

- [ ] **Step 4: Run to verify it passes**

```bash
cargo test -p kulisawit-server --lib error::tests --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --check
```

Expected: 3 tests pass, clippy + fmt clean.

- [ ] **Step 5: Commit**

```bash
git add crates/kulisawit-server/src/error.rs crates/kulisawit-server/Cargo.toml
git commit -m "feat(server): ServerError with IntoResponse"
```

---

### Task 3.1.3: `AppState` + `ServeConfig` + `serve()` entry point

**Files:**
- Modify: `crates/kulisawit-server/src/state.rs`
- Modify: `crates/kulisawit-server/src/lib.rs`

- [ ] **Step 1: Write the failing test (in `lib.rs` inline)**

Append to `crates/kulisawit-server/src/lib.rs`:

```rust
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

        // Launch serve in background. It should bind immediately and stay up
        // until we drop the shutdown notifier.
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
```

- [ ] **Step 2: Run to verify it fails**

```bash
cargo test -p kulisawit-server --lib lib_tests --locked
```

Expected: compile error — `serve_with_shutdown` doesn't exist, `ServeConfig` is a placeholder without a real `AppState`.

- [ ] **Step 3: Implement `state.rs`**

Replace `crates/kulisawit-server/src/state.rs`:

```rust
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
```

- [ ] **Step 4: Implement `lib.rs`**

Replace `crates/kulisawit-server/src/lib.rs`:

```rust
//! Kulisawit HTTP + SSE server.
//!
//! Public surface:
//! - [`ServeConfig`] — declarative configuration.
//! - [`serve`] — bind, wire an `Orchestrator`, run until Ctrl-C.
//! - [`serve_with_shutdown`] — same, with a caller-supplied future that
//!   completes to signal graceful shutdown. Used by integration tests.

pub mod error;
pub mod state;
pub mod wire;

mod routes;

pub use error::{ServerError, ServerResult};
pub use state::{AppState, ServeConfig};

use std::net::SocketAddr;
use std::sync::Arc;

use kulisawit_agent::MockAgent;
use kulisawit_core::AgentAdapter;
use kulisawit_db::{connect, migrate};
use kulisawit_orchestrator::{AgentRegistry, Orchestrator};

/// Bind the HTTP server and run until Ctrl-C.
pub async fn serve(config: ServeConfig) -> ServerResult<SocketAddr> {
    serve_with_shutdown(config, shutdown_signal()).await
}

/// Same as [`serve`] but accepts an external shutdown future.
pub async fn serve_with_shutdown<S>(config: ServeConfig, shutdown: S) -> ServerResult<SocketAddr>
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

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await?;

    Ok(local_addr)
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("shutdown signal received");
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
```

- [ ] **Step 5: Add an empty router stub in `routes/mod.rs`**

Replace `crates/kulisawit-server/src/routes/mod.rs`:

```rust
//! Router composition.
//!
//! Subsequent tasks hang endpoint groups off the root router. For Task 3.1.3
//! the router exists only so `axum::serve` has something to serve.

pub mod attempts;
pub mod projects;
pub mod tasks;

use axum::Router;

use crate::AppState;

pub fn router(state: AppState) -> Router {
    Router::new().with_state(state)
}
```

- [ ] **Step 6: Run to verify**

```bash
cargo test -p kulisawit-server --lib lib_tests --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --check
```

Expected: 1 test pass, clippy + fmt clean.

- [ ] **Step 7: Commit**

```bash
git add crates/kulisawit-server/src/
git commit -m "feat(server): AppState and serve entry point with graceful shutdown"
```

---

### Task 3.1.4: Wire DTOs

**Files:**
- Modify: `crates/kulisawit-server/src/wire.rs`

- [ ] **Step 1: Write the failing test (inline)**

Append to `crates/kulisawit-server/src/wire.rs`:

```rust
#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use kulisawit_core::{AttemptId, AttemptStatus, ProjectId, TaskId};

    #[test]
    fn project_response_serializes_snake_case() {
        let r = ProjectResponse {
            id: ProjectId::new(),
            name: "Demo".into(),
            repo_path: "/tmp/demo".into(),
            created_at: 1_700_000_000_000,
        };
        let json = serde_json::to_string(&r).expect("ser");
        assert!(json.contains("\"repo_path\""));
        assert!(json.contains("\"created_at\":1700000000000"));
    }

    #[test]
    fn attempt_response_omits_verification_fields() {
        let r = AttemptResponse {
            id: AttemptId::new(),
            task_id: TaskId::new(),
            agent_id: "mock".into(),
            status: AttemptStatus::Queued,
            prompt_variant: None,
            worktree_path: "/tmp/wt".into(),
            branch_name: "b".into(),
            started_at: None,
            completed_at: None,
        };
        let json = serde_json::to_string(&r).expect("ser");
        assert!(!json.contains("verification"), "no verification fields: {json}");
    }

    #[test]
    fn dispatch_request_accepts_no_variants() {
        let body = r#"{"agent":"mock","batch":3}"#;
        let r: DispatchRequest = serde_json::from_str(body).expect("de");
        assert_eq!(r.agent, "mock");
        assert_eq!(r.batch, 3);
        assert!(r.variants.is_none());
    }
}
```

- [ ] **Step 2: Run to verify it fails**

```bash
cargo test -p kulisawit-server --lib wire::tests --locked
```

Expected: compile error — all DTOs are absent (the file is just a doc comment).

- [ ] **Step 3: Implement**

Replace `crates/kulisawit-server/src/wire.rs`:

```rust
//! JSON wire DTOs.
//!
//! All fields are `snake_case`. Responses are thin re-serializations of the
//! DB-layer structs; no additional projection logic lives here. Requests
//! mirror the `New*` struct shapes from `kulisawit-db`.

use serde::{Deserialize, Serialize};

use kulisawit_core::{AgentEvent, AttemptId, AttemptStatus, ColumnId, ProjectId, TaskId};

// ---- Requests ----

#[derive(Debug, Deserialize)]
pub struct NewProjectRequest {
    pub name: String,
    pub repo_path: String,
}

#[derive(Debug, Deserialize)]
pub struct NewTaskRequest {
    pub project_id: ProjectId,
    pub column_id: ColumnId,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub linked_files: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct DispatchRequest {
    pub agent: String,
    pub batch: usize,
    #[serde(default)]
    pub variants: Option<Vec<String>>,
}

// ---- Responses ----

#[derive(Debug, Serialize)]
pub struct ProjectResponse {
    pub id: ProjectId,
    pub name: String,
    pub repo_path: String,
    pub created_at: i64,
}

#[derive(Debug, Serialize)]
pub struct TaskResponse {
    pub id: TaskId,
    pub project_id: ProjectId,
    pub column_id: ColumnId,
    pub title: String,
    pub description: Option<String>,
    pub position: i64,
    pub tags: Vec<String>,
    pub linked_files: Vec<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Serialize)]
pub struct AttemptResponse {
    pub id: AttemptId,
    pub task_id: TaskId,
    pub agent_id: String,
    pub status: AttemptStatus,
    pub prompt_variant: Option<String>,
    pub worktree_path: String,
    pub branch_name: String,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct DispatchResponse {
    pub attempt_ids: Vec<AttemptId>,
}

#[derive(Debug, Serialize)]
pub struct EventEnvelope {
    pub attempt_id: AttemptId,
    pub event: AgentEvent,
    pub ts_ms: i64,
}

// ---- Conversions from DB structs ----

impl From<kulisawit_db::project::Project> for ProjectResponse {
    fn from(p: kulisawit_db::project::Project) -> Self {
        Self {
            id: p.id,
            name: p.name,
            repo_path: p.repo_path,
            created_at: p.created_at,
        }
    }
}

impl From<kulisawit_db::task::Task> for TaskResponse {
    fn from(t: kulisawit_db::task::Task) -> Self {
        Self {
            id: t.id,
            project_id: t.project_id,
            column_id: t.column_id,
            title: t.title,
            description: t.description,
            position: t.position,
            tags: t.tags,
            linked_files: t.linked_files,
            created_at: t.created_at,
            updated_at: t.updated_at,
        }
    }
}

impl From<kulisawit_db::attempt::Attempt> for AttemptResponse {
    fn from(a: kulisawit_db::attempt::Attempt) -> Self {
        Self {
            id: a.id,
            task_id: a.task_id,
            agent_id: a.agent_id,
            status: a.status,
            prompt_variant: a.prompt_variant,
            worktree_path: a.worktree_path,
            branch_name: a.branch_name,
            started_at: a.started_at,
            completed_at: a.completed_at,
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use kulisawit_core::{AttemptId, AttemptStatus, ProjectId, TaskId};

    #[test]
    fn project_response_serializes_snake_case() {
        let r = ProjectResponse {
            id: ProjectId::new(),
            name: "Demo".into(),
            repo_path: "/tmp/demo".into(),
            created_at: 1_700_000_000_000,
        };
        let json = serde_json::to_string(&r).expect("ser");
        assert!(json.contains("\"repo_path\""));
        assert!(json.contains("\"created_at\":1700000000000"));
    }

    #[test]
    fn attempt_response_omits_verification_fields() {
        let r = AttemptResponse {
            id: AttemptId::new(),
            task_id: TaskId::new(),
            agent_id: "mock".into(),
            status: AttemptStatus::Queued,
            prompt_variant: None,
            worktree_path: "/tmp/wt".into(),
            branch_name: "b".into(),
            started_at: None,
            completed_at: None,
        };
        let json = serde_json::to_string(&r).expect("ser");
        assert!(!json.contains("verification"), "no verification fields: {json}");
    }

    #[test]
    fn dispatch_request_accepts_no_variants() {
        let body = r#"{"agent":"mock","batch":3}"#;
        let r: DispatchRequest = serde_json::from_str(body).expect("de");
        assert_eq!(r.agent, "mock");
        assert_eq!(r.batch, 3);
        assert!(r.variants.is_none());
    }
}
```

- [ ] **Step 4: Run to verify**

```bash
cargo test -p kulisawit-server --lib wire::tests --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --check
```

Expected: 3 tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/kulisawit-server/src/wire.rs
git commit -m "feat(server): wire DTOs for projects/tasks/attempts/dispatch/SSE"
```

---

### Task 3.1.5: `POST /api/projects` + `GET /api/projects/:id`

**Files:**
- Modify: `crates/kulisawit-server/src/routes/projects.rs`
- Modify: `crates/kulisawit-server/src/routes/mod.rs`
- Create: `crates/kulisawit-server/tests/projects.rs`

- [ ] **Step 1: Write the failing integration tests**

Create `crates/kulisawit-server/tests/projects.rs`:

```rust
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use http_body_util::BodyExt;
use kulisawit_agent::MockAgent;
use kulisawit_core::AgentAdapter;
use kulisawit_db::{connect, migrate};
use kulisawit_orchestrator::{AgentRegistry, Orchestrator, RuntimeConfig};
use kulisawit_server::{routes_for_testing, AppState};
use tempfile::tempdir;
use tower::ServiceExt;

async fn fresh_app() -> axum::Router {
    let pool = connect("sqlite::memory:").await.expect("pool");
    migrate(&pool).await.expect("mig");
    let mut registry = AgentRegistry::new();
    registry.register(Arc::new(MockAgent::default()) as Arc<dyn AgentAdapter>);
    let dir = tempdir().expect("tmp");
    let orch = Arc::new(Orchestrator::new(
        pool,
        registry,
        dir.path().to_path_buf(),
        dir.path().join("wt"),
        RuntimeConfig::default(),
    ));
    // Keep `dir` alive via leak — acceptable in tests.
    std::mem::forget(dir);
    routes_for_testing(AppState { orch })
}

#[tokio::test]
async fn post_projects_with_valid_body_returns_200_and_inserts() {
    let app = fresh_app().await;
    let body = r#"{"name":"Demo","repo_path":"/tmp/demo"}"#;
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/projects")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["name"], "Demo");
    assert_eq!(json["repo_path"], "/tmp/demo");
    assert!(json["id"].is_string());
    assert!(json["created_at"].is_i64());
}

#[tokio::test]
async fn post_projects_with_missing_name_returns_400() {
    let app = fresh_app().await;
    let body = r#"{"repo_path":"/tmp/demo"}"#;
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/projects")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    // axum's Json extractor returns 422 or 400 depending on version; axum 0.7
    // returns 422 for missing fields. Accept either as "client error".
    assert!(
        resp.status().is_client_error(),
        "expected 4xx, got {}",
        resp.status()
    );
}

#[tokio::test]
async fn get_project_by_unknown_id_returns_404() {
    let app = fresh_app().await;
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/projects/01900000-0000-0000-0000-000000000000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
```

Also add a `pub fn routes_for_testing` to `lib.rs` so integration tests can build a router without calling `serve`.

Append to `crates/kulisawit-server/src/lib.rs` (before `#[cfg(test)]`):

```rust
/// Construct the router for integration tests. Bypasses the bind/listen steps
/// of [`serve`] so a test can exercise handlers with `tower::ServiceExt::oneshot`.
pub fn routes_for_testing(state: AppState) -> axum::Router {
    routes::router(state)
}
```

- [ ] **Step 2: Run to verify it fails**

```bash
cargo test -p kulisawit-server --test projects --locked
```

Expected: compile error — `routes_for_testing` may exist (from above step) but the router has no `/api/projects` routes yet.

- [ ] **Step 3: Implement `routes/projects.rs`**

Replace `crates/kulisawit-server/src/routes/projects.rs`:

```rust
//! `/api/projects` endpoints.

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};

use kulisawit_core::ProjectId;
use kulisawit_db::project::{self, NewProject};

use crate::wire::{NewProjectRequest, ProjectResponse};
use crate::{AppState, ServerError, ServerResult};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/projects", post(create))
        .route("/api/projects/:id", get(get_by_id))
}

async fn create(
    State(state): State<AppState>,
    Json(req): Json<NewProjectRequest>,
) -> ServerResult<Json<ProjectResponse>> {
    let id = project::create(
        state.orch.pool(),
        NewProject {
            name: req.name.clone(),
            repo_path: req.repo_path.clone(),
        },
    )
    .await?;
    let row = project::get(state.orch.pool(), &id)
        .await?
        .ok_or_else(|| ServerError::Internal("project vanished after insert".into()))?;
    Ok(Json(row.into()))
}

async fn get_by_id(
    State(state): State<AppState>,
    Path(id): Path<ProjectId>,
) -> ServerResult<Json<ProjectResponse>> {
    let row = project::get(state.orch.pool(), &id)
        .await?
        .ok_or_else(|| ServerError::NotFound {
            entity: "project",
            id: id.as_str().to_owned(),
        })?;
    Ok(Json(row.into()))
}
```

- [ ] **Step 4: Wire into `routes/mod.rs`**

Replace `crates/kulisawit-server/src/routes/mod.rs`:

```rust
//! Router composition.

pub mod attempts;
pub mod projects;
pub mod tasks;

use axum::Router;

use crate::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .merge(projects::routes())
        .with_state(state)
}
```

- [ ] **Step 5: Run to verify it passes**

```bash
cargo test -p kulisawit-server --test projects --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --check
```

Expected: 3 tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/kulisawit-server/
git commit -m "feat(server): POST and GET /api/projects"
```

---

### Task 3.1.6: `POST /api/tasks` + `GET /api/tasks/:id` with pre-validation

**Files:**
- Modify: `crates/kulisawit-server/src/routes/tasks.rs`
- Modify: `crates/kulisawit-server/src/routes/mod.rs`
- Create: `crates/kulisawit-server/tests/tasks.rs`

- [ ] **Step 1: Write the failing integration tests**

Create `crates/kulisawit-server/tests/tasks.rs`:

```rust
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use http_body_util::BodyExt;
use kulisawit_agent::MockAgent;
use kulisawit_core::{AgentAdapter, ColumnId, ProjectId};
use kulisawit_db::{columns, connect, migrate, project};
use kulisawit_orchestrator::{AgentRegistry, Orchestrator, RuntimeConfig};
use kulisawit_server::{routes_for_testing, AppState};
use tempfile::tempdir;
use tower::ServiceExt;

/// Build an app + pre-seed a project and one column so task tests can insert.
async fn app_with_project() -> (axum::Router, ProjectId, ColumnId) {
    let pool = connect("sqlite::memory:").await.expect("pool");
    migrate(&pool).await.expect("mig");
    let mut registry = AgentRegistry::new();
    registry.register(Arc::new(MockAgent::default()) as Arc<dyn AgentAdapter>);
    let dir = tempdir().expect("tmp");

    let project_id = project::create(
        &pool,
        project::NewProject {
            name: "P".into(),
            repo_path: dir.path().display().to_string(),
        },
    )
    .await
    .expect("p");
    let cols = columns::seed_defaults(&pool, &project_id).await.expect("c");
    let column_id = cols[0].clone();

    let orch = Arc::new(Orchestrator::new(
        pool,
        registry,
        dir.path().to_path_buf(),
        dir.path().join("wt"),
        RuntimeConfig::default(),
    ));
    std::mem::forget(dir);
    (
        routes_for_testing(AppState { orch }),
        project_id,
        column_id,
    )
}

#[tokio::test]
async fn post_tasks_with_valid_body_returns_200() {
    let (app, project_id, column_id) = app_with_project().await;
    let body = serde_json::json!({
        "project_id": project_id.as_str(),
        "column_id": column_id.as_str(),
        "title": "my task"
    })
    .to_string();
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/tasks")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn post_tasks_with_bogus_project_id_returns_400() {
    let (app, _, column_id) = app_with_project().await;
    let body = serde_json::json!({
        "project_id": "does-not-exist",
        "column_id": column_id.as_str(),
        "title": "my task"
    })
    .to_string();
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/tasks")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let body = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(body.contains("project not found"), "got: {body}");
}

#[tokio::test]
async fn get_task_by_unknown_id_returns_404() {
    let (app, _, _) = app_with_project().await;
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/tasks/01900000-0000-0000-0000-000000000000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
```

- [ ] **Step 2: Run to verify it fails**

```bash
cargo test -p kulisawit-server --test tasks --locked
```

Expected: compile / test failures — `/api/tasks` routes not wired.

- [ ] **Step 3: Implement `routes/tasks.rs`**

Replace `crates/kulisawit-server/src/routes/tasks.rs`:

```rust
//! `/api/tasks` endpoints (dispatch lives here; implemented in Task 3.1.8).

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};

use kulisawit_core::TaskId;
use kulisawit_db::{
    columns,
    project,
    task::{self, NewTask},
};

use crate::wire::{NewTaskRequest, TaskResponse};
use crate::{AppState, ServerError, ServerResult};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/tasks", post(create))
        .route("/api/tasks/:id", get(get_by_id))
}

async fn create(
    State(state): State<AppState>,
    Json(req): Json<NewTaskRequest>,
) -> ServerResult<Json<TaskResponse>> {
    // Pre-validate: project must exist.
    if project::get(state.orch.pool(), &req.project_id)
        .await?
        .is_none()
    {
        return Err(ServerError::InvalidInput(format!(
            "project not found: {}",
            req.project_id.as_str()
        )));
    }
    // Pre-validate: column must exist and belong to this project.
    let cols = columns::list_for_project(state.orch.pool(), &req.project_id).await?;
    if !cols.iter().any(|c| c.id == req.column_id) {
        return Err(ServerError::InvalidInput(format!(
            "column not found in project: {}",
            req.column_id.as_str()
        )));
    }

    let id = task::create(
        state.orch.pool(),
        NewTask {
            project_id: req.project_id.clone(),
            column_id: req.column_id.clone(),
            title: req.title,
            description: req.description,
            tags: req.tags,
            linked_files: req.linked_files,
        },
    )
    .await?;
    let row = task::get(state.orch.pool(), &id)
        .await?
        .ok_or_else(|| ServerError::Internal("task vanished after insert".into()))?;
    Ok(Json(row.into()))
}

async fn get_by_id(
    State(state): State<AppState>,
    Path(id): Path<TaskId>,
) -> ServerResult<Json<TaskResponse>> {
    let row = task::get(state.orch.pool(), &id)
        .await?
        .ok_or_else(|| ServerError::NotFound {
            entity: "task",
            id: id.as_str().to_owned(),
        })?;
    Ok(Json(row.into()))
}
```

**Pre-flight note for the implementer:** `kulisawit_db::columns::list_for_project(pool, &ProjectId) -> DbResult<Vec<Column>>` must exist. If it does not, STOP and report BLOCKED — we do NOT add new DB functions in this task.

- [ ] **Step 4: Wire into `routes/mod.rs`**

Replace `crates/kulisawit-server/src/routes/mod.rs`:

```rust
//! Router composition.

pub mod attempts;
pub mod projects;
pub mod tasks;

use axum::Router;

use crate::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .merge(projects::routes())
        .merge(tasks::routes())
        .with_state(state)
}
```

- [ ] **Step 5: Run to verify**

```bash
cargo test -p kulisawit-server --test tasks --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --check
```

Expected: 3 tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/kulisawit-server/
git commit -m "feat(server): POST and GET /api/tasks with pre-validation"
```

---

### Task 3.1.7: `dispatch_batch_spawned` in orchestrator

**Files:**
- Modify: `crates/kulisawit-orchestrator/src/dispatch.rs`
- Modify: `crates/kulisawit-orchestrator/src/lib.rs`

The goal: expose a function that returns `AttemptId`s after all DB rows are inserted and the agent drives run detached via `tokio::spawn`. CLI `run` keeps using the existing `dispatch_batch` (sync-await-all).

**Design:** split `dispatch_single_attempt` into two steps via a new helper `reserve_and_launch`:
1. Reserve the DB row + install cancel flag (returns `AttemptId` + a boxed future that drives the rest).
2. Caller either `.await`s the future in place (existing `dispatch_batch` behavior) or spawns it (new `dispatch_batch_spawned` behavior).

- [ ] **Step 1: Write the failing integration test**

Append to `crates/kulisawit-orchestrator/tests/dispatch_batch.rs` (the file already exists from Task 2.1.8):

```rust
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn dispatch_batch_spawned_returns_ids_before_agents_finish() {
    let base = tempdir().expect("tmp");
    init_repo(base.path());
    let pool = connect("sqlite::memory:").await.expect("pool");
    migrate(&pool).await.expect("mig");
    let mut registry = AgentRegistry::new();
    registry.register(Arc::new(MockAgent::new(MockMode::Slow)) as Arc<dyn AgentAdapter>);
    let orch = std::sync::Arc::new(Orchestrator::new(
        pool,
        registry,
        base.path().to_path_buf(),
        base.path().join(".kulisawit/worktrees"),
        RuntimeConfig::default(),
    ));

    let project_id = project::create(
        orch.pool(),
        project::NewProject {
            name: "K".into(),
            repo_path: base.path().display().to_string(),
        },
    )
    .await
    .expect("p");
    let cols = columns::seed_defaults(orch.pool(), &project_id).await.expect("c");
    let task_id = task::create(
        orch.pool(),
        task::NewTask {
            project_id,
            column_id: cols[0].clone(),
            title: "batch spawned".into(),
            description: None,
            tags: vec![],
            linked_files: vec![],
        },
    )
    .await
    .expect("t");

    let before = std::time::Instant::now();
    let ids = kulisawit_orchestrator::dispatch_batch_spawned(&orch, &task_id, "mock", 2, None)
        .await
        .expect("spawned");
    let elapsed_ms = before.elapsed().as_millis();
    assert_eq!(ids.len(), 2);
    // Slow mock takes ≥1 second; if we returned in <500ms we clearly didn't wait.
    assert!(
        elapsed_ms < 500,
        "dispatch_batch_spawned should return immediately, took {elapsed_ms}ms"
    );

    // Attempts must exist in DB already (even if not yet terminal).
    for id in &ids {
        let row = attempt::get(orch.pool(), id).await.expect("get").expect("row");
        // Status could be Queued or Running by now; must not be terminal.
        assert!(
            matches!(
                row.status,
                kulisawit_core::AttemptStatus::Queued | kulisawit_core::AttemptStatus::Running
            ),
            "expected Queued/Running, got {:?}",
            row.status
        );
    }

    // Wait for completion by polling attempts.
    for id in &ids {
        for _ in 0..200 {
            let row = attempt::get(orch.pool(), id).await.expect("get").expect("row");
            if matches!(
                row.status,
                kulisawit_core::AttemptStatus::Completed
                    | kulisawit_core::AttemptStatus::Failed
                    | kulisawit_core::AttemptStatus::Cancelled
            ) {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    }
}
```

- [ ] **Step 2: Run to verify it fails**

```bash
cargo test -p kulisawit-orchestrator --test dispatch_batch dispatch_batch_spawned_returns_ids_before_agents_finish --locked
```

Expected: compile error — `dispatch_batch_spawned` doesn't exist.

- [ ] **Step 3: Implement `dispatch_batch_spawned`**

Edit `crates/kulisawit-orchestrator/src/dispatch.rs`. Above the existing `dispatch_batch` function, add a private helper + the new public function. The helper extracts the "reserve DB row" portion of `dispatch_single_attempt` into a synchronous step (still async because of DB I/O) that returns the `AttemptId` and a future that drives the remaining lifecycle.

Full new additions (paste above `pub async fn dispatch_batch`):

```rust
/// Split `dispatch_single_attempt` into two phases so the server can return
/// AttemptIds immediately while the agent run proceeds in the background.
///
/// Returns `(attempt_id, run_future)`. The future completes when the attempt
/// reaches a terminal `AttemptStatus` (Completed/Failed/Cancelled).
#[instrument(skip(orch), fields(task = %task_id, agent = agent_id))]
async fn reserve_and_build_run(
    orch: Arc<Orchestrator>,
    task_id: TaskId,
    agent_id: String,
    prompt_variant: Option<String>,
) -> OrchestratorResult<(
    AttemptId,
    std::pin::Pin<Box<dyn std::future::Future<Output = OrchestratorResult<()>> + Send>>,
)> {
    let _permit = orch
        .semaphore()
        .clone()
        .acquire_owned()
        .await
        .map_err(|e| OrchestratorError::Invalid(format!("semaphore closed: {e}")))?;

    let task_row = task::get(orch.pool(), &task_id)
        .await?
        .ok_or_else(|| OrchestratorError::Invalid(format!("task not found: {task_id}")))?;

    let prompt = crate::prompt::compose_prompt(&task_row, prompt_variant.as_deref());

    let adapter = orch
        .registry()
        .get(&agent_id)
        .ok_or_else(|| OrchestratorError::Invalid(format!("agent not registered: {agent_id}")))?;

    let draft_id = AttemptId::new();
    let attempt_short = short_attempt(draft_id.as_str());
    let task_short = short(task_id.as_str());
    let branch_name = format!("kulisawit/{task_short}/{attempt_short}");
    let worktree_path = orch
        .worktree_root()
        .join(format!("attempt-{attempt_short}"));

    let base_ref = head_commit_sha(orch.repo_root()).map_err(OrchestratorError::from)?;

    let attempt_id = attempt::create(
        orch.pool(),
        attempt::NewAttempt {
            task_id: task_id.clone(),
            agent_id: agent_id.clone(),
            prompt_variant: prompt_variant.clone(),
            worktree_path: worktree_path.display().to_string(),
            branch_name: branch_name.clone(),
        },
    )
    .await?;

    let wt_outcome = create_worktree(CreateWorktreeRequest {
        repo_root: orch.repo_root().to_path_buf(),
        worktree_root: orch.worktree_root().to_path_buf(),
        attempt_short_id: attempt_short.clone(),
        branch_name: branch_name.clone(),
        base_ref,
    })
    .await?;

    let cancel_notify = orch.install_cancel_flag(&attempt_id).await;

    // Build the future that finishes the lifecycle. Capture everything it needs.
    let orch_for_run = orch.clone();
    let attempt_id_for_run = attempt_id.clone();
    let attempt_title = task_row.title;
    let run_future = Box::pin(async move {
        let orch = orch_for_run;
        let attempt_id = attempt_id_for_run;

        attempt::mark_running(orch.pool(), &attempt_id).await?;

        let run_ctx = RunContext {
            run_id: attempt_id.as_str().to_owned(),
            worktree_path: wt_outcome.worktree_path.clone(),
            prompt,
            prompt_variant,
            env: std::collections::HashMap::new(),
        };

        let mut stream = adapter.run(run_ctx).await?;

        let terminal: AttemptStatus = loop {
            tokio::select! {
                biased;
                _ = cancel_notify.notified() => {
                    let _ = adapter.cancel(attempt_id.as_str()).await;
                    let evt = AgentEvent::Status {
                        status: RunStatus::Cancelled,
                        detail: Some("cancelled by orchestrator".into()),
                    };
                    let _ = events::append(orch.pool(), &attempt_id, &evt).await;
                    orch.broadcaster().send(&attempt_id, evt);
                    break AttemptStatus::Cancelled;
                }
                next = stream.next() => {
                    let Some(evt) = next else {
                        warn!(attempt = %attempt_id, "adapter stream ended without terminal status");
                        break AttemptStatus::Failed;
                    };
                    let _ = events::append(orch.pool(), &attempt_id, &evt).await;
                    orch.broadcaster().send(&attempt_id, evt.clone());
                    if let AgentEvent::Status { status, .. } = &evt {
                        if let Some(mapped) = AttemptStatus::from_terminal_run_status(*status) {
                            break mapped;
                        }
                    }
                }
            }
        };

        let commit_msg = format!("kulisawit: attempt {attempt_short} for {attempt_title}");
        if let Err(e) = commit_all_in_worktree(&wt_outcome.worktree_path, &commit_msg).await {
            warn!(attempt = %attempt_id, "commit_all_in_worktree failed: {e}");
        }

        attempt::mark_terminal(orch.pool(), &attempt_id, terminal).await?;

        orch.broadcaster().close(&attempt_id);
        orch.remove_cancel_flag(&attempt_id).await;

        // Drop the draft_id so the compiler doesn't warn; it was only used for
        // path naming.
        let _ = draft_id;
        drop(_permit);
        Ok(())
    });

    Ok((attempt_id, run_future))
}

/// Dispatch a batch; return the `AttemptId`s after each attempt has its DB row
/// inserted and its worktree created. The agent drives run in a background
/// `tokio::spawn`. This is the async-dispatch entry point used by the HTTP
/// server.
#[instrument(skip(orch), fields(task = %task_id, agent = agent_id, n = batch_size))]
pub async fn dispatch_batch_spawned(
    orch: &Arc<Orchestrator>,
    task_id: &TaskId,
    agent_id: &str,
    batch_size: usize,
    variants: Option<Vec<String>>,
) -> OrchestratorResult<Vec<AttemptId>> {
    if batch_size == 0 {
        return Err(OrchestratorError::Invalid("batch_size must be >= 1".into()));
    }
    if let Some(v) = &variants {
        if v.len() != batch_size {
            return Err(OrchestratorError::Invalid(format!(
                "variants length {} != batch_size {}",
                v.len(),
                batch_size
            )));
        }
    }

    let mut ids = Vec::with_capacity(batch_size);
    for i in 0..batch_size {
        let variant = variants.as_ref().and_then(|v| v.get(i).cloned());
        let (id, run_future) = reserve_and_build_run(
            Arc::clone(orch),
            task_id.clone(),
            agent_id.to_owned(),
            variant,
        )
        .await?;
        ids.push(id);
        tokio::spawn(async move {
            if let Err(e) = run_future.await {
                warn!("spawned attempt failed: {e}");
            }
        });
    }
    Ok(ids)
}
```

Note: `reserve_and_build_run` holds a semaphore permit for the duration of the future. Since we move `_permit` into the future via `drop(_permit)` at the end, the permit is released when the future completes. Before Step 3 lands, skim the existing `dispatch_single_attempt` — the new helper is a near-duplicate with the same error behavior, so if a prior commit changed `dispatch_single_attempt`, mirror the same change here.

**Imports needed at the top of `dispatch.rs`** — ensure these are present (most already are from Task 2.1.7):

```rust
use std::sync::Arc;

use futures::StreamExt;
use kulisawit_core::{AgentEvent, AttemptId, AttemptStatus, RunContext, RunStatus, TaskId};
use kulisawit_db::{attempt, events, task};
use kulisawit_git::{
    branch::commit_all_in_worktree,
    query::head_commit_sha,
    worktree::{create_worktree, CreateWorktreeRequest},
};
use tracing::{instrument, warn};

use crate::{Orchestrator, OrchestratorError, OrchestratorResult};
```

- [ ] **Step 4: Re-export from `lib.rs`**

Update `crates/kulisawit-orchestrator/src/lib.rs`:

```rust
pub use dispatch::{dispatch_batch, dispatch_batch_spawned, dispatch_single_attempt};
```

- [ ] **Step 5: Run to verify**

```bash
cargo test -p kulisawit-orchestrator --test dispatch_batch --locked
cargo test -p kulisawit-orchestrator --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --check
```

Expected: new test passes (3 existing dispatch_batch tests + 1 new = 4; full crate count bumps by 1).

- [ ] **Step 6: Commit**

```bash
git add crates/kulisawit-orchestrator/
git commit -m "feat(orchestrator): dispatch_batch_spawned returns ids before agents finish"
```

---

### Task 3.1.8: `POST /api/tasks/:id/dispatch`

**Files:**
- Modify: `crates/kulisawit-server/src/routes/tasks.rs`
- Create: `crates/kulisawit-server/tests/dispatch.rs`

- [ ] **Step 1: Write the failing integration tests**

Create `crates/kulisawit-server/tests/dispatch.rs`:

```rust
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::process::Command;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use http_body_util::BodyExt;
use kulisawit_agent::MockAgent;
use kulisawit_core::{AgentAdapter, TaskId};
use kulisawit_db::{columns, connect, migrate, project, task};
use kulisawit_orchestrator::{AgentRegistry, Orchestrator, RuntimeConfig};
use kulisawit_server::{routes_for_testing, AppState};
use tempfile::tempdir;
use tower::ServiceExt;

fn init_repo(dir: &std::path::Path) {
    Command::new("git").args(["init", "-b", "main"]).current_dir(dir).status().unwrap();
    std::fs::write(dir.join("README.md"), "# t\n").unwrap();
    Command::new("git")
        .args(["-c", "user.email=t@t", "-c", "user.name=t", "add", "."])
        .current_dir(dir)
        .status()
        .unwrap();
    Command::new("git")
        .args(["-c", "user.email=t@t", "-c", "user.name=t", "commit", "-m", "i"])
        .current_dir(dir)
        .status()
        .unwrap();
}

async fn app_with_task() -> (axum::Router, TaskId) {
    let pool = connect("sqlite::memory:").await.expect("pool");
    migrate(&pool).await.expect("mig");
    let mut registry = AgentRegistry::new();
    registry.register(Arc::new(MockAgent::default()) as Arc<dyn AgentAdapter>);
    let dir = tempdir().expect("tmp");
    init_repo(dir.path());

    let project_id = project::create(
        &pool,
        project::NewProject {
            name: "P".into(),
            repo_path: dir.path().display().to_string(),
        },
    )
    .await
    .expect("p");
    let cols = columns::seed_defaults(&pool, &project_id).await.expect("c");
    let task_id = task::create(
        &pool,
        task::NewTask {
            project_id,
            column_id: cols[0].clone(),
            title: "t".into(),
            description: None,
            tags: vec![],
            linked_files: vec![],
        },
    )
    .await
    .expect("t");

    let orch = Arc::new(Orchestrator::new(
        pool,
        registry,
        dir.path().to_path_buf(),
        dir.path().join("wt"),
        RuntimeConfig::default(),
    ));
    std::mem::forget(dir);
    (routes_for_testing(AppState { orch }), task_id)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn dispatch_unknown_task_returns_404() {
    let (app, _) = app_with_task().await;
    let body = r#"{"agent":"mock","batch":1}"#;
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/tasks/01900000-0000-0000-0000-000000000000/dispatch")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn dispatch_batch_zero_returns_400() {
    let (app, task_id) = app_with_task().await;
    let body = r#"{"agent":"mock","batch":0}"#;
    let uri = format!("/api/tasks/{}/dispatch", task_id.as_str());
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(&uri)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn dispatch_valid_returns_ids() {
    let (app, task_id) = app_with_task().await;
    let body = r#"{"agent":"mock","batch":2}"#;
    let uri = format!("/api/tasks/{}/dispatch", task_id.as_str());
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(&uri)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let ids = json["attempt_ids"].as_array().expect("ids array");
    assert_eq!(ids.len(), 2);
    for id in ids {
        assert!(id.is_string());
    }
}
```

- [ ] **Step 2: Run to verify it fails**

```bash
cargo test -p kulisawit-server --test dispatch --locked
```

Expected: test failures / route not found.

- [ ] **Step 3: Extend `routes/tasks.rs`**

Edit `crates/kulisawit-server/src/routes/tasks.rs`. Update the imports and add the `dispatch` handler + route:

```rust
//! `/api/tasks` endpoints including `/api/tasks/:id/dispatch`.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};

use kulisawit_core::TaskId;
use kulisawit_db::{
    columns,
    project,
    task::{self, NewTask},
};
use kulisawit_orchestrator::dispatch_batch_spawned;

use crate::wire::{DispatchRequest, DispatchResponse, NewTaskRequest, TaskResponse};
use crate::{AppState, ServerError, ServerResult};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/tasks", post(create))
        .route("/api/tasks/:id", get(get_by_id))
        .route("/api/tasks/:id/dispatch", post(dispatch))
}

async fn create(
    State(state): State<AppState>,
    Json(req): Json<NewTaskRequest>,
) -> ServerResult<Json<TaskResponse>> {
    if project::get(state.orch.pool(), &req.project_id)
        .await?
        .is_none()
    {
        return Err(ServerError::InvalidInput(format!(
            "project not found: {}",
            req.project_id.as_str()
        )));
    }
    let cols = columns::list_for_project(state.orch.pool(), &req.project_id).await?;
    if !cols.iter().any(|c| c.id == req.column_id) {
        return Err(ServerError::InvalidInput(format!(
            "column not found in project: {}",
            req.column_id.as_str()
        )));
    }

    let id = task::create(
        state.orch.pool(),
        NewTask {
            project_id: req.project_id.clone(),
            column_id: req.column_id.clone(),
            title: req.title,
            description: req.description,
            tags: req.tags,
            linked_files: req.linked_files,
        },
    )
    .await?;
    let row = task::get(state.orch.pool(), &id)
        .await?
        .ok_or_else(|| ServerError::Internal("task vanished after insert".into()))?;
    Ok(Json(row.into()))
}

async fn get_by_id(
    State(state): State<AppState>,
    Path(id): Path<TaskId>,
) -> ServerResult<Json<TaskResponse>> {
    let row = task::get(state.orch.pool(), &id)
        .await?
        .ok_or_else(|| ServerError::NotFound {
            entity: "task",
            id: id.as_str().to_owned(),
        })?;
    Ok(Json(row.into()))
}

async fn dispatch(
    State(state): State<AppState>,
    Path(id): Path<TaskId>,
    Json(req): Json<DispatchRequest>,
) -> ServerResult<Json<DispatchResponse>> {
    // Pre-validate task exists; library-level dispatch already does this, but
    // here we want a clean 404 instead of 500.
    if task::get(state.orch.pool(), &id).await?.is_none() {
        return Err(ServerError::NotFound {
            entity: "task",
            id: id.as_str().to_owned(),
        });
    }

    let orch = Arc::clone(&state.orch);
    let attempt_ids = dispatch_batch_spawned(&orch, &id, &req.agent, req.batch, req.variants)
        .await
        .map_err(|e| match e {
            kulisawit_orchestrator::OrchestratorError::Invalid(msg) => {
                ServerError::InvalidInput(msg)
            }
            other => ServerError::from(other),
        })?;

    Ok(Json(DispatchResponse { attempt_ids }))
}
```

- [ ] **Step 4: Run to verify**

```bash
cargo test -p kulisawit-server --test dispatch --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --check
```

Expected: 3 dispatch tests pass; full workspace pass.

- [ ] **Step 5: Commit**

```bash
git add crates/kulisawit-server/
git commit -m "feat(server): POST /api/tasks/:id/dispatch"
```

---

### Task 3.1.9: `GET /api/attempts/:id`

**Files:**
- Modify: `crates/kulisawit-server/src/routes/attempts.rs`
- Modify: `crates/kulisawit-server/src/routes/mod.rs`
- Create: `crates/kulisawit-server/tests/attempts.rs`

- [ ] **Step 1: Write the failing test**

Create `crates/kulisawit-server/tests/attempts.rs`:

```rust
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use http_body_util::BodyExt;
use kulisawit_agent::MockAgent;
use kulisawit_core::AgentAdapter;
use kulisawit_db::{attempt as attempt_db, columns, connect, migrate, project, task};
use kulisawit_orchestrator::{AgentRegistry, Orchestrator, RuntimeConfig};
use kulisawit_server::{routes_for_testing, AppState};
use tempfile::tempdir;
use tower::ServiceExt;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_attempt_unknown_returns_404() {
    let pool = connect("sqlite::memory:").await.expect("pool");
    migrate(&pool).await.expect("mig");
    let mut registry = AgentRegistry::new();
    registry.register(Arc::new(MockAgent::default()) as Arc<dyn AgentAdapter>);
    let dir = tempdir().expect("tmp");
    let orch = Arc::new(Orchestrator::new(
        pool,
        registry,
        dir.path().to_path_buf(),
        dir.path().join("wt"),
        RuntimeConfig::default(),
    ));
    std::mem::forget(dir);
    let app = routes_for_testing(AppState { orch });

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/attempts/01900000-0000-0000-0000-000000000000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_attempt_existing_returns_200_with_expected_shape() {
    let pool = connect("sqlite::memory:").await.expect("pool");
    migrate(&pool).await.expect("mig");
    let mut registry = AgentRegistry::new();
    registry.register(Arc::new(MockAgent::default()) as Arc<dyn AgentAdapter>);
    let dir = tempdir().expect("tmp");

    let project_id = project::create(
        &pool,
        project::NewProject {
            name: "P".into(),
            repo_path: dir.path().display().to_string(),
        },
    )
    .await
    .expect("p");
    let cols = columns::seed_defaults(&pool, &project_id).await.expect("c");
    let task_id = task::create(
        &pool,
        task::NewTask {
            project_id,
            column_id: cols[0].clone(),
            title: "t".into(),
            description: None,
            tags: vec![],
            linked_files: vec![],
        },
    )
    .await
    .expect("t");
    let attempt_id = attempt_db::create(
        &pool,
        attempt_db::NewAttempt {
            task_id: task_id.clone(),
            agent_id: "mock".into(),
            prompt_variant: None,
            worktree_path: "/tmp/wt".into(),
            branch_name: "kulisawit/a/b".into(),
        },
    )
    .await
    .expect("a");

    let orch = Arc::new(Orchestrator::new(
        pool,
        registry,
        dir.path().to_path_buf(),
        dir.path().join("wt"),
        RuntimeConfig::default(),
    ));
    std::mem::forget(dir);
    let app = routes_for_testing(AppState { orch });

    let uri = format!("/api/attempts/{}", attempt_id.as_str());
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(&uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["id"], attempt_id.as_str());
    assert_eq!(json["agent_id"], "mock");
    assert_eq!(json["status"], "queued");
    assert_eq!(json["branch_name"], "kulisawit/a/b");
    assert!(!json.as_object().unwrap().contains_key("verification_status"));
}
```

- [ ] **Step 2: Run to verify it fails**

```bash
cargo test -p kulisawit-server --test attempts --locked
```

Expected: route not found.

- [ ] **Step 3: Implement `routes/attempts.rs`**

Replace `crates/kulisawit-server/src/routes/attempts.rs`:

```rust
//! `/api/attempts` endpoints. SSE stream lands in Task 3.1.10.

use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};

use kulisawit_core::AttemptId;
use kulisawit_db::attempt;

use crate::wire::AttemptResponse;
use crate::{AppState, ServerError, ServerResult};

pub fn routes() -> Router<AppState> {
    Router::new().route("/api/attempts/:id", get(get_by_id))
}

async fn get_by_id(
    State(state): State<AppState>,
    Path(id): Path<AttemptId>,
) -> ServerResult<Json<AttemptResponse>> {
    let row = attempt::get(state.orch.pool(), &id)
        .await?
        .ok_or_else(|| ServerError::NotFound {
            entity: "attempt",
            id: id.as_str().to_owned(),
        })?;
    Ok(Json(row.into()))
}
```

- [ ] **Step 4: Wire into `routes/mod.rs`**

Replace `crates/kulisawit-server/src/routes/mod.rs`:

```rust
//! Router composition.

pub mod attempts;
pub mod projects;
pub mod tasks;

use axum::Router;

use crate::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .merge(projects::routes())
        .merge(tasks::routes())
        .merge(attempts::routes())
        .with_state(state)
}
```

- [ ] **Step 5: Run to verify**

```bash
cargo test -p kulisawit-server --test attempts --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --check
```

Expected: 2 tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/kulisawit-server/
git commit -m "feat(server): GET /api/attempts/:id"
```

---

### Task 3.1.10: `GET /api/attempts/:id/events` (SSE)

**Files:**
- Modify: `crates/kulisawit-server/src/routes/attempts.rs`
- Create: `crates/kulisawit-server/tests/sse.rs`

- [ ] **Step 1: Write the failing test**

Create `crates/kulisawit-server/tests/sse.rs`:

```rust
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use http_body_util::BodyExt;
use kulisawit_agent::MockAgent;
use kulisawit_core::{AgentAdapter, AgentEvent, RunStatus};
use kulisawit_db::{attempt as attempt_db, columns, connect, migrate, project, task};
use kulisawit_orchestrator::{AgentRegistry, Orchestrator, RuntimeConfig};
use kulisawit_server::{routes_for_testing, AppState};
use tempfile::tempdir;
use tower::ServiceExt;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sse_unknown_attempt_returns_404() {
    let pool = connect("sqlite::memory:").await.expect("pool");
    migrate(&pool).await.expect("mig");
    let mut registry = AgentRegistry::new();
    registry.register(Arc::new(MockAgent::default()) as Arc<dyn AgentAdapter>);
    let dir = tempdir().expect("tmp");
    let orch = Arc::new(Orchestrator::new(
        pool,
        registry,
        dir.path().to_path_buf(),
        dir.path().join("wt"),
        RuntimeConfig::default(),
    ));
    std::mem::forget(dir);
    let app = routes_for_testing(AppState { orch });

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/attempts/01900000-0000-0000-0000-000000000000/events")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sse_terminal_attempt_emits_single_status_and_closes() {
    let pool = connect("sqlite::memory:").await.expect("pool");
    migrate(&pool).await.expect("mig");
    let mut registry = AgentRegistry::new();
    registry.register(Arc::new(MockAgent::default()) as Arc<dyn AgentAdapter>);
    let dir = tempdir().expect("tmp");

    let project_id = project::create(
        &pool,
        project::NewProject {
            name: "P".into(),
            repo_path: dir.path().display().to_string(),
        },
    )
    .await
    .expect("p");
    let cols = columns::seed_defaults(&pool, &project_id).await.expect("c");
    let task_id = task::create(
        &pool,
        task::NewTask {
            project_id,
            column_id: cols[0].clone(),
            title: "t".into(),
            description: None,
            tags: vec![],
            linked_files: vec![],
        },
    )
    .await
    .expect("t");
    let attempt_id = attempt_db::create(
        &pool,
        attempt_db::NewAttempt {
            task_id,
            agent_id: "mock".into(),
            prompt_variant: None,
            worktree_path: "/tmp/wt".into(),
            branch_name: "b".into(),
        },
    )
    .await
    .expect("a");
    // Drive it straight to Completed without going through the orchestrator.
    attempt_db::mark_running(&pool, &attempt_id).await.expect("run");
    attempt_db::mark_terminal(&pool, &attempt_id, kulisawit_core::AttemptStatus::Completed)
        .await
        .expect("term");

    let orch = Arc::new(Orchestrator::new(
        pool,
        registry,
        dir.path().to_path_buf(),
        dir.path().join("wt"),
        RuntimeConfig::default(),
    ));
    std::mem::forget(dir);
    let app = routes_for_testing(AppState { orch });

    let uri = format!("/api/attempts/{}/events", attempt_id.as_str());
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(&uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        resp.headers()
            .get(axum::http::header::CONTENT_TYPE)
            .map(|v| v.to_str().unwrap_or("")),
        Some("text/event-stream")
    );
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let s = String::from_utf8_lossy(&body);
    assert!(
        s.contains("\"status\":\"Completed\"") || s.contains("\"status\":\"completed\""),
        "expected Completed status envelope, got:\n{s}"
    );
}

/// Exercise the live path: subscribe to SSE while the broadcaster is open,
/// send one event, and assert the client reads it back.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sse_live_attempt_streams_events() {
    let pool = connect("sqlite::memory:").await.expect("pool");
    migrate(&pool).await.expect("mig");
    let mut registry = AgentRegistry::new();
    registry.register(Arc::new(MockAgent::default()) as Arc<dyn AgentAdapter>);
    let dir = tempdir().expect("tmp");

    let project_id = project::create(
        &pool,
        project::NewProject {
            name: "P".into(),
            repo_path: dir.path().display().to_string(),
        },
    )
    .await
    .expect("p");
    let cols = columns::seed_defaults(&pool, &project_id).await.expect("c");
    let task_id = task::create(
        &pool,
        task::NewTask {
            project_id,
            column_id: cols[0].clone(),
            title: "t".into(),
            description: None,
            tags: vec![],
            linked_files: vec![],
        },
    )
    .await
    .expect("t");
    let attempt_id = attempt_db::create(
        &pool,
        attempt_db::NewAttempt {
            task_id,
            agent_id: "mock".into(),
            prompt_variant: None,
            worktree_path: "/tmp/wt".into(),
            branch_name: "b".into(),
        },
    )
    .await
    .expect("a");
    attempt_db::mark_running(&pool, &attempt_id).await.expect("run");

    let orch = Arc::new(Orchestrator::new(
        pool,
        registry,
        dir.path().to_path_buf(),
        dir.path().join("wt"),
        RuntimeConfig::default(),
    ));
    let broadcaster = orch.broadcaster().clone();
    let attempt_id_bg = attempt_id.clone();
    // Send an event on a delay so the SSE endpoint has time to subscribe.
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        broadcaster.send(
            &attempt_id_bg,
            AgentEvent::Stdout { text: "live-event".into() },
        );
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        broadcaster.send(
            &attempt_id_bg,
            AgentEvent::Status {
                status: RunStatus::Completed,
                detail: None,
            },
        );
        // Drop the broadcaster's entry so the stream ends.
        broadcaster.close(&attempt_id_bg);
    });
    std::mem::forget(dir);
    let app = routes_for_testing(AppState { orch });

    let uri = format!("/api/attempts/{}/events", attempt_id.as_str());
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(&uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let s = String::from_utf8_lossy(&body);
    assert!(s.contains("live-event"), "expected stdout in stream:\n{s}");
    assert!(
        s.contains("Completed") || s.contains("completed"),
        "expected Completed status:\n{s}"
    );
}
```

- [ ] **Step 2: Run to verify it fails**

```bash
cargo test -p kulisawit-server --test sse --locked
```

Expected: route not found or compile error.

- [ ] **Step 3: Extend `routes/attempts.rs`**

Replace `crates/kulisawit-server/src/routes/attempts.rs`:

```rust
//! `/api/attempts` endpoints including SSE.

use std::convert::Infallible;
use std::time::Duration;

use axum::extract::{Path, State};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use chrono::Utc;
use futures::{stream, Stream, StreamExt};

use kulisawit_core::{AgentEvent, AttemptId, AttemptStatus, RunStatus};
use kulisawit_db::attempt;

use crate::wire::{AttemptResponse, EventEnvelope};
use crate::{AppState, ServerError, ServerResult};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/attempts/:id", get(get_by_id))
        .route("/api/attempts/:id/events", get(events))
}

async fn get_by_id(
    State(state): State<AppState>,
    Path(id): Path<AttemptId>,
) -> ServerResult<Json<AttemptResponse>> {
    let row = attempt::get(state.orch.pool(), &id)
        .await?
        .ok_or_else(|| ServerError::NotFound {
            entity: "attempt",
            id: id.as_str().to_owned(),
        })?;
    Ok(Json(row.into()))
}

async fn events(
    State(state): State<AppState>,
    Path(id): Path<AttemptId>,
) -> Result<axum::response::Response, ServerError> {
    let row = attempt::get(state.orch.pool(), &id)
        .await?
        .ok_or_else(|| ServerError::NotFound {
            entity: "attempt",
            id: id.as_str().to_owned(),
        })?;

    let id_owned = id.clone();
    let stream: std::pin::Pin<
        Box<dyn Stream<Item = Result<Event, Infallible>> + Send>,
    > = if is_terminal(row.status) {
        // Synthesize a final Status event and close.
        let run_status = attempt_to_run_status(row.status);
        let evt = AgentEvent::Status {
            status: run_status,
            detail: None,
        };
        let envelope = EventEnvelope {
            attempt_id: id_owned,
            event: evt,
            ts_ms: Utc::now().timestamp_millis(),
        };
        let event = Event::default()
            .data(serde_json::to_string(&envelope).unwrap_or_default());
        Box::pin(stream::iter(vec![Ok(event)]))
    } else {
        let rx = state.orch.broadcaster().subscribe(&id_owned);
        let id_for_map = id_owned.clone();
        Box::pin(
            tokio_stream::wrappers::BroadcastStream::new(rx)
                .filter_map(move |res| {
                    let id = id_for_map.clone();
                    async move {
                        match res {
                            Ok(evt) => {
                                let envelope = EventEnvelope {
                                    attempt_id: id,
                                    event: evt,
                                    ts_ms: Utc::now().timestamp_millis(),
                                };
                                let json = serde_json::to_string(&envelope).ok()?;
                                Some(Ok(Event::default().data(json)))
                            }
                            Err(tokio_stream::wrappers::errors::BroadcastStreamRecvError::Lagged(_)) => None,
                        }
                    }
                }),
        )
    };

    let sse = Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keepalive"),
    );
    Ok(sse.into_response())
}

fn is_terminal(s: AttemptStatus) -> bool {
    matches!(
        s,
        AttemptStatus::Completed | AttemptStatus::Failed | AttemptStatus::Cancelled
    )
}

fn attempt_to_run_status(s: AttemptStatus) -> RunStatus {
    match s {
        AttemptStatus::Completed => RunStatus::Completed,
        AttemptStatus::Failed => RunStatus::Failed,
        AttemptStatus::Cancelled => RunStatus::Cancelled,
        // Non-terminal: caller ensured this branch isn't reached; fall back to Failed.
        _ => RunStatus::Failed,
    }
}
```

- [ ] **Step 4: Add `tokio-stream` workspace dependency**

Edit the root `Cargo.toml`. Under `[workspace.dependencies]`, add:

```toml
tokio-stream = { version = "0.1", features = ["sync"] }
```

Edit `crates/kulisawit-server/Cargo.toml`. Under `[dependencies]`, add:

```toml
tokio-stream.workspace = true
```

- [ ] **Step 5: Run to verify**

```bash
cargo test -p kulisawit-server --test sse --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --check
```

Expected: 3 tests pass.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml crates/kulisawit-server/
git commit -m "feat(server): GET /api/attempts/:id/events SSE stream"
```

---

### Task 3.1.11: CLI `serve` subcommand

**Files:**
- Modify: `crates/kulisawit-cli/Cargo.toml`
- Modify: `crates/kulisawit-cli/src/main.rs`
- Modify: `crates/kulisawit-cli/src/commands/mod.rs`
- Create: `crates/kulisawit-cli/src/commands/serve.rs`
- Modify: `crates/kulisawit-cli/tests/cli_help.rs` (extend help test)

- [ ] **Step 1: Extend the help test**

Edit `crates/kulisawit-cli/tests/cli_help.rs`. Inside `help_lists_version_and_run_subcommands`, add one more assertion:

```rust
    assert!(combined.contains("serve"), "help missing 'serve': {combined}");
```

- [ ] **Step 2: Run to verify it fails**

```bash
cargo test -p kulisawit-cli --test cli_help --locked
```

Expected: assertion fail — `serve` not in help yet.

- [ ] **Step 3: Add `kulisawit-server` to CLI deps**

Edit `crates/kulisawit-cli/Cargo.toml`. Under `[dependencies]`, append:

```toml
kulisawit-server.workspace = true
```

Root `Cargo.toml` already has `kulisawit-server` listed under `[workspace.dependencies]` from Task 3.1.1 — confirm the presence first; if missing, add:

```toml
kulisawit-server = { path = "crates/kulisawit-server", version = "0.1.0-dev" }
```

- [ ] **Step 4: Implement `commands/serve.rs`**

Create `crates/kulisawit-cli/src/commands/serve.rs`:

```rust
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
```

- [ ] **Step 5: Register the subcommand in `main.rs`**

Edit `crates/kulisawit-cli/src/main.rs`. In the `Command` enum, add a new variant; in `main()`, add its match arm.

Full file replacement:

```rust
//! Kulisawit CLI binary.

mod commands;

use clap::{Parser, Subcommand};

use kulisawit_core::TaskId;
use std::path::PathBuf;

/// Kulisawit — plant N parallel AI coding agents per task.
#[derive(Debug, Parser)]
#[command(
    name = "kulisawit",
    version = env!("CARGO_PKG_VERSION"),
    about = "Kulisawit — plant N parallel AI coding agents per task",
    long_about = None
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Print version and exit.
    Version,
    /// Dispatch a batch of attempts for a task.
    Run(RunArgs),
    /// Start the HTTP + SSE server.
    Serve(commands::serve::ServeArgs),
}

#[derive(Debug, clap::Args)]
pub struct RunArgs {
    #[arg(long)]
    pub db: PathBuf,
    #[arg(long)]
    pub repo: PathBuf,
    #[arg(long)]
    pub task: TaskId,
    #[arg(long, default_value = "mock")]
    pub agent: String,
    #[arg(long, default_value_t = 1)]
    pub batch: usize,
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();
    match cli.command {
        Command::Version => commands::version::run(),
        Command::Run(args) => {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()?;
            rt.block_on(commands::run::run(args))
        }
        Command::Serve(args) => {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()?;
            rt.block_on(commands::serve::run(args))
        }
    }
}
```

Edit `crates/kulisawit-cli/src/commands/mod.rs`:

```rust
pub mod run;
pub mod serve;
pub mod version;
```

- [ ] **Step 6: Run to verify**

```bash
cargo test -p kulisawit-cli --test cli_help --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --check
cargo run -p kulisawit-cli --quiet -- serve --help
```

Expected: `help_lists_version_and_run_subcommands` passes (now also asserting `serve`); `--help` for serve lists `--db`, `--repo`, `--port`, `--worktree-root`.

- [ ] **Step 7: Commit**

```bash
git add crates/kulisawit-cli/
git commit -m "feat(cli): serve subcommand starts kulisawit-server"
```

---

### Task 3.1.12: End-to-end smoke with `reqwest`

**Files:**
- Modify: `crates/kulisawit-server/src/lib.rs` (adds `serve_with_shutdown_ready`)
- Create: `crates/kulisawit-server/tests/e2e.rs`

The e2e test binds the server to `127.0.0.1:0` (ephemeral port) and needs the actual bound port to make HTTP requests. We add a `serve_with_shutdown_ready` helper that emits the `SocketAddr` via a `oneshot::Sender` once the listener is ready. The existing `serve` and `serve_with_shutdown` delegate to it.

- [ ] **Step 1: Add `serve_with_shutdown_ready` helper**

Edit `crates/kulisawit-server/src/lib.rs`. Replace the existing `serve_with_shutdown` with a pair:

```rust
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

pub async fn serve(config: ServeConfig) -> ServerResult<SocketAddr> {
    serve_with_shutdown(config, shutdown_signal()).await
}
```

- [ ] **Step 2: Write the e2e test using the ready channel**

Create `crates/kulisawit-server/tests/e2e.rs`:

```rust
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use futures::StreamExt;
use kulisawit_db::{columns, connect, migrate, project, task};
use kulisawit_orchestrator::RuntimeConfig;
use kulisawit_server::{serve_with_shutdown_ready, ServeConfig};
use tempfile::tempdir;

fn init_repo(dir: &std::path::Path) {
    Command::new("git").args(["init", "-b", "main"]).current_dir(dir).status().unwrap();
    std::fs::write(dir.join("README.md"), "# t\n").unwrap();
    Command::new("git")
        .args(["-c", "user.email=t@t", "-c", "user.name=t", "add", "."])
        .current_dir(dir)
        .status()
        .unwrap();
    Command::new("git")
        .args(["-c", "user.email=t@t", "-c", "user.name=t", "commit", "-m", "i"])
        .current_dir(dir)
        .status()
        .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn end_to_end_dispatch_and_sse() {
    let dir = tempdir().expect("tmp");
    init_repo(dir.path());
    let db_path = dir.path().join("k.sqlite");

    let pool = connect(db_path.to_str().expect("utf8")).await.expect("pool");
    migrate(&pool).await.expect("mig");
    let project_id = project::create(
        &pool,
        project::NewProject {
            name: "E2E".into(),
            repo_path: dir.path().display().to_string(),
        },
    )
    .await
    .expect("p");
    let cols = columns::seed_defaults(&pool, &project_id).await.expect("c");
    let task_id = task::create(
        &pool,
        task::NewTask {
            project_id,
            column_id: cols[0].clone(),
            title: "e2e".into(),
            description: None,
            tags: vec![],
            linked_files: vec![],
        },
    )
    .await
    .expect("t");
    pool.close().await;

    let shutdown = Arc::new(tokio::sync::Notify::new());
    let shutdown_clone = shutdown.clone();
    let cfg = ServeConfig {
        bind: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0),
        db_path: db_path.clone(),
        repo_root: dir.path().to_path_buf(),
        worktree_root: dir.path().join(".kulisawit/worktrees"),
        runtime: RuntimeConfig::default(),
    };
    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
    let server_handle = tokio::spawn(async move {
        serve_with_shutdown_ready(cfg, async move { shutdown_clone.notified().await }, Some(ready_tx)).await
    });
    let addr = tokio::time::timeout(Duration::from_secs(3), ready_rx)
        .await
        .expect("server ready")
        .expect("addr");

    let client = reqwest::Client::builder()
        .build()
        .expect("client");
    let base = format!("http://{addr}");

    // Dispatch 2 attempts.
    let resp = client
        .post(format!("{base}/api/tasks/{}/dispatch", task_id.as_str()))
        .json(&serde_json::json!({"agent":"mock","batch":2}))
        .send()
        .await
        .expect("dispatch");
    assert_eq!(resp.status(), reqwest::StatusCode::OK);
    let body: serde_json::Value = resp.json().await.expect("json");
    let ids: Vec<String> = body["attempt_ids"]
        .as_array()
        .expect("ids")
        .iter()
        .map(|v| v.as_str().unwrap().to_owned())
        .collect();
    assert_eq!(ids.len(), 2);

    // Subscribe to SSE for the first attempt; collect events until the stream ends
    // or 10 seconds elapse.
    let url = format!("{base}/api/attempts/{}/events", ids[0]);
    let resp = client.get(&url).send().await.expect("sse");
    assert_eq!(resp.status(), reqwest::StatusCode::OK);
    assert_eq!(
        resp.headers().get(reqwest::header::CONTENT_TYPE).map(|v| v.to_str().unwrap_or("")),
        Some("text/event-stream")
    );

    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    let mut bytes_stream = resp.bytes_stream();
    let mut collected = String::new();
    while tokio::time::Instant::now() < deadline {
        match tokio::time::timeout(Duration::from_millis(500), bytes_stream.next()).await {
            Ok(Some(Ok(chunk))) => {
                collected.push_str(&String::from_utf8_lossy(&chunk));
                if collected.contains("\"Completed\"") || collected.contains("\"completed\"") {
                    break;
                }
            }
            Ok(Some(Err(e))) => panic!("stream error: {e}"),
            Ok(None) => break, // stream closed
            Err(_) => continue, // timeout tick; keep waiting for more chunks
        }
    }
    assert!(
        collected.contains("\"Completed\"") || collected.contains("\"completed\""),
        "SSE did not see terminal Completed event within 10s:\n{collected}"
    );

    // Poll /api/attempts/:id for both ids; expect Completed.
    for id in &ids {
        let resp = client
            .get(format!("{base}/api/attempts/{id}"))
            .send()
            .await
            .expect("get attempt");
        assert_eq!(resp.status(), reqwest::StatusCode::OK);
        let body: serde_json::Value = resp.json().await.expect("json");
        assert_eq!(body["status"], "completed");
    }

    // Shut down.
    shutdown.notify_one();
    tokio::time::timeout(Duration::from_secs(5), server_handle)
        .await
        .expect("drain within 5s")
        .expect("join")
        .expect("serve ok");
}
```

- [ ] **Step 3: Run to verify**

```bash
cargo test -p kulisawit-server --test e2e --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --check
```

Expected: 1 e2e test pass; full workspace green.

- [ ] **Step 4: Commit**

```bash
git add crates/kulisawit-server/
git commit -m "feat(server): end-to-end smoke via reqwest"
```

---

### Task 3.1.13: Tag `phase-3.1`

**Files:** none (git-only).

- [ ] **Step 1: Confirm the green bar**

```bash
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --check
cargo build --workspace --all-targets --locked
```

All must pass.

- [ ] **Step 2: Count tests**

```bash
cargo test --workspace --locked 2>&1 | grep -E "^test result:" | awk '{s += $4} END {print s}'
```

Record the number.

- [ ] **Step 3: Create the annotated tag**

```bash
PHASE2_HEAD=$(git rev-list -n1 phase-2 2>/dev/null || echo "phase-2-missing")
HEAD_SHA=$(git rev-parse --short HEAD)
PHASE31_TESTS=$(cargo test --workspace --locked 2>&1 | grep -E "^test result:" | awk '{s += $4} END {print s}')
git tag -a phase-3.1 -m "Kulisawit Phase 3.1 — HTTP + SSE server.

Delta from phase-2 (${PHASE2_HEAD:0:7}):
- New crate: kulisawit-server (axum router, graceful shutdown)
- 6 JSON endpoints + 1 SSE endpoint under /api
- New: kulisawit_orchestrator::dispatch_batch_spawned
- CLI: kulisawit serve --db --repo --port
- End-to-end reqwest smoke test

Tests: ${PHASE31_TESTS} total (phase-2 baseline: 87).
HEAD: ${HEAD_SHA}."
```

Do NOT run `git push` or `git push --tags`. Tagging is local.

- [ ] **Step 4: Verify**

```bash
git tag -l phase-3.1
git show phase-3.1 --stat | head -20
```

Expected: annotated tag visible with the delta summary.

---

## Self-review checklist

Work through each item after all tasks land; every box must be checked before calling Phase 3.1 complete.

- [ ] **Spec coverage.** Each of the six JSON + one SSE endpoints has a task. The new `dispatch_batch_spawned` function has a task. The CLI integration has a task. E2e has a task. ✓
- [ ] **No authentication / cloud / Electron.** None introduced. ✓
- [ ] **No breaking change to `kulisawit run`.** Phase 2 test `cli_run` runs untouched; verify the full workspace test still passes after Task 3.1.7 (the only change that touches the orchestrator).
- [ ] **No `#![deny(...)]` attributes** in any new file; workspace lints govern.
- [ ] **No `Co-Authored-By:` lines** in any commit message in this plan.
- [ ] **English generic identifiers** everywhere in code; Kulisawit vocabulary not introduced in Phase 3.1.
- [ ] **Exit criteria satisfied** (see top of plan).
- [ ] **All code is verbatim, paste-ready.** No `// TBD`, no `// similar to above`.
- [ ] **Import paths:** every new file imports `kulisawit_core` types from the crate root, never via `adapter::`.

---

## Critical files for implementation

- `/home/bimbim/works/kulisawit/docs/superpowers/specs/2026-04-18-kulisawit-phase-3.1-server-sse-design.md` (design)
- `/home/bimbim/works/kulisawit/crates/kulisawit-server/src/lib.rs`
- `/home/bimbim/works/kulisawit/crates/kulisawit-server/src/routes/mod.rs`
- `/home/bimbim/works/kulisawit/crates/kulisawit-orchestrator/src/dispatch.rs`
- `/home/bimbim/works/kulisawit/crates/kulisawit-cli/src/main.rs`
