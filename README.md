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

_Filled in Task 7._

## Database Schema

_Filled in Task 7._

## Testing

_Filled in Task 8._

## Release Plan

_Filled in Task 8._

## Roadmap

_Filled in Task 9._

## Contributing

_Filled in Task 9._

## Documentation Index

_Filled in Task 10._

## License

_Filled in Task 10._

## Acknowledgments

_Filled in Task 10._
