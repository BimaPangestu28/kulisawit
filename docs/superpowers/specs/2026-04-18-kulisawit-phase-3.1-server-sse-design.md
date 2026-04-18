# Kulisawit Phase 3.1 — Server + SSE Design Spec

**Date:** 2026-04-18
**Status:** Accepted (brainstorming approved 2026-04-18)
**Predecessor:** Phase 2 complete (tag `phase-2` at `3475031`)
**Successor:** Phase 3.2 (minimal UI, separate spec)

---

## Goal

Add an in-process HTTP server to Kulisawit so a browser-based UI (Phase 3.2) and external scripts can create projects/tasks, dispatch attempts, and consume live agent event streams. Ship the server as a new `kulisawit serve` subcommand that coexists with the existing `kulisawit run` headless entry point.

## Non-goals

- No authentication, sessions, or user accounts (PRD §13.4).
- No cloud sync, multi-host, or non-localhost binding.
- No kanban UI, comparison view, or diff viewer — all deferred to Phase 3.2.
- No `sortir` (test/lint runner) or `panen` (merge) — deferred to Phase 3.3.
- No breaking changes to `kulisawit run`: the Phase 2 CLI e2e test (`cli_run.rs`) must still pass unchanged.

---

## Scope — API surface (minimalist)

Six endpoints total. Rationale: build UI-driven in Phase 3.2; expand surface only when the UI demonstrably needs it.

| Method | Path | Purpose |
| --- | --- | --- |
| `POST` | `/api/projects` | Create a project (wraps `kulisawit_db::project::create`). |
| `GET`  | `/api/projects/:id` | Read a project by id. |
| `POST` | `/api/tasks` | Create a task (wraps `kulisawit_db::task::create`). |
| `GET`  | `/api/tasks/:id` | Read a task by id. |
| `POST` | `/api/tasks/:id/dispatch` | Dispatch N attempts; returns `AttemptId`s immediately (async semantics). |
| `GET`  | `/api/attempts/:id` | Read an attempt row by id. |
| `GET`  | `/api/attempts/:id/events` | Server-Sent Events stream of live `AgentEvent`s for the attempt. |

The SSE endpoint shares a path prefix with the attempt GET; routing-wise they are separate routes. Counting by distinct functions: 7 (six JSON + one SSE). Informally documented as "six" in the Q1 brainstorming.

---

## Architecture

```
┌──────────────────────────────────────────────────────┐
│ kulisawit serve process                              │
│                                                      │
│   ┌─────────────┐      ┌──────────────────────────┐  │
│   │  Axum app   │─────▶│ Arc<Orchestrator>        │  │
│   │  + tracing  │      │  ├─ Arc<DbPool>          │  │
│   │  + graceful │      │  ├─ Arc<AgentRegistry>   │  │
│   │    shutdown │      │  ├─ Arc<EventBroadcaster>│  │
│   └─────────────┘      │  └─ Arc<Semaphore>       │  │
│          │             └──────────────────────────┘  │
│          │                         ▲                 │
│          ▼                         │                 │
│   HTTP 127.0.0.1:3000              │ tokio::spawn   │
│                                    │ per dispatch    │
└──────────────────────────────────────────────────────┘
```

**Constraints:**
- Bind `127.0.0.1` only. Port default `3000`, override via `--port`.
- `axum::Router` as the single router. Composition: one module per endpoint group.
- State = `Arc<Orchestrator>` via `axum::extract::State<AppState>`.
- Shutdown: Ctrl-C → `axum::serve(...).with_graceful_shutdown(...)` → up to 5s drain for in-flight SSE streams, then hard close.

**CLI integration:** two independent entry points.
- `kulisawit serve --db X --repo Y --port 3000` → starts the HTTP server + orchestrator in-process.
- `kulisawit run ...` → unchanged. Headless dispatch. Not a client of the server.

No inter-process communication between the two modes. Each process owns its own DB pool.

---

## Crate layout

