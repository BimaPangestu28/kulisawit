# PRD — Kulisawit

> **One-liner:** A plantation of AI workers for your codebase. Open-source, local-first orchestration tool for solo developers to run, compare, and harvest multiple AI coding agents in parallel — built in Rust.

---

## 0. Project Glossary

Kulisawit uses a plantation metaphor consistently throughout the codebase, UI, and docs. This is intentional branding. Contributors must preserve this vocabulary.

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

When writing code identifiers, **use English technical terms** (card, attempt, worktree) for clarity. When writing user-facing strings (CLI output, UI labels, docs), **use the Kulisawit vocabulary**. This separation keeps the code readable for international contributors while preserving the branding in UX.

---

## 1. Context & Vision

### 1.1 Problem
Solo developers using AI coding agents (Claude Code, Codex, Gemini CLI, Aider) face three recurring pains:

1. **Serial execution.** Agents run one task at a time. When an approach fails, user restarts from scratch — losing time and tokens.
2. **Blind trust.** User can't compare alternative implementations before committing. Quality is whatever the first attempt produced.
3. **Context fragmentation.** Task descriptions live in one place, relevant code in another, past agent runs lost to terminal history. Each new run starts cold.

Existing tools (Vibe Kanban, Conductor, Crystal) treat agents as single-shot executors behind a kanban UI. None treat agent execution itself as a first-class, explorable, branchable object. Most are also Electron/Node — heavy to install, slow to start, and opaque about what they're doing to your machine.

### 1.2 Vision
A kanban board where each lahan is not just a task, but an **execution playground**. Users plant N parallel buah per lahan, scrub through execution timelines, fork from any point, and sortir results automatically before panen. The board is the interface; the real product is multi-agent orchestration with git-like semantics.

Built in Rust as a single ~15MB binary that starts in under 100ms, idles at near-zero memory, and handles dozens of concurrent kuli without breaking a sweat.

### 1.3 Target User
**Primary:** Solo developers and indie hackers already using AI coding agents daily. Comfortable with terminal, git, self-hosting. Appreciate fast, native tooling.
**Secondary:** Rust developers interested in contributing to tools that showcase the language's strengths in systems programming.
**Tertiary:** Open-source contributors who want to extend Kulisawit with new agent adapters or verifiers.

Explicit non-goals: team collaboration, enterprise auth, cloud hosting, non-technical users.

### 1.4 Why Rust (architectural rationale)
This project has characteristics that make Rust a genuine fit, not just a preference:

- **Subprocess orchestration at scale.** Spawning and managing N concurrent agent processes with backpressure, cancellation, and structured event streaming. Tokio's async primitives are purpose-built for this.
- **Single binary distribution.** A core promise of the tool. Rust produces a true static binary with no runtime dependency. Node/Bun binaries bundle a JS runtime and feel heavy.
- **Long-running daemon.** Mandor runs for hours in the background. Rust's memory profile means users forget it's running. No GC pauses.
- **Embedded web assets.** `rust-embed` bakes the React UI into the binary. Zero config, zero "where are my static files".
- **Filesystem watching, git ops, IPC.** All areas where Rust's ecosystem (notify, git2, tokio) is mature and fast.

### 1.5 Success Metrics (first 6 months post-launch)
- 2,000+ GitHub stars
- 500+ weekly active installations (opt-in telemetry)
- 20+ external contributors with merged PRs
- 5+ community-contributed agent adapters
- Featured in: This Week in Rust, at least one "cool Rust project" roundup
- Viral moment in Indonesian dev Twitter (measurable: 500+ retweets on launch tweet)

---

## 2. Core Principles

1. **Local-first, zero cloud.** All state on user's machine. No accounts, no servers, no telemetry without opt-in.
2. **Agent-agnostic.** Every kuli integrates through the same adapter contract. No first-class favorites.
3. **Git-native.** Git is the substrate for isolation, history, and merging. Don't reinvent version control.
4. **Batteries included.** Default config works on first run. Advanced config is optional.
5. **Observable by default.** Every kuli action is logged, replayable, and forkable. No black boxes.
6. **Single binary, cold-start fast.** One install command, no Docker, no runtime. Mandor ready in under 100ms.
7. **Idiomatic Rust.** No `unwrap()` in code paths that handle user input or IO. Errors propagate via `thiserror` + `anyhow`. `unsafe` requires justification in review.
8. **Preserve the metaphor.** Kulisawit vocabulary stays consistent in user-facing surfaces. This is branding, not decoration.

