# 🌴 Kulisawit

> A plantation of AI workers for your codebase. Open-source, local-first orchestration tool for solo developers to run, compare, and harvest multiple AI coding agents in parallel — built in Rust.

![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue) ![Rust 1.86+](https://img.shields.io/badge/rust-1.86%2B-orange) ![Status: pre-alpha](https://img.shields.io/badge/status-pre--alpha-red) ![Tests: 110 passing](https://img.shields.io/badge/tests-110%20passing-green)

> **⚠ Pre-alpha — Phase 3.1 of 6 complete.** HTTP+SSE backend works end-to-end. UI (Phase 3.2), sortir runner, and panen merge (Phase 3.3) are not implemented yet. See [Roadmap](#roadmap) for what ships today vs what's planned. **Not ready for daily use.**

## Table of Contents

- [Quickstart](#quickstart)
- [Architecture](#architecture)
- [Plantation Glossary](#plantation-glossary)
- [Feature Matrix](#feature-matrix)
- [HTTP API Catalog](#http-api-catalog)
- [Database Schema](#database-schema)
- [Testing](#testing)
- [Release Plan](#release-plan)
- [Roadmap](#roadmap)
- [Contributing](#contributing)
- [Documentation Index](#documentation-index)
- [License](#license)
- [Acknowledgments](#acknowledgments)

## Quickstart

**Prerequisites**

- Rust 1.86+ — `rustup install 1.86 && rustup default 1.86`
- Git
- A target git repository to point Kulisawit at (read+write access). Kulisawit creates worktrees inside this repo's directory tree, so it must be writable.

**Build & run**

```bash
git clone https://github.com/BimaPangestu28/kulisawit
cd kulisawit
cargo build --release
./target/release/kulisawit serve \
  --db ./kulisawit.db \
  --repo /path/to/your/repo \
  --port 7700
```

The first build takes a few minutes (it pulls and compiles `git2` with vendored libgit2, plus the full async stack). On startup, `kulisawit serve` runs `migrations/0001_initial.sql` against the SQLite file specified by `--db`, creating it if absent. The server prints `kulisawit-server listening on 127.0.0.1:7700` once ready.

**Verify the server is alive**

```bash
# 1. Create a project pointing at your repo
curl -sX POST http://localhost:7700/api/projects \
  -H 'content-type: application/json' \
  -d '{"name":"demo","repo_path":"/path/to/your/repo"}'
# → {"id":"<project-id>","name":"demo","repo_path":"...","created_at":...}

# 2. Create a task on a column
#    (Default columns are seeded with the project — list via DB or use the IDs
#     from the project-creation response in a future release.)
curl -sX POST http://localhost:7700/api/tasks \
  -H 'content-type: application/json' \
  -d '{"project_id":"<project-id>","column_id":"<column-id>","title":"hello"}'
# → {"id":"<task-id>",...}

# 3. Plant a tandan of attempts (mock agent is registered for testing)
curl -sX POST http://localhost:7700/api/tasks/<task-id>/dispatch \
  -H 'content-type: application/json' \
  -d '{"agent":"mock","batch":2}'
# → {"attempt_ids":["<id-1>","<id-2>"]}

# 4. Stream attempt events as SSE
curl -N http://localhost:7700/api/attempts/<id-1>/events
```

> **No CLI subcommands for `tanam` / `panen` yet.** Only `serve` is wired in Phase 3.1. Drive the server via the HTTP API directly while Phase 3.2 (UI) is in flight. The mock kuli adapter is the only currently-shipping agent — real adapters land in Phase 4.

## Architecture

Kulisawit is a Cargo workspace of 7 crates plus an embedded React UI (post-Phase 3.2). The CLI is the binary entry point; the orchestrator owns task lifecycle; the server exposes both over HTTP+SSE; lower crates wrap SQLite, git, and agent subprocesses.

```text
┌─────────────────────────────────────────────┐
│  kulisawit-cli  (binary entrypoint, clap)   │
└──────┬──────────────────────────┬───────────┘
       │                          │
       ▼                          ▼
┌─────────────┐         ┌──────────────────┐
│  -server    │◄────────┤  -orchestrator   │
│  (axum+SSE) │         │  (dispatch_batch)│
└──────┬──────┘         └──┬──────────┬────┘
       │                   │          │
       ▼                   ▼          ▼
┌────────┐  ┌────────┐  ┌────────┐  ┌────────┐
│  -db   │  │  -git  │  │ -agent │  │ -core  │
│ (sqlx) │  │ (git2) │  │ (kuli) │  │(types) │
└────────┘  └────────┘  └────────┘  └────────┘
```

| Crate | Responsibility | Key deps | Status |
|---|---|---|---|
| `kulisawit-cli` | Binary entry point, clap subcommand parsing, in-process server launcher | clap, tokio | ✅ `serve` only |
| `kulisawit-server` | axum HTTP + SSE, request validation, response shaping | axum, tower-http | ✅ Phase 3.1 |
| `kulisawit-orchestrator` | Dispatch logic, attempt lifecycle, broadcast event channels | tokio, broadcast | ✅ Phase 2 |
| `kulisawit-agent` | Kuli adapter trait + mock reference impl (subprocess management forthcoming) | tokio process | 🚧 trait + mock only |
| `kulisawit-db` | sqlx pool, migrations, repository functions for project / task / attempt / events | sqlx | ✅ Phase 1 |
| `kulisawit-git` | git2 wrappers for worktree create/list/remove and branch+commit ops | git2 | ✅ Phase 1 |
| `kulisawit-core` | Shared domain types (`Project`, `Task`, `Attempt`, `AgentEvent`, `AttemptStatus`, `RunStatus`, `AttemptId`, `ProjectId`, `TaskId`, `ColumnId`) | serde, uuid, chrono | ✅ Phase 1 |

Status legend: ✅ shipped · 🚧 partial / in progress · ⏳ planned.

## Plantation Glossary

Kulisawit uses a plantation metaphor consistently throughout the codebase, UI, and documentation. This is intentional branding. Contributors must preserve this vocabulary in user-facing surfaces.

| Technical concept | Kulisawit term | Meaning |
|---|---|---|
| Project | Kebun | Plantation (a git repo tracked by Kulisawit) |
| Card / task | Lahan | Plot of land (a unit of work) |
| Parallel attempts group | Tandan | Fruit cluster (N parallel attempts on one lahan) |
| Single attempt | Buah | Fruit (one agent execution) |
| Agent worker | Kuli | Worker (an AI agent running inside a buah) |
| Run attempts | Tanam | Plant (trigger execution) |
| Verification step | Sortir | Sorting (post-run checks) |
| Merge winning attempt | Panen | Harvest (merge selected buah to main) |
| Worktree directory | Petak | Land parcel (isolated git worktree) |
| Config file | Peta kebun | Plantation map |
| Daemon process | Mandor | Foreman (the orchestrator daemon) |
| Event log | Buku kuli | Worker's log |
| Garbage collection | Bersihkan | Cleanup |

> **Naming convention.** Code identifiers use English (`task`, `attempt`, `worktree`); user-facing strings (CLI output, UI labels, future docs) use Kulisawit terms (`lahan`, `buah`, `petak`). This separation keeps internals readable for international contributors while preserving the brand in UX.

## Feature Matrix

**Phase progress** (each completed phase is annotated with a git tag):

| Phase | Scope | Status | Tag |
|---|---|---|---|
| 1 | Foundation: workspace, db migrations, git ops | ✅ Done | `phase-1` |
| 2 | Orchestrator + CLI `run` | ✅ Done | `phase-2` |
| 3.1 | HTTP + SSE server | ✅ Done | `phase-3.1` |
| 3.2 | React UI minimum (kanban + diff viewer) | 🚧 Next | — |
| 3.3 | Sortir runner + panen merge | ⏳ Planned | — |
| 4 | Adapter ecosystem (Codex, Aider, Gemini) | ⏳ Planned | — |
| 5 | Single-binary release (cargo-dist) | ⏳ Planned | — |
| 6 | v0.1 launch polish | ⏳ Planned | — |

**Per-feature** status (PRD §3.1 F1–F12):

| ID | Feature | Status |
|---|---|---|
| F1 | Kebun init from any git repo | ✅ |
| F2 | Kanban board (default columns) | ⏳ |
| F3 | Lahan CRUD (title, description, tags, linked files) | ✅ create + fetch (no list endpoint yet) |
| F4 | Single buah execution with worktree isolation | ✅ |
| F5 | Live SSE streaming of kuli output | ✅ |
| F6 | Tandan: N parallel buah on one lahan | ✅ |
| F7 | Side-by-side diff view across buah | ⏳ |
| F8 | Claude Code adapter (reference) | 🚧 stub |
| F9 | Sortir hooks (test/lint/build commands) | ⏳ |
| F10 | Panen: merge winning buah to main | ⏳ |
| F11 | SQLite persistence | ✅ |
| F12 | Single-binary install with embedded UI | ⏳ |

## HTTP API Catalog

Phase 3.1 ships a JSON HTTP API plus one SSE stream under `/api`. All endpoints are local-only — **no auth, no rate limiting, no CORS preflight** beyond the default. Endpoint shapes are stable within Phase 3.1 but may change before v0.1.

> **Heads-up: no listing endpoints yet.** `GET /api/projects` (all projects) and `GET /api/projects/:id/tasks` (tasks for a project) are not implemented in Phase 3.1; they're planned for Phase 3.2 once the UI needs them. Read directly from the SQLite database for bulk queries until then.

### Projects (kebun)

#### `POST /api/projects`

Create a project pointing at a local git repo.

```json
// Request
{"name": "demo", "repo_path": "/abs/path/to/repo"}

// Response 200
{"id": "<ProjectId>", "name": "demo", "repo_path": "/abs/path/to/repo", "created_at": 1745000000}
```

#### `GET /api/projects/:id`

Fetch a single project by ID.

```json
// Response 200
{"id": "<ProjectId>", "name": "demo", "repo_path": "/abs/path/to/repo", "created_at": 1745000000}

// Response 404 if no project with that ID
```

### Tasks (lahan)

#### `POST /api/tasks`

Create a task in a column. `tags` and `linked_files` default to empty arrays.

```json
// Request
{
  "project_id": "<ProjectId>",
  "column_id": "<ColumnId>",
  "title": "Refactor parser",
  "description": "Optional",
  "tags": ["refactor", "parser"],
  "linked_files": ["src/parser.rs"]
}

// Response 200
{
  "id": "<TaskId>",
  "project_id": "<ProjectId>",
  "column_id": "<ColumnId>",
  "title": "Refactor parser",
  "description": "Optional",
  "position": 0,
  "tags": ["refactor", "parser"],
  "linked_files": ["src/parser.rs"],
  "created_at": 1745000000,
  "updated_at": 1745000000
}
```

#### `GET /api/tasks/:id`

Fetch a single task.

#### `POST /api/tasks/:id/dispatch`

Plant a tandan of N attempts on a task. Returns immediately with attempt IDs; the kuli runs detach via `tokio::spawn`.

```json
// Request
{"agent": "mock", "batch": 3, "variants": ["aggressive", "conservative", "default"]}
// `variants` is optional. When supplied, length must equal `batch`; each
// attempt uses the variant at the matching index for prompt rendering.

// Response 200
{"attempt_ids": ["<AttemptId>", "<AttemptId>", "<AttemptId>"]}
```

### Attempts (buah)

#### `GET /api/attempts/:id`

Snapshot of an attempt's current state.

```json
// Response 200
{
  "id": "<AttemptId>",
  "task_id": "<TaskId>",
  "agent_id": "mock",
  "status": "running",
  "prompt_variant": "aggressive",
  "worktree_path": "/abs/path/to/repo/.kulisawit/worktrees/<id>",
  "branch_name": "kulisawit/<id>",
  "started_at": 1745000000,
  "completed_at": null
}
```

`status` is one of `queued`, `running`, `completed`, `failed`, `cancelled`.

#### `GET /api/attempts/:id/events`

Server-Sent Events stream of `AgentEvent`s for one attempt.

- On connect, the server replays past events from the `events` table.
- After the replay, it streams live events from the broadcast channel.
- On terminal status (`completed` / `failed` / `cancelled`), the broadcast channel closes and the SSE stream ends. Clients should treat stream close as a reliable signal that DB state is final.

```text
# Curl example
curl -N http://localhost:7700/api/attempts/<AttemptId>/events

# Sample frames (each is one SSE event)
data: {"type":"status","status":"running","at":1745000000}

data: {"type":"stdout","chunk":"compiling..."}

data: {"type":"status","status":"completed","at":1745000005}
```

### Response shape reference

All response shapes mirror types in `crates/kulisawit-core/src/` and `crates/kulisawit-server/src/wire.rs`. Read those files for ground truth: `Project`, `Task`, `Attempt`, `AgentEvent`, `AttemptStatus`, `DispatchRequest`, `DispatchResponse`.

## Database Schema

SQLite via `sqlx`. Migrations live in [`migrations/`](migrations/). All primary keys are UUID v7 stored as `TEXT`. `kulisawit serve --db <path>` runs migrations on startup against the target file (creating it if absent).

| Table | Purpose | Key columns | Foreign keys |
|---|---|---|---|
| `project` | Tracked git repos (kebun) | `id`, `name`, `repo_path`, `created_at` | — |
| `columns` | Kanban columns per project | `id`, `project_id`, `name`, `position` | `project_id → project(id)` |
| `task` | Work units (lahan) | `id`, `project_id`, `column_id`, `title`, `description`, `position`, `tags` (JSON array), `linked_files` (JSON array), `created_at`, `updated_at` | `project_id → project(id)`, `column_id → columns(id)` |
| `attempt` | Single agent runs (buah) | `id`, `task_id`, `agent_id`, `prompt_variant`, `worktree_path`, `branch_name`, `status` (`queued` / `running` / `completed` / `failed` / `cancelled`), `started_at`, `completed_at`, `verification_status` (`pending` / `passed` / `failed` / `skipped`), `verification_output` | `task_id → task(id)` |
| `events` | Per-attempt event log (replay source for SSE) | `id` (autoincrement), `attempt_id`, `timestamp`, `type`, `payload` (JSON) | `attempt_id → attempt(id)` |

Indexes, exact column types, and CHECK constraints live in [`migrations/0001_initial.sql`](migrations/0001_initial.sql). That file is the ground truth.

## Testing

**110 tests passing** across the workspace as of tag `phase-3.1`.

| Crate | Tests | Categories |
|---|---|---|
| `kulisawit-core` | 19 | unit: type roundtrips, serde, ID newtypes |
| `kulisawit-db` | 22 | unit + integration: repo functions against tempfile SQLite, migrations, concurrent inserts |
| `kulisawit-git` | 7 | integration: real `git2` repos in tempdirs (worktree create / list / remove, branch + commit) |
| `kulisawit-orchestrator` | 35 | unit + integration: dispatch lifecycle, broadcast, cancellation, prompt rendering, agent registry |
| `kulisawit-server` | 19 | integration + e2e: axum router, request validation, SSE replay+live, reqwest end-to-end on ephemeral port |
| `kulisawit-agent` | 5 | unit: adapter trait object-safety, mock stream contract |
| `kulisawit-cli` | 3 | smoke: `--help` rendering, `kulisawit run` argument parsing |

**Test categories:**

- **Unit** — co-located in `src/` files via `#[cfg(test)] mod tests`.
- **Integration** — in `tests/` per crate; hit real SQLite tempfiles and real `git2` repos in tempdirs. No mocks for the DB or for git.
- **End-to-end** — `crates/kulisawit-server/tests/e2e.rs` spins the server on an ephemeral port via a oneshot ready channel, drives the full task lifecycle with `reqwest`, and drains SSE to terminal close.

**Lints**

Workspace-level lints **deny** `clippy::unwrap_used`, `clippy::expect_used`, and `clippy::panic` in production code. Tests opt in via `#[allow(clippy::expect_used)]` at function or module scope, with a `// Rationale: ...` comment when the rationale isn't obvious.

**How to run**

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

## Release Plan

- **Goal:** single static binary ~15 MB via [`cargo-dist`](https://github.com/axodotdev/cargo-dist), embedded React UI via `rust-embed` (wired in Phase 5).
- **Targets:** `x86_64-unknown-linux-gnu`, `aarch64-apple-darwin`, `x86_64-pc-windows-msvc`. macOS Intel and Linux ARM64 considered for v0.1+.
- **Distribution:** GitHub Releases plus a `curl | sh` install script. Explicitly **not shipping** to npm, brew, or apt for v0.1.
- **Status:** not wired yet. `cargo build --release` produces a working ~12 MB stripped server binary today; UI embed will increase the size once Phase 3.2 + Phase 5 land.

## Roadmap

Kulisawit follows a 6-phase plan toward v0.1 launch. Each completed phase is tagged in git on the `main` branch.

| Phase | Scope | Status | Tag |
|---|---|---|---|
| 1 | Foundation | ✅ Done | `phase-1` |
| 2 | Orchestrator + CLI `run` | ✅ Done | `phase-2` |
| 3.1 | HTTP + SSE server | ✅ Done | `phase-3.1` |
| 3.2 | React UI minimum | 🚧 Next | — |
| 3.3 | Sortir runner + panen merge | ⏳ Planned | — |
| 4 | Adapter ecosystem | ⏳ Planned | — |
| 5 | Single-binary release | ⏳ Planned | — |
| 6 | v0.1 launch polish | ⏳ Planned | — |

- **Now (April 2026):** Phase 3.2 — React UI minimum: kanban board, lahan detail panel, per-buah SSE log viewer.
- **Next:** Phase 3.3 (sortir runner + panen merge), then Phase 4 (additional kuli adapters: Codex, Aider, Gemini CLI).
- **v0.1 launch target:** Q3 2026 (calendar TBD; PRD §13 estimates a 6-week MVP from the start of UI work).

**Post-v0.1 exploration** (PRD §3.2 / §3.3, marked clearly speculative):

- Timeline scrubbing per buah (replay mode)
- Fork-from-step: branch execution from a mid-point with new instructions
- WASM plugin system for custom sortir runners and context providers
- MCP server integration as a context source
- Auto-context: embed repo files, surface relevant ones per lahan

Detailed plans live in [`docs/superpowers/plans/`](docs/superpowers/plans/) — one design doc per phase.

## Contributing

Kulisawit is solo-built by [@BimaPangestu28](https://github.com/BimaPangestu28) but designed for outside contribution from day one.

**Three high-leverage areas:**

1. **New kuli adapter.** Implement the adapter trait in `crates/kulisawit-agent`. Required: subprocess lifecycle (spawn, stream stdout/stderr, capture exit), structured event emission (tool calls, status transitions), error mapping into `AgentError`. Wishlist: Codex CLI, Aider, Gemini CLI, Cursor agent. Reference: the mock adapter (`mock_stream` test binary) shows the trait contract; a Claude Code adapter is the next planned reference impl.
2. **New sortir runner.** Phase 3.3 will define the sortir hook contract (test/lint/build commands run after each buah). Once that lands, ecosystem-specific runners (pytest, jest, golangci-lint, mypy) are direct contributions.
3. **Bug reports, naming, and docs.** File issues for anything broken or unclear. Plantation-metaphor naming suggestions for new concepts are explicitly welcome — better Indonesian terms for existing PRD entries especially.

**Dev workflow**

```bash
# fork on GitHub, then
git clone https://github.com/<your-username>/kulisawit
cd kulisawit
git remote add upstream https://github.com/BimaPangestu28/kulisawit.git
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
# branch, commit, push, open a pull request
```

**Code style**

- `rustfmt` defaults (no overrides); `cargo fmt --all` before committing.
- Naming convention per PRD §0: English code identifiers, Kulisawit terms in user-facing strings only.
- No `unwrap()` / `expect()` / `panic!()` in non-test code (workspace lints enforce this; tests opt in with `#[allow(...)]` plus a rationale comment when the reason isn't obvious).
- Commits: Conventional Commits (`feat(scope): …`, `fix(scope): …`, `docs(scope): …`).

A formal `CONTRIBUTING.md` lands closer to v0.1. For now, ask in Issues if anything's unclear.

## Documentation Index

- [`docs/PRD.md`](docs/PRD.md) — full product requirements (608 lines, source of truth for vision and scope)
- [`docs/superpowers/specs/`](docs/superpowers/specs/) — per-phase design docs (one file per phase, dated)
- [`docs/superpowers/plans/`](docs/superpowers/plans/) — per-phase implementation plans
- [`migrations/`](migrations/) — SQLite schema (ground truth for the database section above)
- This README — entry point + reference

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

## Acknowledgments

**Prior art.** [Vibe Kanban](https://github.com/), [Conductor](https://github.com/), and [Crystal](https://github.com/) — the agent-orchestration tools that motivated this project (PRD §1.1). Kulisawit owes the kanban-as-orchestrator framing to all three; the multi-attempt and worktree-isolation primitives are our contribution.

**Stack.** Built on [Tokio](https://tokio.rs), [Axum](https://github.com/tokio-rs/axum), [sqlx](https://github.com/launchbadge/sqlx), [git2-rs](https://github.com/rust-lang/git2-rs), and [rust-embed](https://github.com/pyrossh/rust-embed). The Rust ecosystem makes single-binary local-first tools genuinely easy.

**Plantation metaphor.** Named in honor of palm-plantation workers in Indonesia. The metaphor is a tribute, not a costume.