```
crates/kulisawit-server/
├── Cargo.toml              # axum 0.7, tower-http, serde, kulisawit-{core,db,agent,orchestrator}
└── src/
    ├── lib.rs              # `pub async fn serve(config: ServeConfig) -> ServerResult<()>`
    ├── state.rs            # AppState { orch: Arc<Orchestrator> }
    ├── error.rs            # ServerError + axum IntoResponse
    ├── routes/
    │   ├── mod.rs          # composes Router
    │   ├── projects.rs     # POST + GET /api/projects
    │   ├── tasks.rs        # POST + GET /api/tasks, POST /api/tasks/:id/dispatch
    │   └── attempts.rs     # GET /api/attempts/:id, GET /api/attempts/:id/events
    └── wire.rs             # serde DTOs

crates/kulisawit-cli/src/commands/
└── serve.rs                # parses flags, calls kulisawit_server::serve()
```

**Public surface of `kulisawit-server`:**

```rust
pub struct ServeConfig {
    pub bind: SocketAddr,              // default 127.0.0.1:3000
    pub db_path: PathBuf,
    pub repo_root: PathBuf,
    pub worktree_root: PathBuf,        // defaults to repo_root/.kulisawit/worktrees
    pub runtime: RuntimeConfig,
}

pub async fn serve(config: ServeConfig) -> ServerResult<()>;
```

---

## Error taxonomy

`ServerError` enum + `impl IntoResponse`:

| Variant | HTTP status | Wire body |
| --- | --- | --- |
| `NotFound { entity: &'static str, id: String }` | 404 | `{"error": "not_found", "entity": "task", "id": "..."}` |
| `InvalidInput(String)` | 400 | `{"error": "invalid_input", "message": "..."}` |
| `Conflict(String)` | 409 | `{"error": "conflict", "message": "..."}` |
| `Internal(String)` | 500 | `{"error": "internal"}` (no detail leak; full chain goes to `tracing::error!`) |

`ServerError::Internal` receives the `#[from]` impls for `OrchestratorError`, `DbError`, `GitError`, `io::Error`. The chain is logged via `tracing` with the full debug representation so operators have diagnostics, while clients get a stable opaque 500.

---

## Wire format

**Encoding:** JSON, UTF-8, `snake_case` field names (Rust-default, zero-config).

**Requests:**

```rust
#[derive(Deserialize)]
pub struct NewProjectRequest {
    pub name: String,
    pub repo_path: String,
}

#[derive(Deserialize)]
pub struct NewTaskRequest {
    pub project_id: ProjectId,
    pub column_id: ColumnId,
    pub title: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub linked_files: Vec<String>,
}

#[derive(Deserialize)]
pub struct DispatchRequest {
    pub agent: String,
    pub batch: usize,
    pub variants: Option<Vec<String>>,
}
```

**Responses:**

```rust
#[derive(Serialize)]
pub struct ProjectResponse {
    pub id: ProjectId,
    pub name: String,
    pub repo_path: String,
    pub created_at: i64,
}

#[derive(Serialize)]
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

#[derive(Serialize)]
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

#[derive(Serialize)]
pub struct DispatchResponse {
    pub attempt_ids: Vec<AttemptId>,
}
```

**SSE payload** (one event per `data: ...\n\n` frame):

```rust
#[derive(Serialize)]
pub struct EventEnvelope {
    pub attempt_id: AttemptId,
    pub event: AgentEvent,
    pub ts_ms: i64,
}
```

`AgentEvent` is `kulisawit_core::AgentEvent` — already `Serialize` per Phase 2. The envelope adds `attempt_id` (redundant with URL path but useful for multi-attempt multiplexing later) and `ts_ms` (server-side timestamp for client ordering).

---

## Data flow — `POST /api/tasks/:id/dispatch`

This is the load-bearing path. All other handlers are thin DB passthroughs.