---

## 3. Feature Scope

### 3.1 MVP (v0.1) — must ship to launch

- **F1.** Kebun initialization from any git repo
- **F2.** Kanban board with customizable columns (default: Backlog, Todo, Doing, Review, Done)
- **F3.** Lahan CRUD with title, description, tags, linked files
- **F4.** Single buah execution per lahan with git worktree isolation
- **F5.** Live streaming of kuli output (stdout/stderr/tool calls) to UI
- **F6.** Tandan: plant N buah on same lahan in parallel, each in separate petak
- **F7.** Side-by-side diff view across buah in a tandan
- **F8.** Adapter for Claude Code (reference kuli implementation)
- **F9.** Sortir hooks: run user-defined commands (test, lint, build) on buah completion
- **F10.** Panen: merge selected buah back to main branch via PR or direct merge
- **F11.** SQLite persistence of all kebun, lahan, and buah state
- **F12.** Single-binary install with embedded web UI

### 3.2 v0.2 — post-launch priorities
- Additional kuli adapters: Codex, Gemini CLI, Aider
- Timeline scrubbing per buah (replay mode)
- Fork-from-step: resume execution from mid-point with new instructions
- Auto-context: embed repo files, surface relevant ones per lahan
- Keyboard-first navigation (command palette, vim-like shortcuts)

### 3.3 v0.3+ — future exploration
- WASM plugin system for custom sortir runners and context providers
- Task decomposition: AI breaks high-level intent into sub-lahan
- DAG view for inter-lahan dependencies
- MCP server integration as context source
- Voice input and ambient progress notifications

### 3.4 Explicit non-goals
- Real-time multiplayer collaboration
- Mobile apps
- Hosted SaaS version
- Built-in chat/messaging
- Integrated billing or subscription logic

---

## 4. User Stories (MVP)

**US1.** As a solo dev, I run `kulisawit tanam` in my repo so the tool initializes a kebun and opens the board in my browser.

**US2.** As a solo dev, I create a lahan titled "add rate limiting to /api/login" with a description, so I can track what I want built.

**US3.** As a solo dev, I drag the lahan to "Doing" and select "Plant 3 buah with Claude Code" so three kuli start working in parallel, each in its own petak.

**US4.** As a solo dev, I watch live output from all three kuli in tabs within the lahan, so I can see progress without waiting.

**US5.** As a solo dev, after buah finish, I view a side-by-side diff of the three implementations, so I can compare approaches.

**US6.** As a solo dev, I see which buah passed sortir (test, lint), so I can filter to working solutions only.

**US7.** As a solo dev, I select the best buah and click "Panen", so the changes are merged to my main branch and the lahan moves to Done.

**US8.** As a contributor, I write a new kuli adapter by implementing the `KuliAdapter` trait in a single file, so I can add support for my preferred agent without touching core code.

---

## 5. Architecture

### 5.1 High-level components

```
┌─────────────────────────────────────────────────┐
│          Web UI (React + Tailwind)              │
│        embedded in binary via rust-embed        │
└────────────────┬────────────────────────────────┘
                 │ HTTP + SSE (localhost only)
┌────────────────▼────────────────────────────────┐
│            Mandor Daemon (Rust)                 │
│  ┌──────────┐  ┌───────────┐  ┌──────────────┐ │
│  │  Axum    │  │ Orchestr- │  │    Sortir    │ │
│  │  Server  │  │   ator    │  │    Runner    │ │
│  └──────────┘  └───────────┘  └──────────────┘ │
│  ┌──────────┐  ┌───────────┐  ┌──────────────┐ │
│  │  SQLx +  │  │  git2-rs  │  │     Kuli     │ │
│  │  SQLite  │  │   Petak   │  │   Adapter    │ │
│  │          │  │  Manager  │  │   Registry   │ │
│  └──────────┘  └───────────┘  └──────────────┘ │
│                                                 │
│         Runtime: Tokio (multi-threaded)         │
└─────────────────────────────────────────────────┘
                 │ tokio::process::Command
┌────────────────▼────────────────────────────────┐
│    Kuli processes (Claude Code, etc.)           │
│    each in isolated petak (git worktree)        │
└─────────────────────────────────────────────────┘
```

