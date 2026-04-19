# README.md — Design Spec

**Date:** 2026-04-19
**Status:** Approved (brainstorming complete, ready for implementation plan)
**Target file:** `/README.md` (project root, currently does not exist)

---

## 1. Goals & Non-Goals

### Goals
- Single entry-point document for the Kulisawit repo on GitHub.
- Serve two audiences in priority order: (1) Rust developers exploring or contributing, (2) curious solo devs evaluating the project.
- Be honest about pre-alpha status (Phase 3.1 of 6 complete) without burying the vision.
- Establish plantation-metaphor branding from the first scroll.
- Self-contained "kitchen-sink" reference: a contributor should not need to read other files to understand the project shape, build it, run the server, contribute a kuli adapter, or know what's coming next.
- Surface a known repo-metadata bug (`Cargo.toml` `workspace.package.repository` points to wrong GitHub URL) for fixing in the same PR as README creation.

### Non-Goals
- Not a marketing landing page (lives elsewhere if/when we make one).
- Not end-user documentation for `kulisawit run` / `panen` workflows — those don't exist in shippable form yet.
- Not a tutorial. Quickstart is enough; deep tutorials wait until v0.1.
- No screenshots, GIFs, or asciinema recordings in this iteration (no UI exists; CLI demo deferred).
- No live shields.io badges (avoids CI dependency before CI exists).

---

## 2. Top-Level Decisions

| Decision | Choice | Rationale |
|---|---|---|
| Status framing | Hybrid — launch structure + pre-alpha banner | User-approved; lets structure stabilize early without making false claims. |
| Language | English body + Indonesian Kulisawit terms | Maximizes international reach (PRD success metric: This Week in Rust); preserves brand per PRD §0. |
| Depth | Kitchen-sink (~700+ lines acceptable) | User-approved; one source of truth during pre-alpha when other docs (CONTRIBUTING, ARCHITECTURE) don't exist yet. |
| Demo section | Skip for now | No UI; CLI demo deferred. Re-add when Phase 3.2 ships. |
| Structure | Technical-first (Approach B) | Primary readers in pre-alpha = developers, not end-users. |
| Emoji policy | One palm tree (🌴) in H1 only; ✅🚧⏳⚠ allowed in status tables | Single anchor; avoids generic AI-emoji aesthetic. |

---

## 3. Section Outline (final, in order)

1. Hero (H1 + tagline + badge row)
2. Status banner (block-quote callout)
3. Table of Contents
4. Quickstart
5. Architecture
6. Plantation Glossary
7. Feature Matrix
8. HTTP API Catalog
9. Database Schema Summary
10. Testing
11. Release Plan
12. Roadmap
13. Contributing
14. Documentation Index
15. License
16. Acknowledgments

---

## 4. Section Specifications

### 4.1 Hero

```markdown
# 🌴 Kulisawit

> A plantation of AI workers for your codebase. Open-source, local-first orchestration tool for solo developers to run, compare, and harvest multiple AI coding agents in parallel — built in Rust.

![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue) ![Rust 1.86+](https://img.shields.io/badge/rust-1.86%2B-orange) ![Status: pre-alpha](https://img.shields.io/badge/status-pre--alpha-red) ![Tests: 110 passing](https://img.shields.io/badge/tests-110%20passing-green)
```

Tagline copy is verbatim from PRD §0 one-liner.

Badges use shields.io static URL pattern (no API calls, no rate limits, no CI dependency). Test count is current snapshot (110, per session_state memory) — updated manually on README revisions, not live.

### 4.2 Status Banner

```markdown
> **⚠ Pre-alpha — Phase 3.1 of 6 complete.** HTTP+SSE backend works end-to-end. UI (Phase 3.2), sortir runner, and panen merge (Phase 3.3) are not implemented yet. See [Roadmap](#roadmap) for what ships today vs what's planned. **Not ready for daily use.**
```