```
Client ──POST {"agent":"mock","batch":3}──▶ tasks::dispatch handler
                                                    │
                                                    ▼
                                    validate task_id exists (db::task::get)
                                                    │
                                                    ▼
                        orch.dispatch_batch_spawned(task_id, agent, batch, variants)
                        ^ new function (see below) — returns after all
                          attempt rows inserted, agent runs detached.
                                                    │
                                                    ▼
                                 return 202 {"attempt_ids": [...]}
```

**Required change to `kulisawit-orchestrator`:** add a new public function

```rust
pub async fn dispatch_batch_spawned(
    orch: &Arc<Orchestrator>,
    task_id: &TaskId,
    agent_id: &str,
    batch_size: usize,
    variants: Option<Vec<String>>,
) -> OrchestratorResult<Vec<AttemptId>>;
```

**Semantics:**
1. Validate `batch_size ≥ 1` and `variants.len() == batch_size` if provided (same as `dispatch_batch`).
2. For each index `i` in `0..batch_size`:
    - Spawn a `tokio::task` that clones the `Arc<Orchestrator>` and calls `dispatch_single_attempt(...)`.
    - Each spawned task also inserts its own attempt row (currently done inside `dispatch_single_attempt` at step 7).
3. Collect `AttemptId`s by having each spawned task send its freshly-generated id via an `mpsc` or by reserving the ids *before* spawn.

**Id-reservation choice:** spawned tasks currently generate their `AttemptId` inside `attempt::create(...)`. To return the ids before the agents run we have two options:

- **Option A (preferred):** two-phase in the existing helper. Add `dispatch_single_attempt_spawn_after_reserve` that runs steps 1–7 (reserve the DB row + install the cancel flag) synchronously, then spawns the remaining work (`mark_running` onward). Return the `AttemptId` after step 7 and before the spawn. The server's `dispatch_batch_spawned` awaits each reservation, collects ids, returns 202. Agents complete in the background.

- Option B: refactor `attempt::create` to accept a pre-generated `AttemptId` so the server can allocate ids upfront and spawn fully-detached tasks.

Option A is recommended because it keeps id generation inside the `attempt::create` insert (single source of truth, matches Phase 2 behavior) at the cost of one more helper function. Option B touches the DB layer for a server-only concern.

**CLI `kulisawit run` remains unaffected** — it keeps using the existing `dispatch_batch` which awaits all agents to completion.

---

## SSE stream — `GET /api/attempts/:id/events`

```
Client ──GET──▶ handler
                   │
                   ▼
          validate attempt_id exists (db::attempt::get)
                   │
                   ▼
          orch.broadcaster().subscribe(&attempt_id)
          ^ returns a tokio::sync::broadcast::Receiver<AgentEvent>
                   │
                   ▼
          axum::response::sse::Sse::new(stream)
          – map each AgentEvent to axum::response::sse::Event::default().data(<json>)
          – heartbeat via KeepAlive every 15s
          – stream ends naturally when broadcaster drops the channel
            (the dispatcher calls broadcaster.close on terminal transition)
          – client disconnect → stream dropped, Receiver dropped, no side effect
```

**Edge cases:**
- Subscribing to an attempt that is already in a terminal state: the broadcaster may have already closed its channel. Handler detects this by first checking `attempt.status` from the DB; if terminal, emit a single synthetic "Status" envelope with the final status and end the stream immediately. This avoids a stream that closes with no events ever emitted.
- Subscribing to an attempt that does not exist: 404, no stream.
- `broadcast::Receiver::Lagged` (receiver fell >256 events behind): skip lagged events, continue. Documented in `AttemptId` event stream semantics.

---

## Testing strategy

**Location:** `crates/kulisawit-server/tests/` integration tests.

**Unit-ish (router-level via `tower::ServiceExt::oneshot`):**
1. `POST /api/projects` with valid body → 201 + project row exists in DB.
2. `POST /api/projects` with missing `name` → 400.
3. `GET /api/projects/:id` 404 when id unknown.
4. `POST /api/tasks` with bogus project_id → 400. Handler pre-validates project existence via `db::project::get` before calling `db::task::create`, so the error is `InvalidInput("project not found: <id>")`, not a raw FK violation. Same pattern for bogus column_id.
5. `POST /api/tasks/:id/dispatch` with unknown task → 404.
6. `POST /api/tasks/:id/dispatch` with `batch: 0` → 400.
7. `GET /api/attempts/:id` 200 + shape assertion.