### 5.2 Stack

**Backend (Rust):**
- **Runtime:** Tokio (multi-threaded, full features)
- **Web framework:** Axum (first-class Tokio, tower middleware ecosystem)
- **Database:** SQLx with SQLite (compile-time checked queries, async)
- **Git operations:** git2-rs (libgit2 bindings) for queries; shell out to `git` CLI for worktree and merge operations
- **Process management:** `tokio::process::Command` with structured stdio piping
- **Filesystem watching:** `notify` crate for detecting kuli file edits in petak
- **Embedded assets:** `rust-embed` for baking the built React bundle into the binary
- **Serialization:** `serde` + `serde_json` throughout
- **Error handling:** `thiserror` for library errors, `anyhow` for application-layer
- **Logging:** `tracing` + `tracing-subscriber` with JSON output to `.kulisawit/logs/mandor.log`
- **CLI:** `clap` with derive macros
- **Config:** plain serde with `toml`

**Frontend:**
- React 18 + Vite + TypeScript
- Tailwind + shadcn/ui
- TanStack Query for server state, Zustand for client state
- SSE via native EventSource API

**Build & distribution:**
- `cargo build --release` with LTO and `strip` for small binaries
- `cargo-dist` for cross-platform release pipeline
- Targets: `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `aarch64-apple-darwin`, `x86_64-apple-darwin`, `x86_64-pc-windows-msvc`

### 5.3 Crate structure (workspace)

```
kulisawit/
  Cargo.toml                 # workspace root
  crates/
    kulisawit-cli/           # binary crate; clap CLI + mandor entrypoint
    kulisawit-core/          # orchestrator, domain types, adapter trait
    kulisawit-db/            # SQLx models, migrations, query functions
    kulisawit-git/           # petak management, panen/merge logic
    kulisawit-server/        # Axum routes, SSE handlers, embedded UI
    kulisawit-kuli/          # built-in kuli adapters (claude-code, etc.)
  ui/                        # React app, built into crates/kulisawit-server/assets/
  migrations/                # SQLx migration files
```

Rationale: splitting into crates keeps compile times manageable and makes the adapter boundary explicit. Contributors adding a new kuli touch `kulisawit-kuli` only.

### 5.4 Directory layout (user's repo)
```
<user-repo>/
  .kulisawit/                # per-kebun state (added to .gitignore)
    db.sqlite                # all metadata
    petak/                   # isolated worktrees
      buah-<uuid>/
    logs/
      mandor.log             # structured tracing output
      buah-<uuid>.jsonl      # per-buah event log
    peta-kebun.toml          # kebun-level config
```

### 5.5 Data model (SQLite via SQLx migrations)

Note: table names stay English for code clarity. User-facing surfaces translate these to Kulisawit terms.

```sql
-- migrations/0001_initial.sql

CREATE TABLE kebun (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  repo_path TEXT NOT NULL,
  created_at INTEGER NOT NULL
);

CREATE TABLE columns (
  id TEXT PRIMARY KEY,
  kebun_id TEXT NOT NULL REFERENCES kebun(id),
  name TEXT NOT NULL,
  position INTEGER NOT NULL
);