Single block-quote, immediately under badges. Bold the warning prefix and the closing disclaimer.

### 4.3 Table of Contents

Markdown anchor list, manually maintained (GitHub auto-generates anchors from headings — match those slugs).

### 4.4 Quickstart

**Prerequisites** (bullet list):
- Rust 1.86+ (`rustup install 1.86`)
- Git
- A target git repo to point Kulisawit at (read+write access)

**Build & run** (single fenced bash block):
```bash
git clone https://github.com/BimaPangestu28/kulisawit
cd kulisawit
cargo build --release
./target/release/kulisawit serve \
  --db ./kulisawit.db \
  --repo /path/to/your/repo \
  --port 7700
```

**Verify** (separate fenced block):
```bash
# Create a project pointing at your repo
curl -X POST http://localhost:7700/api/projects \
  -H 'content-type: application/json' \
  -d '{"name":"demo","repo_path":"/path/to/your/repo"}'

# Create a task
curl -X POST http://localhost:7700/api/projects/<project-id>/tasks \
  -H 'content-type: application/json' \
  -d '{"title":"hello","description":"say hi"}'

# Stream attempt events (after dispatch)
curl -N http://localhost:7700/api/attempts/<attempt-id>/events
```

**Note** (callout): "No CLI subcommands for `tanam` / `panen` yet — only `serve`. Drive the server via HTTP API directly while Phase 3.2 (UI) is in flight."

### 4.5 Architecture

Intro paragraph: "Kulisawit is a Cargo workspace of 7 crates plus an embedded React UI (post-Phase 3.2). The CLI is the binary entry point; the orchestrator owns task lifecycle; the server exposes both over HTTP+SSE; lower crates wrap SQLite, git, and agent subprocesses."

Dependency diagram (ASCII, fenced as `text`):
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

Per-crate table (columns: Crate, Responsibility, Key deps, Status):

| Crate | Responsibility | Key deps | Status |
|---|---|---|---|
| `kulisawit-cli` | Binary entry, clap subcommand parsing, in-process server launcher | clap, tokio | ✅ `serve` only |
| `kulisawit-server` | axum HTTP + SSE, request validation, response shaping | axum, tower-http | ✅ Phase 3.1 |
| `kulisawit-orchestrator` | Dispatch logic, attempt lifecycle, broadcast event channels | tokio, broadcast | ✅ Phase 2 |
| `kulisawit-agent` | Kuli adapter trait + Claude Code reference impl (subprocess mgmt) | tokio process | 🚧 stub + adapter trait |
| `kulisawit-db` | sqlx pool, migrations, repository fns for projects/tasks/attempts | sqlx | ✅ Phase 1 |
| `kulisawit-git` | git2 wrappers for worktree create/list/remove, commit ops | git2 | ✅ Phase 1 |
| `kulisawit-core` | Shared domain types (Project, Task, Attempt, AttemptStatus, RunStatus, AttemptId) | serde, uuid, chrono | ✅ Phase 1 |

Status emoji legend: ✅ shipped | 🚧 partial / in progress | ⏳ planned.

### 4.6 Plantation Glossary

Direct port of PRD §0 table (13 rows). Columns: **Technical concept**, **Kulisawit term**, **Meaning**.

After the table, add naming-convention callout:

> **Naming convention.** Code identifiers use English (`task`, `attempt`, `worktree`); user-facing strings (CLI output, UI labels, future docs) use Kulisawit terms (`lahan`, `buah`, `petak`). This separation keeps internals readable for international contributors while preserving the brand in UX.

### 4.7 Feature Matrix

**Phase progress** table:

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

**Per-feature** table (PRD §3.1 F1–F12), columns: **ID**, **Feature**, **Status**:

| ID | Feature | Status |
|---|---|---|
| F1 | Kebun init from any git repo | ✅ |
| F2 | Kanban board (default columns) | ⏳ |
| F3 | Lahan CRUD (title, description, tags, files) | ✅ (no `tags` / `files` yet) |
| F4 | Single buah execution with worktree isolation | ✅ |
| F5 | Live SSE streaming of kuli output | ✅ |
| F6 | Tandan: N parallel buah on one lahan | ✅ |
| F7 | Side-by-side diff view across buah | ⏳ |
| F8 | Claude Code adapter (reference) | 🚧 stub |
| F9 | Sortir hooks (test/lint/build commands) | ⏳ |
| F10 | Panen: merge winning buah to main | ⏳ |
| F11 | SQLite persistence | ✅ |
| F12 | Single-binary install with embedded UI | ⏳ |

### 4.8 HTTP API Catalog

Intro: "Phase 3.1 ships a JSON HTTP API plus one SSE stream under `/api`. All endpoints are local-only — no auth, no rate limiting, no CORS preflight beyond the default. Endpoint shapes are stable within Phase 3.1 but may change before v0.1."

Three subsections (`### Projects (kebun)`, `### Tasks (lahan)`, `### Attempts (buah)`).

For each endpoint, present as `#### METHOD /path` heading + 1-paragraph description + fenced request example + fenced response shape (TypeScript-ish or JSON skeleton).

**Projects**:
- `POST /api/projects` — create project from local repo path. Validates repo is a git directory.
- `GET /api/projects` — list all projects.
- `GET /api/projects/:id` — fetch one.

**Tasks**:
- `POST /api/projects/:id/tasks` — create task on a project.
- `GET /api/projects/:id/tasks` — list tasks for a project.
- `POST /api/tasks/:id/dispatch` — plant N attempts (request body: `{count: number}`). Returns attempt IDs immediately; agents run detached via `tokio::spawn`.

**Attempts**:
- `GET /api/attempts/:id` — current state snapshot.
- `GET /api/attempts/:id/events` — SSE stream. Event kinds: `status` (RunStatus changes), `tool_call`, `stdout` chunks, `terminal_close` (stream end). Server replays events from DB on connect, then streams live from broadcast channel until terminal status, then closes.

Cross-reference for response shapes: "All response shapes mirror types in `crates/kulisawit-core/src/`. See `Project`, `Task`, `Attempt`, `AttemptEvent`, `AttemptStatus`, `RunStatus`."

### 4.9 Database Schema Summary

Intro paragraph: "SQLite via sqlx. Migrations live in `migrations/`. All primary keys are UUID v7 stored as `TEXT`. `kulisawit serve --db <path>` runs migrations on startup against the target file (creates if absent)."

Per-table summary (columns: Table, Purpose, Key columns, Foreign keys):

| Table | Purpose | Key columns | Foreign keys |
|---|---|---|---|
| `projects` | Tracked git repos (kebun) | `id`, `name`, `repo_path`, `created_at` | — |
| `tasks` | Work units (lahan) | `id`, `project_id`, `title`, `description`, `status`, `created_at` | `project_id → projects(id)` |
| `attempts` | Single agent runs (buah) | `id`, `task_id`, `worktree_path`, `status`, `started_at`, `finished_at` | `task_id → tasks(id)` |
| `attempt_events` | Per-attempt event log (replay source for SSE) | `id`, `attempt_id`, `event_kind`, `payload_json`, `seq`, `created_at` | `attempt_id → attempts(id)` |

Footer: "Indexes and exact column types are in `migrations/`. Check there for ground truth."

### 4.10 Testing

Intro: "**110 tests passing** across the workspace as of Phase 3.1 (tag `phase-3.1`)."

Breakdown table (columns: Crate, Tests, Categories present):

| Crate | Tests (approx) | Categories |
|---|---|---|
| `-core` | unit | type roundtrips, serde |
| `-db` | unit + integration | repo fns against tempfile SQLite |
| `-git` | integration | real `git2` repos in tempdirs |
| `-orchestrator` | unit + integration | dispatch lifecycle, broadcast |
| `-server` | integration + e2e | axum router, reqwest e2e on ephemeral port |
| `-agent` | unit | adapter trait contract |
| `-cli` | smoke | `serve` boot |