**Lifecycle (multi_thread runtime, `axum::serve` on ephemeral port):**
8. Full e2e: create project → create task → dispatch batch 2 → subscribe to SSE for one attempt → assert ≥5 events received + terminal Status + stream closes cleanly.
9. SSE on already-terminal attempt → single Status envelope + stream closes.
10. SSE on unknown attempt → 404, no stream.
11. Graceful shutdown: start server, open SSE, send shutdown signal, assert drain within 5s.
12. No regression: Phase 2 `cli_run.rs` still passes (this runs by default in workspace test).

**Test count target:** 11 new (in server crate) + 0 regressions. Workspace total target: 87 (phase-2) + 11 = ~98.

---

## Exit criteria (tag `phase-3.1`)

- All 11 server tests green.
- Phase 2 tests unchanged and green (no regression).
- `cargo clippy --workspace --all-targets --locked -- -D warnings` clean.
- `cargo fmt --check` clean.
- `cargo build --workspace --all-targets --locked` clean.
- Manual smoke: `kulisawit serve --db /tmp/k.sqlite --repo /tmp/k-repo` starts cleanly; `curl -N http://127.0.0.1:3000/api/attempts/<id>/events` shows live events; Ctrl-C drains within 5s.
- Annotated tag `phase-3.1` at HEAD, local only, with test-count delta message.

---

## Dependencies to add (`[workspace.dependencies]`)

- `axum = "0.7"` (features: `json`, `macros`, `tokio`, `tower-log`)
- `tower-http = "0.6"` (features: `trace`, `cors` — cors only if needed for 3.2 browser UI)
- `reqwest = "0.12"` (dev-dep, for e2e tests; features `json`, `stream`, `rustls-tls` default-off)

`axum` and `tower-http` are both tokio-based and align with the "Tokio only" PRD rule.

---

## Out of scope (explicit deferrals)

- CORS, auth headers, rate limiting — add in Phase 3.2 only if the browser UI requires.
- OpenAPI / schema docs — not a goal. Wire format is documented here.
- `DELETE` endpoints — Phase 3.2 or later.
- Concurrent SSE multiplexing (one stream for all attempts of a project) — if 3.2 needs it, add there.
- `tracing-opentelemetry`, structured access logs to disk — stay with `tracing_subscriber::fmt()`.

---

## Open questions resolved

- **JSON casing:** `snake_case`.
- **Port:** 3000 default, overridable.
- **Bind:** `127.0.0.1` only.
- **Dispatch semantics:** async (3a — new `dispatch_batch_spawned` function in orchestrator).
- **CLI story:** two independent entry points; `run` untouched.
- **Scope:** minimalist 6 JSON endpoints + 1 SSE endpoint.
- **Decomposition:** Phase 3 split into 3.1 (this spec) + 3.2 (UI) + 3.3 (sortir+panen).

---

## Risks and mitigations

| Risk | Mitigation |
| --- | --- |
| `dispatch_batch_spawned` leaks spawned tasks if server exits before they finish | `tokio::spawn` handles are tracked in a `JoinSet` owned by `AppState`; graceful shutdown awaits up to 5s then hard aborts. |
| SSE client disconnects mid-stream leave orphan receivers | `broadcast::Receiver` drops when the axum response future is cancelled. No leak. |
| Two processes (`serve` + `run`) hit same DB file → SQLite lock contention | Out of scope for 3.1; user's responsibility not to run both against the same DB. Document in `kulisawit serve --help`. |
| Port already in use → cryptic panic | `ServeConfig::bind` failure in `kulisawit_server::serve` returns `ServerError::Io(e)` with a clear message. |