CREATE TABLE lahan (
  id TEXT PRIMARY KEY,
  kebun_id TEXT NOT NULL REFERENCES kebun(id),
  column_id TEXT NOT NULL REFERENCES columns(id),
  title TEXT NOT NULL,
  description TEXT,
  position INTEGER NOT NULL,
  tags TEXT,                  -- JSON array
  linked_files TEXT,          -- JSON array of repo-relative paths
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE TABLE buah (
  id TEXT PRIMARY KEY,
  lahan_id TEXT NOT NULL REFERENCES lahan(id),
  kuli_id TEXT NOT NULL,      -- which adapter
  prompt_variant TEXT,
  petak_path TEXT NOT NULL,   -- worktree path
  branch_name TEXT NOT NULL,
  status TEXT NOT NULL,       -- queued | running | completed | failed | cancelled
  started_at INTEGER,
  completed_at INTEGER,
  sortir_status TEXT,         -- pending | passed | failed | skipped
  sortir_output TEXT
);

CREATE TABLE events (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  buah_id TEXT NOT NULL REFERENCES buah(id),
  timestamp INTEGER NOT NULL,
  type TEXT NOT NULL,
  payload TEXT NOT NULL       -- JSON
);

CREATE INDEX idx_events_buah ON events(buah_id, timestamp);
CREATE INDEX idx_lahan_kebun ON lahan(kebun_id, column_id, position);
```

### 5.6 Kuli adapter contract (Rust trait)

Every kuli integrates through this trait. Core daemon never knows about agent-specific details.

```rust
use async_trait::async_trait;
use futures::stream::BoxStream;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[async_trait]
pub trait KuliAdapter: Send + Sync {
    /// Stable identifier, e.g. "claude-code"
    fn id(&self) -> &str;

    /// Human-readable name for UI
    fn display_name(&self) -> &str;

    /// Adapter version
    fn version(&self) -> &str;

    /// Verify the adapter can run (binary installed, auth ok, etc.)
    async fn check(&self) -> Result<CheckResult, KuliError>;

    /// Run the kuli on a prepared petak. Returns a stream of events.
    async fn run(
        &self,
        ctx: RunContext,
    ) -> Result<BoxStream<'static, KuliEvent>, KuliError>;

    /// Request cancellation of a running buah
    async fn cancel(&self, run_id: &str) -> Result<(), KuliError>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunContext {
    pub run_id: String,
    pub petak_path: PathBuf,
    pub prompt: String,
    pub prompt_variant: Option<String>,
    pub env: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum KuliEvent {
    Stdout { text: String },
    Stderr { text: String },
    ToolCall { name: String, input: serde_json::Value },
    ToolResult { name: String, output: serde_json::Value },
    FileEdit { path: String, diff: Option<String> },
    Status { status: RunStatus, detail: Option<String> },
}
```

Reference implementation (`ClaudeCodeKuli`) wraps `tokio::process::Command`, pipes the agent's stdout through a line-based parser, and uses `notify` to detect file edits in the petak.

### 5.7 Orchestration flow (tandan execution)

When user clicks "Tanam N buah" on a lahan:

1. Orchestrator inserts N `buah` rows with status `queued`.
2. For each buah, spawn a Tokio task that:
   - Creates petak at `.kulisawit/petak/buah-<uuid>` via `git worktree add` (shell out — libgit2 worktree API is incomplete).
   - Creates branch `kulisawit/<lahan-id>/<buah-short-id>` based on target branch HEAD.
   - Composes the prompt from lahan title, description, linked files, and variant strategy.
   - Calls the adapter's `run()` method and consumes the event stream.
   - For each event: persist to `events` table and broadcast over `tokio::sync::broadcast` channel consumed by SSE handlers.
3. On adapter completion, sortir runner executes configured commands in the petak, captures output, updates `buah.sortir_status`.
4. All petak file changes are committed to the buah branch with message `kulisawit: buah <short-id> for <lahan-title>`.

Concurrency model: each buah runs in its own Tokio task. A semaphore limits total concurrent buah across the mandor (default 8, configurable) to prevent overwhelming the machine.

### 5.8 Panen (merge) flow
1. User selects winning buah in UI.
2. Orchestrator checks out target branch, fast-forward merges buah branch if possible, else creates a merge commit.
3. If `gh` CLI is available and user opted in, opens PR instead of merging directly.
4. Petak for unselected buah are retained per config (default 7 days) then cleaned up (`bersihkan`) on mandor start.
5. Lahan moves to configured "Done" column, all buah for the lahan are archived (soft-delete via status).

---

## 6. UI/UX Specification

### 6.1 Layout
Single-page React app served from the binary, three main views:

- **Kebun view** (default): kanban columns, lahan draggable between columns
- **Lahan detail view**: opens as right-side panel or full overlay; shows description, linked files, buah list, and active buah stream
- **Settings view**: kebun config, kuli config, sortir commands

### 6.2 Lahan interactions
- Click lahan → opens detail panel
- Drag lahan across columns → updates `column_id`
- Right-click lahan → context menu: Tanam buah, Duplicate, Archive, Delete
- Detail panel shows buah as horizontal tabs; each tab has live output, file changes, sortir status

### 6.3 Tandan comparison view
When lahan has 2+ buah:
- Toggle "Compare mode" in lahan detail
- Grid layout (2 or 3 columns) showing diff per buah side-by-side
- Each buah has: status badge, sortir result, token/time cost, "Panen this" button
- File-level tabs so user can jump between changed files across all buah simultaneously

### 6.4 Live streaming UX
- SSE stream of `KuliEvent` drives the UI in real time
- Auto-scroll with "pause on hover" behavior
- Tool calls rendered as collapsible blocks with input/output
- File edits shown as inline mini-diffs
- Status indicator (spinner → checkmark/cross) at top of stream

### 6.5 Visual design direction
- Dark mode default, light mode available
- Monospace for code/logs, system sans for UI chrome
- Density: comfortable-to-dense, not airy — power-user tool
- Color palette:
  - Primary: `#D97706` (palm oil amber)
  - Accent: `#15803D` (leaf green)
  - Dark: `#1C1917` (soil black)
- Inspiration: Linear's polish, GitHub's information density, terminal aesthetic for execution views

### 6.6 Microcopy guidelines
Use Kulisawit vocabulary in UI strings where it flows naturally. Never force the metaphor — if a term feels awkward, fall back to plain English.

Good examples:
- Button: "Tanam 3 buah" (clearer intent than "Run 3 attempts" in context)
- Empty state: "Kebun masih kosong. Bikin lahan pertama kamu."
- Loading state: "Mandor sedang menyiapkan petak..."
- Success toast: "Panen berhasil. Perubahan sudah di-merge ke main."

Bad examples:
- Error: "Panen gagal: konflik bersihkan tandan" → incomprehensible
  - Better: "Merge failed due to conflicts. Resolve manually and retry."

Rule of thumb: **flavor, not translation**. Keep technical clarity over metaphor purity.

---

## 7. Configuration

### 7.1 Kebun config (`.kulisawit/peta-kebun.toml`)
```toml
version = 1
default_kuli = "claude-code"
default_tandan_size = 1
petak_retention_days = 7
max_concurrent_buah = 8

[[sortir.commands]]
name = "check"
cmd = "cargo check"

[[sortir.commands]]
name = "test"
cmd = "cargo test"

[[sortir.commands]]
name = "clippy"
cmd = "cargo clippy -- -D warnings"

[sortir]
require_all_pass = false

[kuli.claude-code]
enabled = true
extra_args = []
```

### 7.2 CLI commands (MVP)

Hybrid naming: top-level commands in English for international accessibility, domain-specific actions use Kulisawit vocabulary.

```bash
kulisawit init              # initialize kebun in current repo
kulisawit start             # start mandor + open UI in browser
kulisawit stop              # gracefully stop mandor
kulisawit status            # print active buah, kebun stats
kulisawit tanam <lahan-id>  # plant buah on a lahan from CLI (without UI)
kulisawit panen <buah-id>   # merge a buah from CLI
kulisawit bersihkan         # GC old petak
kulisawit dokter            # run adapter check() for all configured kuli
kulisawit telemetry <on|off>  # opt in/out of anonymous usage data
```

Short alias: binary also symlinks as `ksw` for power users (`ksw start`, `ksw status`).

---

## 8. Security & Privacy

- **No network calls by default** except those the kuli itself makes.
- **Opt-in telemetry only**, disabled until user runs `kulisawit telemetry on`. Collects anonymous install ID, version, adapter types used, buah counts. Never collects prompts, code, or file contents.
- **Kuli sandboxing is out of scope for MVP.** Users run kuli that already have access to their repo; Kulisawit does not add a security boundary. Document this explicitly in README.
- **API surface binds to `127.0.0.1` only.** No remote access. No auth required because no network exposure.
- **`unsafe` policy:** no `unsafe` blocks in MVP. If ever needed, must be accompanied by a `// SAFETY:` comment and reviewed by a second contributor.

---

## 9. Testing Strategy

- **Unit tests:** core orchestrator state machine, adapter trait object safety, SQLx query correctness (compile-time verified), config parsing. Use `cargo test`.
- **Integration tests:** full run of a `MockKuli` through the orchestrator, verifying event stream and DB state. Tests live in `crates/kulisawit-core/tests/`.
- **DB tests:** SQLx test macros with in-memory SQLite.
- **E2E smoke test:** CI boots mandor, creates kebun, plants mock kuli on a fixture repo, verifies panen works. Uses Playwright against the running mandor.
- **Manual QA checklist** before each release: init on fresh repo, tandan of 3, fail one sortir, panen winner, bersihkan cleanup, mandor restart mid-run.
- **`cargo clippy -- -D warnings`** and **`cargo fmt --check`** enforced in CI.

---

## 10. Release & Distribution

- **v0.1 MVP release:** GitHub Releases with prebuilt binaries via `cargo-dist` for macOS (arm64, x64), Linux (x64, arm64), Windows (x64).
- **Install paths:**
  - `cargo install kulisawit` — crates.io
  - Homebrew tap — macOS/Linux
  - Shell installer: `curl -sSf https://kulisawit.dev/install.sh | sh`
  - `cargo binstall kulisawit` for Rust users
- **Versioning:** SemVer. Pre-1.0 allows breaking changes, documented in CHANGELOG.
- **Launch channels:**
  - Hacker News "Show HN"
  - r/rust, r/programming
  - This Week in Rust submission
  - Twitter/X (both English and Indonesian dev community)
  - dev.to writeup
  - Product Hunt
- **Demo video:** 60-90 seconds showing a tandan of 3 buah racing, diff comparison, sortir passing, panen. Essential.
- **Indonesian audience outreach:** launch tweet in Bahasa, thread di Twitter Indonesia, post di komunitas dev Indo (Telegram, Discord groups). The name is a deliberate hook — lean into it.
- **Rust-specific outreach:** blog post on "building a concurrent process orchestrator in Rust" as personal branding content.

---

## 11. Open Questions (resolve before v0.1)

1. **Tauri or embedded web UI?** Tauri gives native feel; embedded UI is simpler. *Leaning: embedded UI for MVP, Tauri as optional packaging later.*
2. **Prompt composition:** how much context to auto-inject? *Leaning: minimal injection in MVP (lahan title + description + linked files); smarter context in v0.2.*
3. **Sortir scope:** per-kebun only or per-lahan override? *Leaning: per-kebun for MVP, per-lahan override in v0.2.*
4. **Merge conflicts:** auto-resolution, manual in UI, or shell-out? *Leaning: surface conflicts in UI, user resolves via their editor of choice.*
5. **libgit2 vs git CLI shell-outs:** *Leaning: libgit2 for queries (status, diff, log), shell out for worktree management and merge operations.*
6. **Kulisawit vocabulary in logs?** Do `tracing` spans use Indonesian or English terms? *Leaning: English in logs (debugging surface), Indonesian in user-facing UI and CLI output.*

---

## 12. Milestones

| Week | Deliverable |
|------|-------------|
| 1 | Cargo workspace scaffold, SQLx migrations, Axum server skeleton, embedded UI harness with placeholder React app |
| 2 | Lahan CRUD API + UI, petak manager, `MockKuli` for dev without real agents |
| 3 | Claude Code kuli adapter, SSE event streaming, single-buah execution end-to-end |
| 4 | Tandan (parallel buah), side-by-side diff view, buah selection and panen |
| 5 | Sortir runner, config system, cancellation handling, graceful shutdown |
| 6 | `cargo-dist` release pipeline, install docs, demo video, launch |

---

## 13. Implementation Guide for Claude Code

This section contains execution-specific guidance. Read before starting any task.

### 13.1 Build order

1. **Scaffold the workspace first.** Create all six crates with empty `lib.rs` and workspace `Cargo.toml`. Add dependencies listed in section 5.2. Do not implement features yet — just make `cargo build` succeed.
2. **SQLx migrations next.** Implement section 5.5 as migration files under `migrations/`. Set up `sqlx::migrate!()` in `kulisawit-db`. Verify with a scratch test that an in-memory SQLite can run all migrations.
3. **Adapter trait and MockKuli.** Define `KuliAdapter` trait in `kulisawit-core` per section 5.6. Implement a `MockKuli` in `kulisawit-kuli` that emits scripted events for testing. This unblocks orchestrator work without real agents.
4. **Petak manager.** In `kulisawit-git`, implement functions to create/delete worktrees, create branches, commit changes. Use `std::process::Command` for worktree ops, `git2-rs` for queries.
5. **Orchestrator core.** In `kulisawit-core`, implement the flow from section 5.7. Use `tokio::sync::Semaphore` for bounded concurrency, `tokio::sync::broadcast` for event fanout.
6. **Axum server with SSE.** In `kulisawit-server`, expose REST endpoints for lahan CRUD and an SSE endpoint streaming `KuliEvent`s per buah.
7. **React UI.** Build kanban board, lahan detail, tandan comparison view. TanStack Query for API, EventSource for SSE.
8. **Sortir runner.** In `kulisawit-core`, after a buah completes, run configured shell commands in the petak and capture output.
9. **Panen flow.** In `kulisawit-git`, implement merge logic per section 5.8.
10. **CLI entry point.** In `kulisawit-cli`, wire up `clap` commands from section 7.2 to core functions.
11. **Release pipeline.** Set up `cargo-dist` per section 10.
12. **Demo video and launch.**

### 13.2 Rust idiom quick-reference

- No `unwrap()` or `expect()` in production code paths. Exceptions: tests, and `main()` startup where panic is acceptable.
- Prefer `&str` in function signatures over `String` unless ownership is needed.
- Use `tracing::instrument` on orchestrator functions for free structured logs.
- Newtype wrappers (`struct BuahId(String)`, `struct LahanId(String)`) for domain IDs to prevent mixups at the type level.
- `Arc<dyn KuliAdapter>` for dynamic dispatch at the registry layer; don't force generics up the stack.
- SQLx queries use `query!` and `query_as!` macros for compile-time checking. Run `cargo sqlx prepare` and commit `.sqlx/` metadata so CI can build without a live DB.
- Errors: use `thiserror`-derived enums at crate boundaries. Use `anyhow::Result` only in `kulisawit-cli` (the binary crate).
- Return `Result<T, E>` wherever IO, parsing, or subprocess spawning happens. Never swallow errors.

### 13.3 Vocabulary enforcement

When writing user-facing strings, consult section 0 (glossary). When writing code identifiers, use English technical terms. If unsure which side a string falls on, default to English and leave a TODO for later localization review.

Examples:
- `pub struct Buah { ... }` — code uses Indonesian name because it's a core domain concept; this is intentional branding reflected in the API.
- `pub fn create_buah(...)` — function uses domain term.
- `tracing::info!("buah {} completed", buah_id)` — log uses domain term for consistency with the rest of the codebase.
- UI button text `"Tanam 3 buah"` — user-facing, domain term.
- Error message `"Failed to create worktree: permission denied"` — technical failure, plain English for debuggability.

### 13.4 Things NOT to do

- Do not add cloud sync, user accounts, or any network feature requiring authentication.
- Do not introduce Electron, Tauri, or any framework that ships a runtime. The single-binary story is core.
- Do not pull in heavyweight ORMs (diesel, sea-orm). SQLx is the choice.
- Do not mix `tokio` and `async-std`. Stay on Tokio.
- Do not introduce a plugin system in MVP. v0.3+ only.
- Do not translate log messages, error types, or public API names to Indonesian. Code stays in English. Only user-facing surfaces get the Kulisawit vocabulary.
- Do not make the metaphor cringe. If a Kulisawit term obscures meaning in a UI string, prefer clarity. "Flavor, not translation."

### 13.5 First command to run

```bash
cargo new --lib kulisawit-core
# ...followed by full workspace scaffold per section 5.3
```

Claude Code: start by creating the workspace `Cargo.toml` at the repo root with the `[workspace]` table listing all six member crates, then `cargo new --lib` each crate. Verify `cargo build` succeeds before writing any real code.