(Exact per-crate counts updated when README is written; aggregate is 110.)

**Test categories**:
- **Unit** — co-located in `src/` files via `#[cfg(test)] mod tests`.
- **Integration** — in `tests/` per crate; hit real SQLite tempfiles and real git2 repos in tempdirs.
- **End-to-end** — `crates/kulisawit-server/tests/e2e.rs`: spins server on ephemeral port via oneshot ready channel, drives full task lifecycle with `reqwest`, drains SSE to terminal close.

**Lints** subsection:
> Workspace-level lints **deny** `clippy::unwrap_used`, `clippy::expect_used`, and `clippy::panic` in production code. Tests opt in via `#[allow(clippy::expect_used)]` at function or module scope, with a `// Rationale: ...` comment when the rationale isn't obvious.

**How to run**:
```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

### 4.11 Release Plan

Bullet narrative:
- **Goal**: single static binary ~15MB via [`cargo-dist`](https://github.com/axodotdev/cargo-dist), embedded React UI via `rust-embed` (wired in Phase 5).
- **Targets**: `x86_64-unknown-linux-gnu`, `aarch64-apple-darwin`, `x86_64-pc-windows-msvc`. macOS Intel and Linux ARM64 considered for v0.1+.
- **Distribution**: GitHub Releases + `curl | sh` install script. Explicitly **not shipping** to npm, brew, or apt for v0.1.
- **Status**: not wired yet. `cargo build --release` produces a working ~12MB stripped server binary today (UI embed bumps it once Phase 3.2 + Phase 5 land).

### 4.12 Roadmap

Short narrative paragraph: "Kulisawit follows a 6-phase plan toward v0.1 launch. Each completed phase is tagged in git on the `main` branch."

Then re-anchor the **Phase Progress** table from §4.7 (just the phase rows, not features) so Roadmap is self-contained. Below the table:

- **Now (April 2026)**: Phase 3.2 — React UI minimum: kanban board, lahan detail panel, per-buah SSE log viewer.
- **Next**: Phase 3.3 (sortir runner + panen merge), then Phase 4 (additional kuli adapters: Codex, Aider, Gemini CLI).
- **v0.1 launch target**: Q3 2026 (calendar TBD; PRD §13 gives a 6-week MVP estimate from start of UI work).

**Post-v0.1 exploration** (PRD §3.2 / §3.3, marked clearly speculative):
- Timeline scrubbing per buah (replay mode)
- Fork-from-step: branch execution from mid-point with new instructions
- WASM plugin system for custom sortir runners and context providers
- MCP server integration as context source
- Auto-context: embed repo files, surface relevant ones per lahan

**Pointer**: "Detailed plans live in [`docs/superpowers/specs/`](docs/superpowers/specs/) — one design doc per phase."

### 4.13 Contributing

Intro: "Kulisawit is solo-built by [@BimaPangestu28](https://github.com/BimaPangestu28) but designed for outside contribution from day one."

**Three high-leverage contribution areas** (each = one paragraph):

1. **New kuli adapter** — implement the adapter trait in `kulisawit-agent`. Required: subprocess lifecycle (spawn, stream stdout/stderr, capture exit), structured event emission (tool calls, status transitions), error mapping into `AgentError`. Examples on the wishlist: Codex CLI, Aider, Gemini CLI, Cursor agent. Reference impl: Claude Code adapter (stub today, fleshing out alongside Phase 3.3).

2. **New sortir runner** — Phase 3.3 will define the sortir hook contract (test/lint/build commands run after each buah). Once that lands, ecosystem-specific runners (pytest, jest, golangci-lint, mypy) are direct contributions.

3. **Bug reports, naming, and docs** — file issues for anything broken or unclear. Plantation-metaphor naming suggestions for new concepts are explicitly welcome (better Indonesian terms for existing PRD entries especially).

**Dev workflow** subsection:
```bash
# fork on GitHub, then
git clone https://github.com/<your-username>/kulisawit
cd kulisawit
git remote add upstream https://github.com/BimaPangestu28/kulisawit.git
cd kulisawit
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
# branch, commit, push, open PR
```

**Code style**:
- `rustfmt` defaults (no overrides); `cargo fmt --all` before committing.
- Naming convention per PRD §0: English code identifiers, Kulisawit terms in user-facing strings only.
- No `unwrap()` / `expect()` / `panic!()` in non-test code (workspace lints enforce; tests opt in with `#[allow(...)]` + rationale comment).
- Commits: conventional style (`feat(scope): …`, `fix(scope): …`, `docs(scope): …`).

**Note**: "A formal `CONTRIBUTING.md` lands closer to v0.1. For now, ask in Issues if anything's unclear."

### 4.14 Documentation Index

Bullet list:
- [`docs/PRD.md`](docs/PRD.md) — full product requirements (608 lines, source of truth for vision and scope)
- [`docs/superpowers/specs/`](docs/superpowers/specs/) — per-phase design docs (one file per phase, dated)
- This README — entry point + reference

### 4.15 License

Standard Rust dual-license boilerplate:

```markdown
Licensed under either of

* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
```

(Implementation note: `LICENSE-APACHE` and `LICENSE-MIT` files do not exist yet; creating them is part of the implementation plan.)

### 4.16 Acknowledgments

Three short paragraphs:

1. **Prior art** — Vibe Kanban, Conductor, Crystal: the agent-orchestration tools that motivated this project (PRD §1.1). Kulisawit owes the kanban-as-orchestrator framing to all three.
2. **Stack** — Built on Tokio, Axum, sqlx, git2-rs, and rust-embed. The Rust ecosystem makes single-binary local-first tools genuinely easy.
3. **Plantation metaphor** — Named in honor of palm-plantation workers in Indonesia. The metaphor is a tribute, not a costume.

---

## 5. Length & Maintenance Estimates

- Estimated rendered length: **~700–900 lines** of markdown.
- Sections requiring manual updates on each phase completion: §4.1 badges (test count), §4.2 status banner (phase number), §4.7 feature matrix, §4.10 testing breakdown, §4.12 roadmap.
- Sections stable across phases: §4.5 architecture, §4.6 glossary, §4.13 contributing, §4.15 license, §4.16 acknowledgments.
- Acceptance check before merge: `cargo test --workspace` count matches badge; phase tags referenced exist (`git tag | grep phase-`); all linked anchors resolve.

---

## 6. Out of Scope (deferred to later iterations)

- Screenshots or GIFs (Phase 3.2 dependency).
- asciinema CLI demo recording (manual action; not blocking).
- Live shields.io badges with CI integration.
- Translated `README.id.md` (declined during brainstorming; revisit if Indonesian dev Twitter traction justifies).
- A full `CONTRIBUTING.md` (referenced as "TBD" from README; lands closer to v0.1).
- `LICENSE-MIT` and `LICENSE-APACHE` files — creating these is implementation-plan work, not design work.

---

## 7. Open Questions for Implementation Plan

1. Confirm test-count breakdown per crate (current: 110 aggregate; per-crate split needed for §4.10 table).
2. Confirm exact column names in `attempt_events` table (`payload_json` vs `payload` vs `data`) by checking migrations.
3. Confirm `POST /api/tasks/:id/dispatch` request body shape (`{count: number}` is design assumption; verify against `kulisawit-server` handler).
4. Decide whether to create `LICENSE-MIT` and `LICENSE-APACHE` files in the same PR or as a follow-up.
5. Fix `Cargo.toml` `workspace.package.repository` from `https://github.com/bimapangestu/kulisawit` to `https://github.com/BimaPangestu28/kulisawit` in the same PR (the current URL 404s).
