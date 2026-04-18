# Kulisawit v0.1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship Kulisawit v0.1 — a single-binary Rust tool that lets a solo dev plant N parallel AI coding agents per task in isolated git worktrees, compare their diffs, run sortir (tests/lint), and panen (merge) the winning attempt — per PRD.

**Architecture:** Cargo workspace of 6 crates (`kulisawit-cli` binary + 5 libraries) producing one binary. Axum+Tokio daemon persists state in SQLite (SQLx), isolates each agent run in a git worktree under `.kulisawit/petak/`, streams structured `KuliEvent`s to an embedded React UI via SSE. Agents integrate through one `KuliAdapter` trait.

**Tech Stack:** Rust 2021 • Tokio (multi-threaded) • Axum • SQLx + SQLite • git2-rs (queries) + `git` CLI (worktree/merge) • rust-embed • clap • thiserror + anyhow • tracing • serde. Frontend: React 18 + Vite + TypeScript + Tailwind + shadcn/ui + TanStack Query + Zustand + native EventSource.

**Source spec:** `/mnt/c/Users/bimap/Downloads/PRD.md` — copied to `docs/PRD.md` in Task 1.1 for stable reference.

---

## Phase Roadmap

| Phase | Scope | Detail level in this doc | Ships |
|-------|-------|--------------------------|-------|
| 1 | Foundation: workspace, domain types, DB, adapter trait, MockKuli, petak manager | **Full TDD** | Libraries with 100% green tests |
| 2 | Orchestrator core: tandan execution, semaphore, event fanout, end-to-end with MockKuli | Milestone outline | `mandor` binary runs MockKuli buah from CLI |
| 3 | Server + UI: Axum REST + SSE, embedded React kanban/lahan/tandan views | Milestone outline | Browser-usable daemon |
| 4 | Real agent + parallel buah: Claude Code adapter, tandan diff view | Milestone outline | End-to-end real flow |
| 5 | Sortir + Panen + CLI + config: production MVP | Milestone outline | Feature-complete v0.1 |
| 6 | cargo-dist release pipeline, demo video, launch | Milestone outline | Public v0.1 on GitHub Releases |

Phases 2–6 list deliverables, file touchpoints, and the acceptance bar, but **not** bite-sized tasks. Before executing any phase, write a dedicated detailed plan for it using the writing-plans skill.

---

## Conventions (apply throughout)

- **Commits:** after every passing test suite. Commit messages follow Conventional Commits (`feat:`, `fix:`, `test:`, `chore:`, `refactor:`, `docs:`).
- **No `unwrap()` / `expect()`** in production code paths. Exception: `main()` startup, tests, and documented `SAFETY:` blocks (none in MVP).
- **Errors:** `thiserror`-derived enums at library crate boundaries. `anyhow::Result` only in `kulisawit-cli` (the binary).
- **Domain IDs:** newtype wrappers (`BuahId(String)`, `LahanId(String)`, `KebunId(String)`, `ColumnId(String)`). All derive `Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize` and wrap UUID v7 strings.
- **SQLx macros:** `query!` / `query_as!` for compile-time checking. Commit `.sqlx/` metadata so CI builds without a live DB. Run `cargo sqlx prepare --workspace` after query changes.
- **Logging:** `tracing` everywhere. `tracing::instrument(skip(self))` on orchestrator functions. Log messages in English.
- **User-facing strings** (CLI stdout, UI labels, future microcopy): use Kulisawit vocabulary (kebun, lahan, tandan, buah, petak, mandor, tanam, sortir, panen, bersihkan). Error messages stay plain English for debuggability.
- **Rust edition:** 2021. MSRV: 1.82.
- **Tests:** one `#[cfg(test)] mod tests` per file for unit tests; integration tests under `crates/<name>/tests/`.
- **Async trait:** use `async_trait` crate (don't rely on stable async-fn-in-trait yet — the ecosystem `dyn` support is still rough).

---

## File Structure (end of Phase 1)

```
kulisawit/
├── Cargo.toml                          # workspace root
├── Cargo.lock
├── .gitignore
├── rust-toolchain.toml                 # pin to stable 1.82+
├── .sqlx/                              # generated offline-query metadata (committed)
├── docs/
│   └── PRD.md                          # copy of source PRD
├── migrations/
│   └── 0001_initial.sql                # all tables per PRD §5.5
├── ui/                                 # placeholder; populated in Phase 3
│   └── .gitkeep
└── crates/
    ├── kulisawit-core/                 # domain types, traits, orchestrator (orchestrator in Phase 2)
    │   ├── Cargo.toml
    │   ├── src/
    │   │   ├── lib.rs
    │   │   ├── ids.rs                  # newtype ID wrappers
    │   │   ├── status.rs               # RunStatus, SortirStatus, BuahStatus
    │   │   ├── error.rs                # CoreError
    │   │   ├── adapter/
    │   │   │   ├── mod.rs              # KuliAdapter trait, RunContext
    │   │   │   └── event.rs            # KuliEvent, CheckResult
    │   │   └── prelude.rs              # re-exports
    │   └── tests/
    │       └── adapter_object_safety.rs
    ├── kulisawit-db/                   # pool + repositories
    │   ├── Cargo.toml
    │   ├── src/
    │   │   ├── lib.rs
    │   │   ├── pool.rs                 # Pool<Sqlite> factory + migration runner
    │   │   ├── error.rs                # DbError
    │   │   ├── kebun.rs                # Kebun repository
    │   │   ├── columns.rs              # Column repository
    │   │   ├── lahan.rs                # Lahan repository
    │   │   ├── buah.rs                 # Buah repository
    │   │   └── events.rs               # Event log repository
    │   └── tests/
    │       ├── migrations.rs
    │       └── repositories.rs
    ├── kulisawit-git/                  # petak (worktree) manager
    │   ├── Cargo.toml
    │   ├── src/
    │   │   ├── lib.rs
    │   │   ├── error.rs                # GitError
    │   │   ├── petak.rs                # create/remove/list petak (shell-out)
    │   │   ├── branch.rs               # create branch, commit in petak
    │   │   └── query.rs                # git2-rs: status, diff, log
    │   └── tests/
    │       └── petak_roundtrip.rs      # operates on a tempdir git repo
    ├── kulisawit-kuli/                 # built-in adapters
    │   ├── Cargo.toml
    │   └── src/
    │       ├── lib.rs
    │       └── mock.rs                 # MockKuli (Phase 1); claude_code.rs in Phase 4
    ├── kulisawit-server/               # Axum (Phase 3)
    │   ├── Cargo.toml
    │   └── src/lib.rs                  # empty stub in Phase 1
    └── kulisawit-cli/                  # binary crate (Phase 2+)
        ├── Cargo.toml
        └── src/main.rs                 # prints version in Phase 1
```

---

## Phase 1: Foundation

**Goal:** A cargo workspace where every library has green tests covering its own surface, and all future phases have primitives to build on: domain types, DB repositories, the adapter contract with a working mock, and git worktree management.

**Exit criteria:**
- `cargo build --workspace` succeeds clean
- `cargo test --workspace` passes
- `cargo clippy --workspace -- -D warnings` passes
- `cargo fmt --check` passes
- `cargo sqlx prepare --workspace --check` passes
- `kulisawit-cli` binary prints `kulisawit 0.1.0-dev`
- MockKuli can be driven end-to-end: create petak → run adapter → persist events → delete petak, covered by one integration test in `kulisawit-kuli/tests/`

---

### Task 1.1: Repo skeleton + PRD copy + toolchain pin

**Files:**
- Create: `rust-toolchain.toml`
- Create: `.gitignore`
- Create: `docs/PRD.md` (copy of `/mnt/c/Users/bimap/Downloads/PRD.md`)
- Create: `ui/.gitkeep`
- Create: `migrations/.gitkeep`

- [ ] **Step 1: Pin Rust toolchain**

Create `rust-toolchain.toml`:

```toml
[toolchain]
channel = "1.82"
components = ["rustfmt", "clippy"]
```

- [ ] **Step 2: Write `.gitignore`**

Create `.gitignore`:

```gitignore
/target
Cargo.lock.bak
.kulisawit/
node_modules/
ui/dist/
ui/node_modules/
.DS_Store
*.swp
.idea/
.vscode/*
!.vscode/settings.json
```

Note: Do **not** ignore `Cargo.lock` — we're shipping a binary. Do **not** ignore `.sqlx/` — it must be committed for offline compile.

- [ ] **Step 3: Copy the PRD into the repo**

```bash
mkdir -p docs && cp /mnt/c/Users/bimap/Downloads/PRD.md docs/PRD.md
```

Verify:

```bash
head -1 docs/PRD.md
```

Expected: `# PRD — Kulisawit`

- [ ] **Step 4: Placeholder folders**

```bash
mkdir -p ui migrations && touch ui/.gitkeep migrations/.gitkeep
```

- [ ] **Step 5: Commit**

```bash
git add rust-toolchain.toml .gitignore docs/PRD.md ui/.gitkeep migrations/.gitkeep
git commit -m "chore: pin toolchain, vendor PRD, add gitignore and placeholder dirs"
```

---

### Task 1.2: Cargo workspace + empty crates

**Files:**
- Create: `Cargo.toml` (workspace root)
- Create: `crates/kulisawit-core/{Cargo.toml,src/lib.rs}`
- Create: `crates/kulisawit-db/{Cargo.toml,src/lib.rs}`
- Create: `crates/kulisawit-git/{Cargo.toml,src/lib.rs}`
- Create: `crates/kulisawit-kuli/{Cargo.toml,src/lib.rs}`
- Create: `crates/kulisawit-server/{Cargo.toml,src/lib.rs}`
- Create: `crates/kulisawit-cli/{Cargo.toml,src/main.rs}`

- [ ] **Step 1: Write root `Cargo.toml`**

```toml
[workspace]
resolver = "2"
members = [
    "crates/kulisawit-cli",
    "crates/kulisawit-core",
    "crates/kulisawit-db",
    "crates/kulisawit-git",
    "crates/kulisawit-server",
    "crates/kulisawit-kuli",
]

[workspace.package]
version = "0.1.0-dev"
edition = "2021"
rust-version = "1.82"
license = "MIT OR Apache-2.0"
repository = "https://github.com/bimapangestu/kulisawit"
authors = ["Bima Pangestu"]

[workspace.dependencies]
# Async runtime
tokio = { version = "1.40", features = ["full"] }
tokio-util = { version = "0.7", features = ["io"] }
futures = "0.3"
async-trait = "0.1"

# Web
axum = { version = "0.7", features = ["macros"] }
tower = "0.5"
tower-http = { version = "0.6", features = ["cors", "trace"] }

# DB
sqlx = { version = "0.8", default-features = false, features = ["runtime-tokio-rustls", "sqlite", "macros", "migrate", "chrono", "json"] }

# Git
git2 = { version = "0.19", default-features = false, features = ["vendored-libgit2"] }

# Serde
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"

# Errors / logging / CLI
thiserror = "1"
anyhow = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
clap = { version = "4.5", features = ["derive", "env"] }

# Misc
uuid = { version = "1.10", features = ["v7", "serde"] }
chrono = { version = "0.4", default-features = false, features = ["clock", "serde"] }
notify = "6"
rust-embed = { version = "8", features = ["mime-guess"] }

# Test utilities
tempfile = "3"
assert_matches = "1.5"

# Internal crates
kulisawit-core   = { path = "crates/kulisawit-core",   version = "0.1.0-dev" }
kulisawit-db     = { path = "crates/kulisawit-db",     version = "0.1.0-dev" }
kulisawit-git    = { path = "crates/kulisawit-git",    version = "0.1.0-dev" }
kulisawit-server = { path = "crates/kulisawit-server", version = "0.1.0-dev" }
kulisawit-kuli   = { path = "crates/kulisawit-kuli",   version = "0.1.0-dev" }

[profile.release]
opt-level = 3
lto = "fat"
codegen-units = 1
strip = "symbols"
panic = "abort"

[profile.dev]
debug = 1          # line tables only; faster links
split-debuginfo = "unpacked"
```

- [ ] **Step 2: Create the 5 library crate manifests**

Each library crate has a minimal `Cargo.toml` at this stage. Dependencies are added in later tasks.

`crates/kulisawit-core/Cargo.toml`:

```toml
[package]
name = "kulisawit-core"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
authors.workspace = true
description = "Kulisawit domain types, adapter trait, orchestrator"

[lib]
```

Repeat identically for `kulisawit-db`, `kulisawit-git`, `kulisawit-kuli`, `kulisawit-server` — change only the `name` and `description` fields. Descriptions:

- `kulisawit-db`: "SQLite repositories for Kulisawit"
- `kulisawit-git`: "Git worktree (petak) management for Kulisawit"
- `kulisawit-kuli`: "Built-in Kuli adapters for Kulisawit"
- `kulisawit-server`: "Axum HTTP + SSE server for Kulisawit"

- [ ] **Step 3: Create the binary crate manifest**

`crates/kulisawit-cli/Cargo.toml`:

```toml
[package]
name = "kulisawit-cli"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
authors.workspace = true
description = "Kulisawit CLI"

[[bin]]
name = "kulisawit"
path = "src/main.rs"

[dependencies]
anyhow.workspace = true
```

- [ ] **Step 4: Empty `lib.rs` for each library**

Each of `crates/kulisawit-{core,db,git,kuli,server}/src/lib.rs`:

```rust
//! {crate-description}
//!
//! See the workspace root `README.md` and `docs/PRD.md` for the product brief.

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![warn(missing_debug_implementations, rust_2018_idioms)]
```

Replace `{crate-description}` per crate.

- [ ] **Step 5: Minimal `kulisawit-cli/src/main.rs`**

```rust
fn main() -> anyhow::Result<()> {
    println!("kulisawit {}", env!("CARGO_PKG_VERSION"));
    Ok(())
}
```

- [ ] **Step 6: Verify build**

```bash
cargo build --workspace
```

Expected: clean build, no warnings.

```bash
cargo run -p kulisawit-cli
```

Expected: `kulisawit 0.1.0-dev`

- [ ] **Step 7: Verify lint + fmt**

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
```

Expected: both pass.

- [ ] **Step 8: Commit**

```bash
git add Cargo.toml Cargo.lock crates
git commit -m "chore: scaffold cargo workspace with six crates"
```

---

### Task 1.3: Domain ID newtypes

**Files:**
- Create: `crates/kulisawit-core/src/ids.rs`
- Modify: `crates/kulisawit-core/src/lib.rs`
- Modify: `crates/kulisawit-core/Cargo.toml`

- [ ] **Step 1: Add dependencies for `kulisawit-core`**

`crates/kulisawit-core/Cargo.toml`:

```toml
[dependencies]
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true
uuid.workspace = true
async-trait.workspace = true
futures.workspace = true
tokio = { workspace = true, features = ["sync", "rt"] }
tracing.workspace = true
chrono.workspace = true

[dev-dependencies]
serde_json.workspace = true
```

Verify still builds: `cargo build -p kulisawit-core`.

- [ ] **Step 2: Write the failing test for ID behavior**

Create `crates/kulisawit-core/src/ids.rs` with test skeleton only:

```rust
//! Strongly-typed domain identifiers. All IDs wrap UUID v7 strings.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn different_id_types_do_not_compile_interchangeably() {
        let kebun = KebunId::new();
        let lahan = LahanId::new();
        // Compile-time check via accessor signatures:
        assert_eq!(kebun.as_str().len(), 36);
        assert_eq!(lahan.as_str().len(), 36);
    }

    #[test]
    fn ids_roundtrip_through_json() {
        let id = BuahId::new();
        let json = serde_json::to_string(&id).expect("ser");
        let back: BuahId = serde_json::from_str(&json).expect("de");
        assert_eq!(id, back);
    }

    #[test]
    fn ids_are_unique_across_calls() {
        let a = LahanId::new();
        let b = LahanId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn parse_from_str_accepts_any_non_empty_string() {
        let id = ColumnId::from_string("col-1".to_owned());
        assert_eq!(id.as_str(), "col-1");
    }
}
```

Register the module — `crates/kulisawit-core/src/lib.rs`:

```rust
//! Kulisawit domain types, adapter trait, orchestrator

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![warn(missing_debug_implementations, rust_2018_idioms)]

pub mod ids;

pub use ids::{BuahId, ColumnId, KebunId, LahanId};
```

- [ ] **Step 3: Run and see it fail**

```bash
cargo test -p kulisawit-core ids
```

Expected: FAIL — `cannot find type KebunId`, etc.

- [ ] **Step 4: Implement via a macro**

Append to `crates/kulisawit-core/src/ids.rs` (above the `#[cfg(test)]` block):

```rust
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

macro_rules! define_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// Generate a fresh identifier backed by a UUID v7.
            pub fn new() -> Self {
                Self(Uuid::now_v7().to_string())
            }

            /// Wrap an existing string (e.g. loaded from storage).
            pub fn from_string(s: String) -> Self {
                Self(s)
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }

            pub fn into_string(self) -> String {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl From<String> for $name {
            fn from(s: String) -> Self {
                Self::from_string(s)
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }
    };
}

define_id!(
    /// Identifier for a `kebun` (tracked repository).
    KebunId
);
define_id!(
    /// Identifier for a column on the kanban board.
    ColumnId
);
define_id!(
    /// Identifier for a `lahan` (card/task).
    LahanId
);
define_id!(
    /// Identifier for a `buah` (single agent run).
    BuahId
);
```

- [ ] **Step 5: Run tests — expect PASS**

```bash
cargo test -p kulisawit-core ids
```

Expected: 4 passing.

- [ ] **Step 6: Clippy**

```bash
cargo clippy -p kulisawit-core --all-targets -- -D warnings
```

- [ ] **Step 7: Commit**

```bash
git add crates/kulisawit-core
git commit -m "feat(core): add BuahId/LahanId/KebunId/ColumnId newtypes"
```

---

### Task 1.4: Status enums

**Files:**
- Create: `crates/kulisawit-core/src/status.rs`
- Modify: `crates/kulisawit-core/src/lib.rs`

- [ ] **Step 1: Write failing tests**

`crates/kulisawit-core/src/status.rs`:

```rust
//! Status enums for runs, buah, and sortir.

use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_status_serializes_snake_case() {
        let json = serde_json::to_string(&RunStatus::InProgress).expect("ser");
        assert_eq!(json, "\"in_progress\"");
    }

    #[test]
    fn buah_status_parses_from_db_string() {
        assert_eq!(BuahStatus::try_from("queued").expect("q"), BuahStatus::Queued);
        assert_eq!(BuahStatus::try_from("running").expect("r"), BuahStatus::Running);
        assert_eq!(BuahStatus::try_from("completed").expect("c"), BuahStatus::Completed);
        assert_eq!(BuahStatus::try_from("failed").expect("f"), BuahStatus::Failed);
        assert_eq!(BuahStatus::try_from("cancelled").expect("x"), BuahStatus::Cancelled);
        assert!(BuahStatus::try_from("banana").is_err());
    }

    #[test]
    fn buah_status_round_trips_to_str() {
        for status in [
            BuahStatus::Queued,
            BuahStatus::Running,
            BuahStatus::Completed,
            BuahStatus::Failed,
            BuahStatus::Cancelled,
        ] {
            let s = status.as_str();
            let back = BuahStatus::try_from(s).expect("roundtrip");
            assert_eq!(back, status);
        }
    }

    #[test]
    fn sortir_status_default_is_pending() {
        assert_eq!(SortirStatus::default(), SortirStatus::Pending);
    }

    #[test]
    fn buah_status_terminal_flag() {
        assert!(!BuahStatus::Queued.is_terminal());
        assert!(!BuahStatus::Running.is_terminal());
        assert!(BuahStatus::Completed.is_terminal());
        assert!(BuahStatus::Failed.is_terminal());
        assert!(BuahStatus::Cancelled.is_terminal());
    }
}
```

Register in `lib.rs`:

```rust
pub mod status;
pub use status::{BuahStatus, RunStatus, SortirStatus};
```

- [ ] **Step 2: Run — expect FAIL**

```bash
cargo test -p kulisawit-core status
```

Expected: FAIL (types missing).

- [ ] **Step 3: Implement**

Append to `crates/kulisawit-core/src/status.rs`:

```rust
/// Lifecycle of a buah from an orchestrator perspective.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuahStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl BuahStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}

#[derive(Debug, thiserror::Error)]
#[error("unknown buah status: {0}")]
pub struct UnknownBuahStatus(pub String);

impl TryFrom<&str> for BuahStatus {
    type Error = UnknownBuahStatus;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "queued" => Ok(Self::Queued),
            "running" => Ok(Self::Running),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            "cancelled" => Ok(Self::Cancelled),
            other => Err(UnknownBuahStatus(other.to_owned())),
        }
    }
}

/// High-level status emitted by an adapter while running.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Starting,
    InProgress,
    Succeeded,
    Failed,
    Cancelled,
}

/// Result of running sortir (verification) commands against a buah.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SortirStatus {
    #[default]
    Pending,
    Passed,
    Failed,
    Skipped,
}
```

Add `thiserror` to core deps if not already present (it is, per Task 1.3).

- [ ] **Step 4: Run — expect PASS**

```bash
cargo test -p kulisawit-core status
```

- [ ] **Step 5: Commit**

```bash
git add crates/kulisawit-core
git commit -m "feat(core): add BuahStatus/RunStatus/SortirStatus enums"
```

---

### Task 1.5: `CoreError` type

**Files:**
- Create: `crates/kulisawit-core/src/error.rs`
- Modify: `crates/kulisawit-core/src/lib.rs`

- [ ] **Step 1: Write the module**

`crates/kulisawit-core/src/error.rs`:

```rust
//! Core-level errors.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("adapter error: {0}")]
    Adapter(String),

    #[error("invalid configuration: {0}")]
    Config(String),

    #[error("invariant violated: {0}")]
    Invariant(&'static str),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serialization error: {0}")]
    Serialize(#[from] serde_json::Error),

    #[error("unknown buah status: {0}")]
    UnknownBuahStatus(#[from] crate::status::UnknownBuahStatus),
}

pub type CoreResult<T> = Result<T, CoreError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_contains_reason() {
        let err = CoreError::Config("missing default_kuli".into());
        assert!(format!("{err}").contains("missing default_kuli"));
    }

    #[test]
    fn io_error_converts_via_from() {
        let io: std::io::Error = std::io::Error::new(std::io::ErrorKind::Other, "boom");
        let err: CoreError = io.into();
        assert!(matches!(err, CoreError::Io(_)));
    }
}
```

Register in `lib.rs`:

```rust
pub mod error;
pub use error::{CoreError, CoreResult};
```

- [ ] **Step 2: Run tests — expect PASS (tests live with impl)**

```bash
cargo test -p kulisawit-core error
```

- [ ] **Step 3: Clippy + commit**

```bash
cargo clippy -p kulisawit-core --all-targets -- -D warnings
git add crates/kulisawit-core
git commit -m "feat(core): add CoreError/CoreResult"
```

---

### Task 1.6: `KuliAdapter` trait + `KuliEvent`

**Files:**
- Create: `crates/kulisawit-core/src/adapter/mod.rs`
- Create: `crates/kulisawit-core/src/adapter/event.rs`
- Create: `crates/kulisawit-core/tests/adapter_object_safety.rs`
- Modify: `crates/kulisawit-core/src/lib.rs`

- [ ] **Step 1: Write the failing object-safety test**

Create `crates/kulisawit-core/tests/adapter_object_safety.rs`:

```rust
//! Compile-time checks: the adapter trait must be dyn-compatible.

use kulisawit_core::adapter::KuliAdapter;
use std::sync::Arc;

#[test]
fn kuli_adapter_is_dyn_compatible() {
    // If this compiles, the trait stays object-safe for the orchestrator's
    // `Arc<dyn KuliAdapter>` registry.
    fn _assert_object_safe(_: Arc<dyn KuliAdapter>) {}
}

#[test]
fn kuli_event_serializes_with_tag() {
    use kulisawit_core::adapter::KuliEvent;
    let evt = KuliEvent::Stdout { text: "hello".into() };
    let json = serde_json::to_string(&evt).expect("ser");
    assert!(json.contains("\"type\":\"stdout\""));
    assert!(json.contains("\"text\":\"hello\""));
}
```

- [ ] **Step 2: Run — expect compile failure (types missing)**

```bash
cargo test -p kulisawit-core --test adapter_object_safety
```

- [ ] **Step 3: Implement `KuliEvent` and context types**

`crates/kulisawit-core/src/adapter/event.rs`:

```rust
//! Events emitted by a kuli while running, plus run-time context types.

use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};

use crate::status::RunStatus;

/// Context handed to a kuli for a single run.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunContext {
    pub run_id: String,
    pub petak_path: PathBuf,
    pub prompt: String,
    pub prompt_variant: Option<String>,
    pub env: HashMap<String, String>,
}

/// Result of an adapter health check.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CheckResult {
    pub ok: bool,
    pub message: Option<String>,
    pub version: Option<String>,
}

/// Structured events streamed from a running kuli.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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

`crates/kulisawit-core/src/adapter/mod.rs`:

```rust
//! The `KuliAdapter` contract every agent integration implements.

use async_trait::async_trait;
use futures::stream::BoxStream;

mod event;
pub use event::{CheckResult, KuliEvent, RunContext};

use crate::error::CoreError;

/// Errors an adapter may return. Kept narrow on purpose — detail goes in the message.
#[derive(Debug, thiserror::Error)]
pub enum KuliError {
    #[error("adapter not ready: {0}")]
    NotReady(String),
    #[error("adapter failed: {0}")]
    Failed(String),
    #[error("cancelled")]
    Cancelled,
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

impl From<KuliError> for CoreError {
    fn from(value: KuliError) -> Self {
        CoreError::Adapter(value.to_string())
    }
}

#[async_trait]
pub trait KuliAdapter: Send + Sync + std::fmt::Debug {
    fn id(&self) -> &str;
    fn display_name(&self) -> &str;
    fn version(&self) -> &str;

    async fn check(&self) -> Result<CheckResult, KuliError>;

    async fn run(
        &self,
        ctx: RunContext,
    ) -> Result<BoxStream<'static, KuliEvent>, KuliError>;

    async fn cancel(&self, run_id: &str) -> Result<(), KuliError>;
}
```

Register in `lib.rs`:

```rust
pub mod adapter;
```

- [ ] **Step 4: Run — expect PASS**

```bash
cargo test -p kulisawit-core --test adapter_object_safety
```

- [ ] **Step 5: Clippy + commit**

```bash
cargo clippy -p kulisawit-core --all-targets -- -D warnings
git add crates/kulisawit-core
git commit -m "feat(core): define KuliAdapter trait and KuliEvent stream type"
```

---

### Task 1.7: SQLx migration `0001_initial.sql`

**Files:**
- Create: `migrations/0001_initial.sql`

- [ ] **Step 1: Write the migration verbatim from PRD §5.5 (with small hardening)**

`migrations/0001_initial.sql`:

```sql
-- Kulisawit initial schema.
-- Table names stay English for code clarity; user-facing UI maps them to
-- kebun/lahan/tandan/buah vocabulary.

PRAGMA foreign_keys = ON;

CREATE TABLE kebun (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    repo_path   TEXT NOT NULL,
    created_at  INTEGER NOT NULL
);

CREATE TABLE columns (
    id         TEXT PRIMARY KEY,
    kebun_id   TEXT NOT NULL REFERENCES kebun(id) ON DELETE CASCADE,
    name       TEXT NOT NULL,
    position   INTEGER NOT NULL
);

CREATE INDEX idx_columns_kebun ON columns(kebun_id, position);

CREATE TABLE lahan (
    id            TEXT PRIMARY KEY,
    kebun_id      TEXT NOT NULL REFERENCES kebun(id) ON DELETE CASCADE,
    column_id     TEXT NOT NULL REFERENCES columns(id) ON DELETE RESTRICT,
    title         TEXT NOT NULL,
    description   TEXT,
    position      INTEGER NOT NULL,
    tags          TEXT,            -- JSON array
    linked_files  TEXT,            -- JSON array of repo-relative paths
    created_at    INTEGER NOT NULL,
    updated_at    INTEGER NOT NULL
);

CREATE INDEX idx_lahan_kebun ON lahan(kebun_id, column_id, position);

CREATE TABLE buah (
    id             TEXT PRIMARY KEY,
    lahan_id       TEXT NOT NULL REFERENCES lahan(id) ON DELETE CASCADE,
    kuli_id        TEXT NOT NULL,
    prompt_variant TEXT,
    petak_path     TEXT NOT NULL,
    branch_name    TEXT NOT NULL,
    status         TEXT NOT NULL CHECK (status IN ('queued','running','completed','failed','cancelled')),
    started_at     INTEGER,
    completed_at   INTEGER,
    sortir_status  TEXT CHECK (sortir_status IN ('pending','passed','failed','skipped')),
    sortir_output  TEXT
);

CREATE INDEX idx_buah_lahan ON buah(lahan_id);
CREATE INDEX idx_buah_status ON buah(status);

CREATE TABLE events (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    buah_id    TEXT NOT NULL REFERENCES buah(id) ON DELETE CASCADE,
    timestamp  INTEGER NOT NULL,
    type       TEXT NOT NULL,
    payload    TEXT NOT NULL      -- JSON
);

CREATE INDEX idx_events_buah ON events(buah_id, timestamp);
```

- [ ] **Step 2: Commit the migration alone (so it's visible in git before code references it)**

```bash
git add migrations/0001_initial.sql
git commit -m "feat(db): initial SQLite schema per PRD §5.5"
```

---

### Task 1.8: DB pool + migrator (`kulisawit-db`)

**Files:**
- Modify: `crates/kulisawit-db/Cargo.toml`
- Create: `crates/kulisawit-db/src/error.rs`
- Create: `crates/kulisawit-db/src/pool.rs`
- Modify: `crates/kulisawit-db/src/lib.rs`
- Create: `crates/kulisawit-db/tests/migrations.rs`

- [ ] **Step 1: Add deps**

`crates/kulisawit-db/Cargo.toml`:

```toml
[dependencies]
sqlx.workspace = true
thiserror.workspace = true
tokio.workspace = true
tracing.workspace = true
serde.workspace = true
serde_json.workspace = true
chrono.workspace = true
kulisawit-core.workspace = true

[dev-dependencies]
tokio = { workspace = true, features = ["macros", "rt-multi-thread"] }
tempfile.workspace = true
```

- [ ] **Step 2: Write the failing integration test**

`crates/kulisawit-db/tests/migrations.rs`:

```rust
use kulisawit_db::pool::{connect, migrate};

#[tokio::test]
async fn migrations_apply_cleanly_to_memory_db() {
    let pool = connect("sqlite::memory:").await.expect("connect");
    migrate(&pool).await.expect("migrate");

    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name",
    )
    .fetch_all(&pool)
    .await
    .expect("query");

    let names: Vec<&str> = rows.iter().map(|(n,)| n.as_str()).collect();
    assert!(names.contains(&"kebun"));
    assert!(names.contains(&"columns"));
    assert!(names.contains(&"lahan"));
    assert!(names.contains(&"buah"));
    assert!(names.contains(&"events"));
}
```

- [ ] **Step 3: Run — expect FAIL (module doesn't exist)**

```bash
cargo test -p kulisawit-db --test migrations
```

- [ ] **Step 4: Implement `DbError` and `pool`**

`crates/kulisawit-db/src/error.rs`:

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("sqlx: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("migration: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),
    #[error("invalid row: {0}")]
    Invalid(String),
    #[error("serde: {0}")]
    Serde(#[from] serde_json::Error),
}

pub type DbResult<T> = Result<T, DbError>;
```

`crates/kulisawit-db/src/pool.rs`:

```rust
//! SQLite connection pool and migration runner.

use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
    Pool, Sqlite,
};
use std::str::FromStr;

use crate::error::DbResult;

pub type DbPool = Pool<Sqlite>;

/// Open (or create) a SQLite database at the given URL or path.
///
/// Accepts `sqlite::memory:`, `sqlite://path?...`, or a bare filesystem path.
pub async fn connect(url_or_path: &str) -> DbResult<DbPool> {
    let opts = if url_or_path.starts_with("sqlite:") {
        SqliteConnectOptions::from_str(url_or_path)?
    } else {
        SqliteConnectOptions::new()
            .filename(url_or_path)
            .create_if_missing(true)
    }
    .journal_mode(SqliteJournalMode::Wal)
    .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(8)
        .connect_with(opts)
        .await?;
    Ok(pool)
}

/// Apply all pending migrations from the `migrations/` directory at the
/// workspace root.
pub async fn migrate(pool: &DbPool) -> DbResult<()> {
    sqlx::migrate!("../../migrations").run(pool).await?;
    Ok(())
}
```

`crates/kulisawit-db/src/lib.rs`:

```rust
//! SQLite repositories for Kulisawit

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![warn(missing_debug_implementations, rust_2018_idioms)]

pub mod error;
pub mod pool;

pub use error::{DbError, DbResult};
pub use pool::{connect, migrate, DbPool};
```

- [ ] **Step 5: Run — expect PASS**

```bash
cargo test -p kulisawit-db --test migrations
```

- [ ] **Step 6: Prepare offline metadata**

```bash
cargo sqlx prepare --workspace -- --all-targets
```

Commit `.sqlx/` directory (will populate in later tasks once `query!` macros appear).

- [ ] **Step 7: Commit**

```bash
git add crates/kulisawit-db Cargo.lock
git commit -m "feat(db): add pool + migrator and verify schema apply"
```

---

### Task 1.9: `Kebun` repository

**Files:**
- Create: `crates/kulisawit-db/src/kebun.rs`
- Modify: `crates/kulisawit-db/src/lib.rs`
- Create: `crates/kulisawit-db/tests/kebun_repo.rs`

- [ ] **Step 1: Write failing integration test**

`crates/kulisawit-db/tests/kebun_repo.rs`:

```rust
use kulisawit_core::KebunId;
use kulisawit_db::{connect, kebun, migrate};

async fn fresh_pool() -> kulisawit_db::DbPool {
    let pool = connect("sqlite::memory:").await.expect("connect");
    migrate(&pool).await.expect("migrate");
    pool
}

#[tokio::test]
async fn create_then_get_returns_same_row() {
    let pool = fresh_pool().await;
    let record = kebun::NewKebun {
        name: "Demo".into(),
        repo_path: "/tmp/demo".into(),
    };
    let id = kebun::create(&pool, record).await.expect("create");
    let fetched = kebun::get(&pool, &id).await.expect("get").expect("row");
    assert_eq!(fetched.name, "Demo");
    assert_eq!(fetched.repo_path, "/tmp/demo");
    assert_eq!(fetched.id, id);
}

#[tokio::test]
async fn list_returns_rows_ordered_by_created_at_desc() {
    let pool = fresh_pool().await;
    let a = kebun::create(
        &pool,
        kebun::NewKebun { name: "A".into(), repo_path: "/a".into() },
    )
    .await
    .expect("a");
    let b = kebun::create(
        &pool,
        kebun::NewKebun { name: "B".into(), repo_path: "/b".into() },
    )
    .await
    .expect("b");
    let rows = kebun::list(&pool).await.expect("list");
    assert_eq!(rows.len(), 2);
    // b was created later → comes first.
    assert_eq!(rows[0].id, b);
    assert_eq!(rows[1].id, a);
}

#[tokio::test]
async fn get_missing_returns_none() {
    let pool = fresh_pool().await;
    let result = kebun::get(&pool, &KebunId::new()).await.expect("ok");
    assert!(result.is_none());
}
```

- [ ] **Step 2: Run — expect FAIL (module missing)**

```bash
cargo test -p kulisawit-db --test kebun_repo
```

- [ ] **Step 3: Implement**

`crates/kulisawit-db/src/kebun.rs`:

```rust
//! Kebun repository functions.

use chrono::Utc;
use kulisawit_core::KebunId;
use serde::{Deserialize, Serialize};

use crate::{DbPool, DbResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewKebun {
    pub name: String,
    pub repo_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Kebun {
    pub id: KebunId,
    pub name: String,
    pub repo_path: String,
    pub created_at: i64,
}

pub async fn create(pool: &DbPool, new: NewKebun) -> DbResult<KebunId> {
    let id = KebunId::new();
    let created_at = Utc::now().timestamp();
    let id_str = id.as_str();
    sqlx::query!(
        "INSERT INTO kebun (id, name, repo_path, created_at) VALUES (?, ?, ?, ?)",
        id_str,
        new.name,
        new.repo_path,
        created_at
    )
    .execute(pool)
    .await?;
    Ok(id)
}

pub async fn get(pool: &DbPool, id: &KebunId) -> DbResult<Option<Kebun>> {
    let id_str = id.as_str();
    let row = sqlx::query!(
        "SELECT id, name, repo_path, created_at FROM kebun WHERE id = ?",
        id_str
    )
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|r| Kebun {
        id: KebunId::from_string(r.id),
        name: r.name,
        repo_path: r.repo_path,
        created_at: r.created_at,
    }))
}

pub async fn list(pool: &DbPool) -> DbResult<Vec<Kebun>> {
    let rows = sqlx::query!(
        "SELECT id, name, repo_path, created_at FROM kebun ORDER BY created_at DESC"
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| Kebun {
            id: KebunId::from_string(r.id),
            name: r.name,
            repo_path: r.repo_path,
            created_at: r.created_at,
        })
        .collect())
}
```

Register in `lib.rs`:

```rust
pub mod kebun;
```

- [ ] **Step 4: Refresh offline SQLx metadata**

Because tests use the `query!` macro, you need a live DB for the first run **or** to have the SQLx offline data already. Use `SQLX_OFFLINE=true` once metadata is committed; initially, run:

```bash
export DATABASE_URL="sqlite://$(pwd)/.kulisawit-dev.sqlite"
sqlx database create
sqlx migrate run --source migrations
cargo sqlx prepare --workspace -- --all-targets
unset DATABASE_URL
rm .kulisawit-dev.sqlite
```

- [ ] **Step 5: Run — expect PASS**

```bash
cargo test -p kulisawit-db --test kebun_repo
```

- [ ] **Step 6: Commit**

```bash
git add crates/kulisawit-db .sqlx
git commit -m "feat(db): kebun repository (create/get/list)"
```

---

### Task 1.10: `Column` repository + default kanban setup

**Files:**
- Create: `crates/kulisawit-db/src/columns.rs`
- Modify: `crates/kulisawit-db/src/lib.rs`
- Create: `crates/kulisawit-db/tests/columns_repo.rs`

- [ ] **Step 1: Write failing test**

`crates/kulisawit-db/tests/columns_repo.rs`:

```rust
use kulisawit_db::{columns, connect, kebun, migrate};

async fn setup() -> (kulisawit_db::DbPool, kulisawit_core::KebunId) {
    let pool = connect("sqlite::memory:").await.expect("pool");
    migrate(&pool).await.expect("mig");
    let id = kebun::create(
        &pool,
        kebun::NewKebun { name: "K".into(), repo_path: "/k".into() },
    )
    .await
    .expect("kebun");
    (pool, id)
}

#[tokio::test]
async fn seed_defaults_creates_five_columns_in_order() {
    let (pool, kebun_id) = setup().await;
    columns::seed_defaults(&pool, &kebun_id).await.expect("seed");
    let cols = columns::list_for_kebun(&pool, &kebun_id).await.expect("list");
    let names: Vec<&str> = cols.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, vec!["Backlog", "Todo", "Doing", "Review", "Done"]);
    // Positions are dense starting at 0.
    let positions: Vec<i64> = cols.iter().map(|c| c.position).collect();
    assert_eq!(positions, vec![0, 1, 2, 3, 4]);
}
```

- [ ] **Step 2: Run — expect FAIL**

```bash
cargo test -p kulisawit-db --test columns_repo
```

- [ ] **Step 3: Implement**

`crates/kulisawit-db/src/columns.rs`:

```rust
//! Kanban column repository.

use kulisawit_core::{ColumnId, KebunId};
use serde::{Deserialize, Serialize};

use crate::{DbPool, DbResult};

pub const DEFAULT_COLUMN_NAMES: [&str; 5] =
    ["Backlog", "Todo", "Doing", "Review", "Done"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Column {
    pub id: ColumnId,
    pub kebun_id: KebunId,
    pub name: String,
    pub position: i64,
}

pub async fn seed_defaults(pool: &DbPool, kebun_id: &KebunId) -> DbResult<Vec<ColumnId>> {
    let mut ids = Vec::with_capacity(DEFAULT_COLUMN_NAMES.len());
    for (idx, name) in DEFAULT_COLUMN_NAMES.iter().enumerate() {
        let id = ColumnId::new();
        let id_str = id.as_str();
        let kebun_str = kebun_id.as_str();
        let pos = idx as i64;
        sqlx::query!(
            "INSERT INTO columns (id, kebun_id, name, position) VALUES (?, ?, ?, ?)",
            id_str,
            kebun_str,
            name,
            pos
        )
        .execute(pool)
        .await?;
        ids.push(id);
    }
    Ok(ids)
}

pub async fn list_for_kebun(
    pool: &DbPool,
    kebun_id: &KebunId,
) -> DbResult<Vec<Column>> {
    let kebun_str = kebun_id.as_str();
    let rows = sqlx::query!(
        "SELECT id, kebun_id, name, position FROM columns WHERE kebun_id = ? ORDER BY position ASC",
        kebun_str
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| Column {
            id: ColumnId::from_string(r.id),
            kebun_id: KebunId::from_string(r.kebun_id),
            name: r.name,
            position: r.position,
        })
        .collect())
}
```

Register in `lib.rs`:

```rust
pub mod columns;
```

- [ ] **Step 4: Refresh offline metadata + run + commit**

```bash
cargo sqlx prepare --workspace -- --all-targets
cargo test -p kulisawit-db --test columns_repo
git add crates/kulisawit-db .sqlx
git commit -m "feat(db): column repository + default kanban columns"
```

---

### Task 1.11: `Lahan` repository

**Files:**
- Create: `crates/kulisawit-db/src/lahan.rs`
- Modify: `crates/kulisawit-db/src/lib.rs`
- Create: `crates/kulisawit-db/tests/lahan_repo.rs`

- [ ] **Step 1: Write failing tests**

`crates/kulisawit-db/tests/lahan_repo.rs`:

```rust
use kulisawit_core::{ColumnId, KebunId};
use kulisawit_db::{columns, connect, kebun, lahan, migrate};

async fn setup() -> (kulisawit_db::DbPool, KebunId, ColumnId) {
    let pool = connect("sqlite::memory:").await.expect("pool");
    migrate(&pool).await.expect("mig");
    let kebun_id = kebun::create(
        &pool,
        kebun::NewKebun { name: "K".into(), repo_path: "/k".into() },
    )
    .await
    .expect("kebun");
    let col_ids = columns::seed_defaults(&pool, &kebun_id).await.expect("seed");
    (pool, kebun_id, col_ids[0].clone())
}

#[tokio::test]
async fn create_lahan_and_fetch_by_id() {
    let (pool, kebun_id, col_id) = setup().await;
    let id = lahan::create(
        &pool,
        lahan::NewLahan {
            kebun_id: kebun_id.clone(),
            column_id: col_id.clone(),
            title: "add rate limit to /login".into(),
            description: Some("describe.".into()),
            tags: vec!["auth".into()],
            linked_files: vec!["src/auth.rs".into()],
        },
    )
    .await
    .expect("create");
    let l = lahan::get(&pool, &id).await.expect("get").expect("row");
    assert_eq!(l.title, "add rate limit to /login");
    assert_eq!(l.tags, vec!["auth".to_string()]);
    assert_eq!(l.linked_files, vec!["src/auth.rs".to_string()]);
    assert_eq!(l.column_id, col_id);
}

#[tokio::test]
async fn list_for_column_returns_in_position_order() {
    let (pool, kebun_id, col_id) = setup().await;
    for title in ["first", "second", "third"] {
        lahan::create(
            &pool,
            lahan::NewLahan {
                kebun_id: kebun_id.clone(),
                column_id: col_id.clone(),
                title: title.into(),
                description: None,
                tags: vec![],
                linked_files: vec![],
            },
        )
        .await
        .expect("create");
    }
    let rows = lahan::list_for_column(&pool, &col_id).await.expect("list");
    let titles: Vec<&str> = rows.iter().map(|l| l.title.as_str()).collect();
    assert_eq!(titles, vec!["first", "second", "third"]);
}

#[tokio::test]
async fn update_title_and_description() {
    let (pool, kebun_id, col_id) = setup().await;
    let id = lahan::create(
        &pool,
        lahan::NewLahan {
            kebun_id,
            column_id: col_id,
            title: "old".into(),
            description: None,
            tags: vec![],
            linked_files: vec![],
        },
    )
    .await
    .expect("create");
    lahan::update_text(&pool, &id, "new", Some("fresh desc")).await.expect("update");
    let l = lahan::get(&pool, &id).await.expect("ok").expect("row");
    assert_eq!(l.title, "new");
    assert_eq!(l.description.as_deref(), Some("fresh desc"));
}

#[tokio::test]
async fn move_lahan_to_other_column_updates_column_id_and_bumps_position() {
    let (pool, kebun_id, col_id) = setup().await;
    let cols = columns::list_for_kebun(&pool, &kebun_id).await.expect("cols");
    let target = &cols[2]; // "Doing"
    let id = lahan::create(
        &pool,
        lahan::NewLahan {
            kebun_id,
            column_id: col_id,
            title: "x".into(),
            description: None,
            tags: vec![],
            linked_files: vec![],
        },
    )
    .await
    .expect("create");
    lahan::move_to_column(&pool, &id, &target.id).await.expect("move");
    let l = lahan::get(&pool, &id).await.expect("ok").expect("row");
    assert_eq!(l.column_id, target.id);
}
```

- [ ] **Step 2: Run — expect FAIL**

```bash
cargo test -p kulisawit-db --test lahan_repo
```

- [ ] **Step 3: Implement**

`crates/kulisawit-db/src/lahan.rs`:

```rust
//! Lahan (card/task) repository.

use chrono::Utc;
use kulisawit_core::{ColumnId, KebunId, LahanId};
use serde::{Deserialize, Serialize};

use crate::{DbError, DbPool, DbResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewLahan {
    pub kebun_id: KebunId,
    pub column_id: ColumnId,
    pub title: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub linked_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lahan {
    pub id: LahanId,
    pub kebun_id: KebunId,
    pub column_id: ColumnId,
    pub title: String,
    pub description: Option<String>,
    pub position: i64,
    pub tags: Vec<String>,
    pub linked_files: Vec<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

async fn next_position(pool: &DbPool, column_id: &ColumnId) -> DbResult<i64> {
    let col_str = column_id.as_str();
    let row = sqlx::query!(
        "SELECT COALESCE(MAX(position), -1) + 1 AS next FROM lahan WHERE column_id = ?",
        col_str
    )
    .fetch_one(pool)
    .await?;
    Ok(row.next.unwrap_or(0))
}

pub async fn create(pool: &DbPool, new: NewLahan) -> DbResult<LahanId> {
    let id = LahanId::new();
    let now = Utc::now().timestamp();
    let position = next_position(pool, &new.column_id).await?;
    let tags_json = serde_json::to_string(&new.tags)?;
    let files_json = serde_json::to_string(&new.linked_files)?;
    let id_str = id.as_str();
    let kebun_str = new.kebun_id.as_str();
    let col_str = new.column_id.as_str();
    sqlx::query!(
        "INSERT INTO lahan (id, kebun_id, column_id, title, description, position, tags, linked_files, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        id_str,
        kebun_str,
        col_str,
        new.title,
        new.description,
        position,
        tags_json,
        files_json,
        now,
        now
    )
    .execute(pool)
    .await?;
    Ok(id)
}

fn parse_string_list(raw: Option<&str>) -> DbResult<Vec<String>> {
    Ok(match raw {
        None => vec![],
        Some(s) => serde_json::from_str(s).map_err(DbError::from)?,
    })
}

pub async fn get(pool: &DbPool, id: &LahanId) -> DbResult<Option<Lahan>> {
    let id_str = id.as_str();
    let row = sqlx::query!(
        "SELECT id, kebun_id, column_id, title, description, position, tags, linked_files, created_at, updated_at
         FROM lahan WHERE id = ?",
        id_str
    )
    .fetch_optional(pool)
    .await?;
    row.map(|r| {
        Ok::<_, DbError>(Lahan {
            id: LahanId::from_string(r.id),
            kebun_id: KebunId::from_string(r.kebun_id),
            column_id: ColumnId::from_string(r.column_id),
            title: r.title,
            description: r.description,
            position: r.position,
            tags: parse_string_list(r.tags.as_deref())?,
            linked_files: parse_string_list(r.linked_files.as_deref())?,
            created_at: r.created_at,
            updated_at: r.updated_at,
        })
    })
    .transpose()
}

pub async fn list_for_column(pool: &DbPool, column_id: &ColumnId) -> DbResult<Vec<Lahan>> {
    let col_str = column_id.as_str();
    let rows = sqlx::query!(
        "SELECT id, kebun_id, column_id, title, description, position, tags, linked_files, created_at, updated_at
         FROM lahan WHERE column_id = ? ORDER BY position ASC",
        col_str
    )
    .fetch_all(pool)
    .await?;
    rows.into_iter()
        .map(|r| {
            Ok::<_, DbError>(Lahan {
                id: LahanId::from_string(r.id),
                kebun_id: KebunId::from_string(r.kebun_id),
                column_id: ColumnId::from_string(r.column_id),
                title: r.title,
                description: r.description,
                position: r.position,
                tags: parse_string_list(r.tags.as_deref())?,
                linked_files: parse_string_list(r.linked_files.as_deref())?,
                created_at: r.created_at,
                updated_at: r.updated_at,
            })
        })
        .collect()
}

pub async fn update_text(
    pool: &DbPool,
    id: &LahanId,
    title: &str,
    description: Option<&str>,
) -> DbResult<()> {
    let now = Utc::now().timestamp();
    let id_str = id.as_str();
    sqlx::query!(
        "UPDATE lahan SET title = ?, description = ?, updated_at = ? WHERE id = ?",
        title,
        description,
        now,
        id_str
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn move_to_column(
    pool: &DbPool,
    id: &LahanId,
    column_id: &ColumnId,
) -> DbResult<()> {
    let position = next_position(pool, column_id).await?;
    let now = Utc::now().timestamp();
    let id_str = id.as_str();
    let col_str = column_id.as_str();
    sqlx::query!(
        "UPDATE lahan SET column_id = ?, position = ?, updated_at = ? WHERE id = ?",
        col_str,
        position,
        now,
        id_str
    )
    .execute(pool)
    .await?;
    Ok(())
}
```

Register in `lib.rs`:

```rust
pub mod lahan;
```

- [ ] **Step 4: Refresh SQLx metadata, run, commit**

```bash
cargo sqlx prepare --workspace -- --all-targets
cargo test -p kulisawit-db --test lahan_repo
git add crates/kulisawit-db .sqlx
git commit -m "feat(db): lahan repository (create/get/list/update/move)"
```

---

### Task 1.12: `Buah` repository

**Files:**
- Create: `crates/kulisawit-db/src/buah.rs`
- Modify: `crates/kulisawit-db/src/lib.rs`
- Create: `crates/kulisawit-db/tests/buah_repo.rs`

- [ ] **Step 1: Write failing tests**

`crates/kulisawit-db/tests/buah_repo.rs`:

```rust
use kulisawit_core::{BuahStatus, KebunId, LahanId};
use kulisawit_db::{buah, columns, connect, kebun, lahan, migrate};

async fn setup_lahan() -> (kulisawit_db::DbPool, LahanId) {
    let pool = connect("sqlite::memory:").await.expect("pool");
    migrate(&pool).await.expect("mig");
    let kebun_id = kebun::create(
        &pool,
        kebun::NewKebun { name: "K".into(), repo_path: "/k".into() },
    )
    .await
    .expect("kebun");
    let cols = columns::seed_defaults(&pool, &kebun_id).await.expect("cols");
    let lahan_id = lahan::create(
        &pool,
        lahan::NewLahan {
            kebun_id,
            column_id: cols[0].clone(),
            title: "t".into(),
            description: None,
            tags: vec![],
            linked_files: vec![],
        },
    )
    .await
    .expect("lahan");
    (pool, lahan_id)
}

#[tokio::test]
async fn create_buah_defaults_to_queued() {
    let (pool, lahan_id) = setup_lahan().await;
    let id = buah::create(
        &pool,
        buah::NewBuah {
            lahan_id,
            kuli_id: "mock".into(),
            prompt_variant: None,
            petak_path: "/tmp/petak-1".into(),
            branch_name: "kulisawit/l1/b1".into(),
        },
    )
    .await
    .expect("create");
    let b = buah::get(&pool, &id).await.expect("ok").expect("row");
    assert_eq!(b.status, BuahStatus::Queued);
    assert!(b.started_at.is_none());
    assert!(b.completed_at.is_none());
}

#[tokio::test]
async fn transition_queued_to_running_sets_started_at() {
    let (pool, lahan_id) = setup_lahan().await;
    let id = buah::create(
        &pool,
        buah::NewBuah {
            lahan_id,
            kuli_id: "mock".into(),
            prompt_variant: None,
            petak_path: "/tmp/x".into(),
            branch_name: "kulisawit/x".into(),
        },
    )
    .await
    .expect("c");
    buah::mark_running(&pool, &id).await.expect("running");
    let b = buah::get(&pool, &id).await.expect("ok").expect("row");
    assert_eq!(b.status, BuahStatus::Running);
    assert!(b.started_at.is_some());
}

#[tokio::test]
async fn mark_terminal_sets_completed_at() {
    let (pool, lahan_id) = setup_lahan().await;
    let id = buah::create(
        &pool,
        buah::NewBuah {
            lahan_id,
            kuli_id: "mock".into(),
            prompt_variant: None,
            petak_path: "/tmp/y".into(),
            branch_name: "kulisawit/y".into(),
        },
    )
    .await
    .expect("c");
    buah::mark_running(&pool, &id).await.expect("r");
    buah::mark_terminal(&pool, &id, BuahStatus::Completed).await.expect("done");
    let b = buah::get(&pool, &id).await.expect("ok").expect("row");
    assert_eq!(b.status, BuahStatus::Completed);
    assert!(b.completed_at.is_some());
}

#[tokio::test]
async fn list_for_lahan_returns_all() {
    let (pool, lahan_id) = setup_lahan().await;
    for i in 0..3 {
        buah::create(
            &pool,
            buah::NewBuah {
                lahan_id: lahan_id.clone(),
                kuli_id: "mock".into(),
                prompt_variant: None,
                petak_path: format!("/tmp/{i}"),
                branch_name: format!("b-{i}"),
            },
        )
        .await
        .expect("c");
    }
    let rows = buah::list_for_lahan(&pool, &lahan_id).await.expect("list");
    assert_eq!(rows.len(), 3);
}
```

- [ ] **Step 2: Run — expect FAIL**

```bash
cargo test -p kulisawit-db --test buah_repo
```

- [ ] **Step 3: Implement**

`crates/kulisawit-db/src/buah.rs`:

```rust
//! Buah (single agent run) repository.

use chrono::Utc;
use kulisawit_core::{BuahId, BuahStatus, LahanId, SortirStatus};
use serde::{Deserialize, Serialize};

use crate::{DbError, DbPool, DbResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewBuah {
    pub lahan_id: LahanId,
    pub kuli_id: String,
    pub prompt_variant: Option<String>,
    pub petak_path: String,
    pub branch_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Buah {
    pub id: BuahId,
    pub lahan_id: LahanId,
    pub kuli_id: String,
    pub prompt_variant: Option<String>,
    pub petak_path: String,
    pub branch_name: String,
    pub status: BuahStatus,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
    pub sortir_status: Option<SortirStatus>,
    pub sortir_output: Option<String>,
}

fn row_to_buah(
    id: String,
    lahan_id: String,
    kuli_id: String,
    prompt_variant: Option<String>,
    petak_path: String,
    branch_name: String,
    status: String,
    started_at: Option<i64>,
    completed_at: Option<i64>,
    sortir_status: Option<String>,
    sortir_output: Option<String>,
) -> DbResult<Buah> {
    let status = BuahStatus::try_from(status.as_str())
        .map_err(|e| DbError::Invalid(e.to_string()))?;
    let sortir_status = sortir_status
        .as_deref()
        .map(parse_sortir_status)
        .transpose()?;
    Ok(Buah {
        id: BuahId::from_string(id),
        lahan_id: LahanId::from_string(lahan_id),
        kuli_id,
        prompt_variant,
        petak_path,
        branch_name,
        status,
        started_at,
        completed_at,
        sortir_status,
        sortir_output,
    })
}

fn parse_sortir_status(s: &str) -> DbResult<SortirStatus> {
    Ok(match s {
        "pending" => SortirStatus::Pending,
        "passed" => SortirStatus::Passed,
        "failed" => SortirStatus::Failed,
        "skipped" => SortirStatus::Skipped,
        other => return Err(DbError::Invalid(format!("sortir_status={other}"))),
    })
}

pub async fn create(pool: &DbPool, new: NewBuah) -> DbResult<BuahId> {
    let id = BuahId::new();
    let id_str = id.as_str();
    let lahan_str = new.lahan_id.as_str();
    let status = BuahStatus::Queued.as_str();
    sqlx::query!(
        "INSERT INTO buah (id, lahan_id, kuli_id, prompt_variant, petak_path, branch_name, status)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
        id_str,
        lahan_str,
        new.kuli_id,
        new.prompt_variant,
        new.petak_path,
        new.branch_name,
        status
    )
    .execute(pool)
    .await?;
    Ok(id)
}

pub async fn get(pool: &DbPool, id: &BuahId) -> DbResult<Option<Buah>> {
    let id_str = id.as_str();
    let row = sqlx::query!(
        "SELECT id, lahan_id, kuli_id, prompt_variant, petak_path, branch_name, status,
                started_at, completed_at, sortir_status, sortir_output
         FROM buah WHERE id = ?",
        id_str
    )
    .fetch_optional(pool)
    .await?;
    row.map(|r| {
        row_to_buah(
            r.id,
            r.lahan_id,
            r.kuli_id,
            r.prompt_variant,
            r.petak_path,
            r.branch_name,
            r.status,
            r.started_at,
            r.completed_at,
            r.sortir_status,
            r.sortir_output,
        )
    })
    .transpose()
}

pub async fn list_for_lahan(pool: &DbPool, lahan_id: &LahanId) -> DbResult<Vec<Buah>> {
    let l = lahan_id.as_str();
    let rows = sqlx::query!(
        "SELECT id, lahan_id, kuli_id, prompt_variant, petak_path, branch_name, status,
                started_at, completed_at, sortir_status, sortir_output
         FROM buah WHERE lahan_id = ? ORDER BY id ASC",
        l
    )
    .fetch_all(pool)
    .await?;
    rows.into_iter()
        .map(|r| {
            row_to_buah(
                r.id, r.lahan_id, r.kuli_id, r.prompt_variant, r.petak_path,
                r.branch_name, r.status, r.started_at, r.completed_at,
                r.sortir_status, r.sortir_output,
            )
        })
        .collect()
}

pub async fn mark_running(pool: &DbPool, id: &BuahId) -> DbResult<()> {
    let now = Utc::now().timestamp();
    let id_str = id.as_str();
    let status = BuahStatus::Running.as_str();
    sqlx::query!(
        "UPDATE buah SET status = ?, started_at = ? WHERE id = ?",
        status,
        now,
        id_str
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn mark_terminal(
    pool: &DbPool,
    id: &BuahId,
    status: BuahStatus,
) -> DbResult<()> {
    if !status.is_terminal() {
        return Err(DbError::Invalid(format!(
            "mark_terminal called with non-terminal status: {status:?}"
        )));
    }
    let now = Utc::now().timestamp();
    let id_str = id.as_str();
    let status_str = status.as_str();
    sqlx::query!(
        "UPDATE buah SET status = ?, completed_at = ? WHERE id = ?",
        status_str,
        now,
        id_str
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn set_sortir(
    pool: &DbPool,
    id: &BuahId,
    status: SortirStatus,
    output: Option<&str>,
) -> DbResult<()> {
    let status_str = match status {
        SortirStatus::Pending => "pending",
        SortirStatus::Passed => "passed",
        SortirStatus::Failed => "failed",
        SortirStatus::Skipped => "skipped",
    };
    let id_str = id.as_str();
    sqlx::query!(
        "UPDATE buah SET sortir_status = ?, sortir_output = ? WHERE id = ?",
        status_str,
        output,
        id_str
    )
    .execute(pool)
    .await?;
    Ok(())
}
```

Register in `lib.rs`:

```rust
pub mod buah;
```

- [ ] **Step 4: Refresh, run, commit**

```bash
cargo sqlx prepare --workspace -- --all-targets
cargo test -p kulisawit-db --test buah_repo
git add crates/kulisawit-db .sqlx
git commit -m "feat(db): buah repository with status transitions and sortir fields"
```

---

### Task 1.13: Event log repository

**Files:**
- Create: `crates/kulisawit-db/src/events.rs`
- Modify: `crates/kulisawit-db/src/lib.rs`
- Create: `crates/kulisawit-db/tests/events_repo.rs`

- [ ] **Step 1: Write failing test**

`crates/kulisawit-db/tests/events_repo.rs`:

```rust
use kulisawit_core::adapter::KuliEvent;
use kulisawit_db::{buah, columns, connect, events, kebun, lahan, migrate};

async fn setup() -> (kulisawit_db::DbPool, kulisawit_core::BuahId) {
    let pool = connect("sqlite::memory:").await.expect("pool");
    migrate(&pool).await.expect("mig");
    let kebun_id = kebun::create(
        &pool,
        kebun::NewKebun { name: "K".into(), repo_path: "/k".into() },
    ).await.expect("k");
    let cols = columns::seed_defaults(&pool, &kebun_id).await.expect("c");
    let lahan_id = lahan::create(
        &pool,
        lahan::NewLahan {
            kebun_id, column_id: cols[0].clone(), title: "t".into(),
            description: None, tags: vec![], linked_files: vec![],
        },
    ).await.expect("l");
    let buah_id = buah::create(
        &pool,
        buah::NewBuah {
            lahan_id, kuli_id: "mock".into(), prompt_variant: None,
            petak_path: "/x".into(), branch_name: "b".into(),
        },
    ).await.expect("b");
    (pool, buah_id)
}

#[tokio::test]
async fn append_and_read_event_stream_in_order() {
    let (pool, buah_id) = setup().await;
    events::append(&pool, &buah_id, &KuliEvent::Stdout { text: "one".into() }).await.unwrap();
    events::append(&pool, &buah_id, &KuliEvent::Stdout { text: "two".into() }).await.unwrap();
    events::append(&pool, &buah_id, &KuliEvent::FileEdit { path: "a.rs".into(), diff: None }).await.unwrap();

    let evts = events::list_for_buah(&pool, &buah_id).await.unwrap();
    assert_eq!(evts.len(), 3);
    match &evts[0] {
        KuliEvent::Stdout { text } => assert_eq!(text, "one"),
        other => panic!("unexpected first: {other:?}"),
    }
    match &evts[2] {
        KuliEvent::FileEdit { path, .. } => assert_eq!(path, "a.rs"),
        other => panic!("unexpected third: {other:?}"),
    }
}
```

- [ ] **Step 2: Run — expect FAIL**

```bash
cargo test -p kulisawit-db --test events_repo
```

- [ ] **Step 3: Implement**

`crates/kulisawit-db/src/events.rs`:

```rust
//! Per-buah event log repository.

use chrono::Utc;
use kulisawit_core::{adapter::KuliEvent, BuahId};

use crate::{DbPool, DbResult};

pub async fn append(
    pool: &DbPool,
    buah_id: &BuahId,
    event: &KuliEvent,
) -> DbResult<i64> {
    let ts = Utc::now().timestamp_millis();
    let payload = serde_json::to_string(event)?;
    let type_name = match event {
        KuliEvent::Stdout { .. } => "stdout",
        KuliEvent::Stderr { .. } => "stderr",
        KuliEvent::ToolCall { .. } => "tool_call",
        KuliEvent::ToolResult { .. } => "tool_result",
        KuliEvent::FileEdit { .. } => "file_edit",
        KuliEvent::Status { .. } => "status",
    };
    let buah_str = buah_id.as_str();
    let row = sqlx::query!(
        "INSERT INTO events (buah_id, timestamp, type, payload) VALUES (?, ?, ?, ?) RETURNING id",
        buah_str,
        ts,
        type_name,
        payload
    )
    .fetch_one(pool)
    .await?;
    Ok(row.id)
}

pub async fn list_for_buah(pool: &DbPool, buah_id: &BuahId) -> DbResult<Vec<KuliEvent>> {
    let buah_str = buah_id.as_str();
    let rows = sqlx::query!(
        "SELECT payload FROM events WHERE buah_id = ? ORDER BY id ASC",
        buah_str
    )
    .fetch_all(pool)
    .await?;
    rows.into_iter()
        .map(|r| serde_json::from_str::<KuliEvent>(&r.payload).map_err(Into::into))
        .collect()
}
```

Register in `lib.rs`:

```rust
pub mod events;
```

- [ ] **Step 4: Refresh, run, commit**

```bash
cargo sqlx prepare --workspace -- --all-targets
cargo test -p kulisawit-db --test events_repo
git add crates/kulisawit-db .sqlx
git commit -m "feat(db): per-buah event log append/read"
```

---

### Task 1.14: `MockKuli` adapter

**Files:**
- Modify: `crates/kulisawit-kuli/Cargo.toml`
- Create: `crates/kulisawit-kuli/src/mock.rs`
- Modify: `crates/kulisawit-kuli/src/lib.rs`
- Create: `crates/kulisawit-kuli/tests/mock_stream.rs`

- [ ] **Step 1: Add deps**

`crates/kulisawit-kuli/Cargo.toml`:

```toml
[dependencies]
async-trait.workspace = true
futures.workspace = true
tokio.workspace = true
tracing.workspace = true
kulisawit-core.workspace = true
serde_json.workspace = true

[dev-dependencies]
tokio = { workspace = true, features = ["macros", "rt-multi-thread", "time"] }
futures.workspace = true
```

- [ ] **Step 2: Write failing integration test**

`crates/kulisawit-kuli/tests/mock_stream.rs`:

```rust
use futures::StreamExt;
use kulisawit_core::adapter::{KuliAdapter, KuliEvent, RunContext};
use kulisawit_kuli::MockKuli;
use std::collections::HashMap;
use std::path::PathBuf;

fn ctx() -> RunContext {
    RunContext {
        run_id: "run-1".into(),
        petak_path: PathBuf::from("/tmp/unused"),
        prompt: "do it".into(),
        prompt_variant: None,
        env: HashMap::new(),
    }
}

#[tokio::test]
async fn mock_check_reports_ok() {
    let k = MockKuli::default();
    let res = k.check().await.expect("check");
    assert!(res.ok);
}

#[tokio::test]
async fn mock_run_emits_scripted_sequence_ending_in_status_succeeded() {
    let k = MockKuli::default();
    let mut stream = k.run(ctx()).await.expect("run");
    let mut events = vec![];
    while let Some(evt) = stream.next().await {
        events.push(evt);
    }
    assert!(!events.is_empty());
    match events.last().expect("at least one") {
        KuliEvent::Status { status, .. } => assert!(
            matches!(status, kulisawit_core::status::RunStatus::Succeeded)
        ),
        other => panic!("expected terminal Status event, got {other:?}"),
    }
    // Contains at least one tool_call and one file_edit.
    assert!(events.iter().any(|e| matches!(e, KuliEvent::ToolCall { .. })));
    assert!(events.iter().any(|e| matches!(e, KuliEvent::FileEdit { .. })));
}

#[tokio::test]
async fn mock_id_and_display_name_are_stable() {
    let k = MockKuli::default();
    assert_eq!(k.id(), "mock");
    assert_eq!(k.display_name(), "Mock Kuli");
}
```

- [ ] **Step 3: Run — expect FAIL**

```bash
cargo test -p kulisawit-kuli --test mock_stream
```

- [ ] **Step 4: Implement**

`crates/kulisawit-kuli/src/mock.rs`:

```rust
//! A deterministic adapter used for tests and developer smoke runs.

use async_trait::async_trait;
use futures::stream::{self, BoxStream};
use kulisawit_core::{
    adapter::{CheckResult, KuliAdapter, KuliError, KuliEvent, RunContext},
    status::RunStatus,
};
use std::time::Duration;

#[derive(Debug, Default, Clone)]
pub struct MockKuli;

#[async_trait]
impl KuliAdapter for MockKuli {
    fn id(&self) -> &str { "mock" }
    fn display_name(&self) -> &str { "Mock Kuli" }
    fn version(&self) -> &str { env!("CARGO_PKG_VERSION") }

    async fn check(&self) -> Result<CheckResult, KuliError> {
        Ok(CheckResult { ok: true, message: Some("mock ready".into()), version: Some("0".into()) })
    }

    async fn run(
        &self,
        _ctx: RunContext,
    ) -> Result<BoxStream<'static, KuliEvent>, KuliError> {
        let scripted = vec![
            KuliEvent::Status { status: RunStatus::Starting, detail: None },
            KuliEvent::Stdout { text: "Reading repo…".into() },
            KuliEvent::ToolCall {
                name: "read_file".into(),
                input: serde_json::json!({ "path": "README.md" }),
            },
            KuliEvent::ToolResult {
                name: "read_file".into(),
                output: serde_json::json!({ "bytes": 128 }),
            },
            KuliEvent::Stdout { text: "Drafting change…".into() },
            KuliEvent::FileEdit {
                path: "src/lib.rs".into(),
                diff: Some("@@ -1 +1,2 @@\n+// mock edit\n".into()),
            },
            KuliEvent::Status { status: RunStatus::Succeeded, detail: None },
        ];
        // Turn into an async stream with small delays so consumers can observe ordering.
        let s = stream::unfold(scripted.into_iter(), |mut it| async move {
            let next = it.next()?;
            tokio::time::sleep(Duration::from_millis(5)).await;
            Some((next, it))
        });
        Ok(Box::pin(s))
    }

    async fn cancel(&self, _run_id: &str) -> Result<(), KuliError> { Ok(()) }
}
```

`crates/kulisawit-kuli/src/lib.rs`:

```rust
//! Built-in Kuli adapters for Kulisawit

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![warn(missing_debug_implementations, rust_2018_idioms)]

pub mod mock;
pub use mock::MockKuli;
```

- [ ] **Step 5: Run — expect PASS**

```bash
cargo test -p kulisawit-kuli --test mock_stream
```

- [ ] **Step 6: Commit**

```bash
git add crates/kulisawit-kuli
git commit -m "feat(kuli): add MockKuli adapter with scripted event stream"
```

---

### Task 1.15: Petak manager — create and delete worktree

**Files:**
- Modify: `crates/kulisawit-git/Cargo.toml`
- Create: `crates/kulisawit-git/src/error.rs`
- Create: `crates/kulisawit-git/src/petak.rs`
- Modify: `crates/kulisawit-git/src/lib.rs`
- Create: `crates/kulisawit-git/tests/petak_roundtrip.rs`

- [ ] **Step 1: Add deps**

`crates/kulisawit-git/Cargo.toml`:

```toml
[dependencies]
git2.workspace = true
thiserror.workspace = true
tokio.workspace = true
tracing.workspace = true
serde.workspace = true

[dev-dependencies]
tempfile.workspace = true
tokio = { workspace = true, features = ["macros", "rt-multi-thread", "process"] }
```

- [ ] **Step 2: Write failing integration test**

`crates/kulisawit-git/tests/petak_roundtrip.rs`:

```rust
use kulisawit_git::petak::{create_petak, remove_petak, CreatePetakRequest};
use std::process::Command;
use tempfile::tempdir;

fn init_repo_with_commit(dir: &std::path::Path) {
    Command::new("git").args(["init", "-b", "main"]).current_dir(dir).status().unwrap();
    std::fs::write(dir.join("README.md"), "# test\n").unwrap();
    Command::new("git").args(["add", "."]).current_dir(dir).status().unwrap();
    Command::new("git").args(["-c", "user.email=t@t", "-c", "user.name=t", "commit", "-m", "init"])
        .current_dir(dir).status().unwrap();
}

#[tokio::test]
async fn create_and_remove_worktree_roundtrip() {
    let base = tempdir().expect("tmp");
    init_repo_with_commit(base.path());
    let petak_root = base.path().join(".kulisawit/petak");
    let req = CreatePetakRequest {
        repo_root: base.path().to_path_buf(),
        petak_root,
        buah_short_id: "abc123".into(),
        branch_name: "kulisawit/lx/abc123".into(),
        base_ref: "main".into(),
    };
    let outcome = create_petak(req.clone()).await.expect("create");
    assert!(outcome.petak_path.exists());
    assert!(outcome.petak_path.join("README.md").exists());

    remove_petak(&req.repo_root, &outcome.petak_path).await.expect("remove");
    assert!(!outcome.petak_path.exists());
}
```

- [ ] **Step 3: Run — expect FAIL**

```bash
cargo test -p kulisawit-git --test petak_roundtrip
```

- [ ] **Step 4: Implement error type + petak module**

`crates/kulisawit-git/src/error.rs`:

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GitError {
    #[error("git command {command} failed with status {status}: {stderr}")]
    Command { command: String, status: i32, stderr: String },
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("libgit2: {0}")]
    Libgit2(#[from] git2::Error),
    #[error("invalid input: {0}")]
    Invalid(String),
}

pub type GitResult<T> = Result<T, GitError>;
```

`crates/kulisawit-git/src/petak.rs`:

```rust
//! Managing isolated git worktrees ("petak") per buah.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::process::Command;
use tracing::instrument;

use crate::error::{GitError, GitResult};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreatePetakRequest {
    pub repo_root: PathBuf,
    pub petak_root: PathBuf,
    pub buah_short_id: String,
    pub branch_name: String,
    pub base_ref: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreatePetakOutcome {
    pub petak_path: PathBuf,
    pub branch_name: String,
}

async fn run_git(repo_root: &Path, args: &[&str]) -> GitResult<String> {
    let out = Command::new("git")
        .args(args)
        .current_dir(repo_root)
        .output()
        .await?;
    if !out.status.success() {
        return Err(GitError::Command {
            command: format!("git {}", args.join(" ")),
            status: out.status.code().unwrap_or(-1),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        });
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

#[instrument(skip(req), fields(buah = %req.buah_short_id, branch = %req.branch_name))]
pub async fn create_petak(req: CreatePetakRequest) -> GitResult<CreatePetakOutcome> {
    tokio::fs::create_dir_all(&req.petak_root).await?;
    let petak_path = req.petak_root.join(format!("buah-{}", req.buah_short_id));
    if petak_path.exists() {
        return Err(GitError::Invalid(format!(
            "petak path already exists: {}",
            petak_path.display()
        )));
    }
    let petak_str = petak_path.to_string_lossy();
    run_git(
        &req.repo_root,
        &[
            "worktree",
            "add",
            "-b",
            &req.branch_name,
            &petak_str,
            &req.base_ref,
        ],
    )
    .await?;
    Ok(CreatePetakOutcome {
        petak_path,
        branch_name: req.branch_name,
    })
}

#[instrument(skip(repo_root), fields(petak = %petak_path.display()))]
pub async fn remove_petak(repo_root: &Path, petak_path: &Path) -> GitResult<()> {
    let petak_str = petak_path.to_string_lossy();
    run_git(repo_root, &["worktree", "remove", "--force", &petak_str]).await?;
    Ok(())
}
```

`crates/kulisawit-git/src/lib.rs`:

```rust
//! Git worktree (petak) management for Kulisawit

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![warn(missing_debug_implementations, rust_2018_idioms)]

pub mod error;
pub mod petak;

pub use error::{GitError, GitResult};
```

- [ ] **Step 5: Run — expect PASS**

```bash
cargo test -p kulisawit-git --test petak_roundtrip
```

- [ ] **Step 6: Commit**

```bash
git add crates/kulisawit-git
git commit -m "feat(git): petak create/remove via git worktree"
```

---

### Task 1.16: Petak manager — branch commit helper

**Files:**
- Create: `crates/kulisawit-git/src/branch.rs`
- Modify: `crates/kulisawit-git/src/lib.rs`
- Create: `crates/kulisawit-git/tests/branch_commit.rs`

- [ ] **Step 1: Failing test**

`crates/kulisawit-git/tests/branch_commit.rs`:

```rust
use kulisawit_git::branch::commit_all_in_petak;
use kulisawit_git::petak::{create_petak, CreatePetakRequest};
use std::process::Command;
use tempfile::tempdir;

fn init_repo(dir: &std::path::Path) {
    Command::new("git").args(["init", "-b", "main"]).current_dir(dir).status().unwrap();
    std::fs::write(dir.join("README.md"), "# test\n").unwrap();
    Command::new("git").args(["-c","user.email=t@t","-c","user.name=t","add","."]).current_dir(dir).status().unwrap();
    Command::new("git").args(["-c","user.email=t@t","-c","user.name=t","commit","-m","init"]).current_dir(dir).status().unwrap();
}

#[tokio::test]
async fn commit_all_captures_added_file() {
    let base = tempdir().unwrap();
    init_repo(base.path());
    let outcome = create_petak(CreatePetakRequest {
        repo_root: base.path().to_path_buf(),
        petak_root: base.path().join(".kulisawit/petak"),
        buah_short_id: "ab".into(),
        branch_name: "kulisawit/t/ab".into(),
        base_ref: "main".into(),
    })
    .await
    .unwrap();

    // kuli "edits" a file in the petak
    std::fs::write(outcome.petak_path.join("NEW.txt"), "hello\n").unwrap();

    let summary = commit_all_in_petak(&outcome.petak_path, "kulisawit: buah ab for test").await.unwrap();
    assert!(summary.changed); // commit happened
    assert!(summary.message.starts_with("kulisawit: buah ab"));
}

#[tokio::test]
async fn commit_all_no_op_when_clean() {
    let base = tempdir().unwrap();
    init_repo(base.path());
    let outcome = create_petak(CreatePetakRequest {
        repo_root: base.path().to_path_buf(),
        petak_root: base.path().join(".kulisawit/petak"),
        buah_short_id: "cd".into(),
        branch_name: "kulisawit/t/cd".into(),
        base_ref: "main".into(),
    }).await.unwrap();
    let summary = commit_all_in_petak(&outcome.petak_path, "empty").await.unwrap();
    assert!(!summary.changed);
}
```

- [ ] **Step 2: Run — expect FAIL**

```bash
cargo test -p kulisawit-git --test branch_commit
```

- [ ] **Step 3: Implement**

`crates/kulisawit-git/src/branch.rs`:

```rust
//! Branch + commit operations scoped to a single petak.

use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::process::Command;
use tracing::instrument;

use crate::error::{GitError, GitResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitSummary {
    pub changed: bool,
    pub message: String,
    pub commit_sha: Option<String>,
}

async fn git_in(petak: &Path, args: &[&str]) -> GitResult<(i32, String, String)> {
    let out = Command::new("git").args(args).current_dir(petak).output().await?;
    Ok((
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    ))
}

#[instrument(skip_all, fields(petak = %petak_path.display()))]
pub async fn commit_all_in_petak(petak_path: &Path, message: &str) -> GitResult<CommitSummary> {
    // Fail if nothing changed.
    let (_code, status_out, _) =
        git_in(petak_path, &["status", "--porcelain"]).await?;
    if status_out.trim().is_empty() {
        return Ok(CommitSummary {
            changed: false,
            message: message.to_owned(),
            commit_sha: None,
        });
    }

    let (code, _so, se) = git_in(petak_path, &["add", "-A"]).await?;
    if code != 0 {
        return Err(GitError::Command {
            command: "git add -A".into(),
            status: code,
            stderr: se,
        });
    }

    let (code, _so, se) = git_in(
        petak_path,
        &[
            "-c", "user.email=kulisawit@localhost",
            "-c", "user.name=Kulisawit Mandor",
            "commit",
            "-m", message,
        ],
    )
    .await?;
    if code != 0 {
        return Err(GitError::Command {
            command: "git commit".into(),
            status: code,
            stderr: se,
        });
    }

    let (_, sha, _) = git_in(petak_path, &["rev-parse", "HEAD"]).await?;
    Ok(CommitSummary {
        changed: true,
        message: message.to_owned(),
        commit_sha: Some(sha.trim().to_owned()),
    })
}
```

Register in `lib.rs`:

```rust
pub mod branch;
```

- [ ] **Step 4: Run, commit**

```bash
cargo test -p kulisawit-git --test branch_commit
git add crates/kulisawit-git
git commit -m "feat(git): commit_all_in_petak with clean-tree detection"
```

---

### Task 1.17: Petak queries via `git2-rs`

**Files:**
- Create: `crates/kulisawit-git/src/query.rs`
- Modify: `crates/kulisawit-git/src/lib.rs`
- Create: `crates/kulisawit-git/tests/query.rs`

- [ ] **Step 1: Failing test**

`crates/kulisawit-git/tests/query.rs`:

```rust
use kulisawit_git::query::{head_commit_sha, is_clean};
use std::process::Command;
use tempfile::tempdir;

fn init(dir: &std::path::Path) {
    Command::new("git").args(["init","-b","main"]).current_dir(dir).status().unwrap();
    std::fs::write(dir.join("README.md"), "x").unwrap();
    Command::new("git").args(["-c","user.email=t@t","-c","user.name=t","add","."]).current_dir(dir).status().unwrap();
    Command::new("git").args(["-c","user.email=t@t","-c","user.name=t","commit","-m","init"]).current_dir(dir).status().unwrap();
}

#[test]
fn head_and_clean_on_fresh_repo() {
    let t = tempdir().unwrap();
    init(t.path());
    let sha = head_commit_sha(t.path()).unwrap();
    assert_eq!(sha.len(), 40);
    assert!(is_clean(t.path()).unwrap());
}

#[test]
fn dirty_when_untracked_file_added() {
    let t = tempdir().unwrap();
    init(t.path());
    std::fs::write(t.path().join("new.txt"), "hi").unwrap();
    assert!(!is_clean(t.path()).unwrap());
}
```

- [ ] **Step 2: Run — expect FAIL**

```bash
cargo test -p kulisawit-git --test query
```

- [ ] **Step 3: Implement**

`crates/kulisawit-git/src/query.rs`:

```rust
//! Read-only git queries via libgit2.

use git2::{Repository, StatusOptions};
use std::path::Path;

use crate::error::GitResult;

pub fn head_commit_sha(repo_path: &Path) -> GitResult<String> {
    let repo = Repository::open(repo_path)?;
    let head = repo.head()?.peel_to_commit()?;
    Ok(head.id().to_string())
}

pub fn is_clean(repo_path: &Path) -> GitResult<bool> {
    let repo = Repository::open(repo_path)?;
    let mut opts = StatusOptions::new();
    opts.include_untracked(true).recurse_untracked_dirs(true);
    let statuses = repo.statuses(Some(&mut opts))?;
    Ok(statuses.is_empty())
}
```

Register in `lib.rs`:

```rust
pub mod query;
```

- [ ] **Step 4: Run + commit**

```bash
cargo test -p kulisawit-git --test query
git add crates/kulisawit-git
git commit -m "feat(git): HEAD sha + clean-tree query via libgit2"
```

---

### Task 1.18: Phase 1 green-bar verification + CI config

**Files:**
- Create: `.github/workflows/ci.yml`

- [ ] **Step 1: Full workspace verification**

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
SQLX_OFFLINE=true cargo build --workspace --all-targets
SQLX_OFFLINE=true cargo test --workspace
```

Expected: every command exits 0.

- [ ] **Step 2: CI workflow**

`.github/workflows/ci.yml`:

```yaml
name: CI
on:
  push:
    branches: [main]
  pull_request:

env:
  CARGO_TERM_COLOR: always
  SQLX_OFFLINE: "true"

jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@1.82
        with:
          components: rustfmt, clippy
      - uses: Swatinem/rust-cache@v2
      - run: cargo fmt --check
      - run: cargo clippy --workspace --all-targets -- -D warnings
      - run: cargo build --workspace --all-targets --locked
      - run: cargo test --workspace --locked
```

- [ ] **Step 3: Commit the CI config**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: add clippy/fmt/build/test workflow"
```

- [ ] **Step 4: Tag Phase 1 checkpoint**

```bash
git tag -a phase-1 -m "Phase 1: Foundation complete"
```

Do not push the tag until the user confirms.

---

## Phase 2: Orchestrator core (scope only — request detailed plan before execution)

**Goal:** Given a `TaskId` and a batch size N, run N `MockAgent` attempts concurrently in isolated worktrees, persist every event to SQLite, enforce a global semaphore on concurrent attempts, and support cancellation. The binary gains a `run` subcommand that dispatches and waits for a batch end-to-end, printing a summary.

**NEW CRATE — `kulisawit-orchestrator`** (rationale: `kulisawit-db` already depends on `kulisawit-core`, so placing the orchestrator in `kulisawit-core` would create a dep cycle `-core → -db → -core`. A new crate depends on `-core`, `-db`, `-git`, and `-agent` without cycles. Flagged by end-of-Phase-1 code review.)

Update root `Cargo.toml`:
- `[workspace] members` — add `"crates/kulisawit-orchestrator"`.
- `[workspace.dependencies]` — add `kulisawit-orchestrator = { path = "crates/kulisawit-orchestrator", version = "0.1.0-dev" }`.

**New files:**
- `crates/kulisawit-orchestrator/Cargo.toml` — deps: `kulisawit-core`, `kulisawit-db`, `kulisawit-git`, `kulisawit-agent`, `tokio` (sync+rt+macros), `tracing`, `futures`, `thiserror`.
- `crates/kulisawit-orchestrator/src/lib.rs` — crate root, re-exports.
- `crates/kulisawit-orchestrator/src/error.rs` — `OrchestratorError` thiserror enum wrapping `CoreError`/`DbError`/`GitError`/`AgentError`.
- `crates/kulisawit-orchestrator/src/orchestrator.rs` — public `Orchestrator` struct owning `DbPool`, adapter registry, worktree root, broadcaster.
- `crates/kulisawit-orchestrator/src/dispatch.rs` — `dispatch_batch` function matching PRD §5.7 flow (per-attempt Tokio task: create worktree → run adapter → persist events → commit → update attempt status).
- `crates/kulisawit-orchestrator/src/prompt.rs` — compose prompt from task title/description/linked_files.
- `crates/kulisawit-orchestrator/src/registry.rs` — `Arc<dyn AgentAdapter>` registry indexed by `id()`.
- `crates/kulisawit-orchestrator/src/broadcaster.rs` — `tokio::sync::broadcast` fanout per-attempt.
- `crates/kulisawit-orchestrator/src/config.rs` — project-level runtime settings (max_concurrent_attempts, worktree_retention_days). Loaded from `peta-kebun.toml` (keeping the Indonesian config filename for user-facing branding).
- `crates/kulisawit-cli/src/commands/run.rs` — one-shot CLI subcommand that dispatches a batch.

**Phase 2 kickoff cleanup (first commits, per Phase 1 review):**
- Add concurrent-insert tests on `attempt` and `events` (gap 4.7a/b from review).
- Add `running → failed` / `queued → cancelled` transition tests (gap 4.7c).
- Add worktree error-path tests: create over existing path, create with invalid base_ref (gap 4.7d).
- Extend `MockAgent` with `MockAgent::failing()` / `MockAgent::cancelling()` constructors emitting terminal `Failed`/`Cancelled` events (review M-8).
- Unify timestamp units: convert `events.timestamp` from milliseconds to seconds, or convert all other `*_at` columns to milliseconds (review M-5). Recommendation: milliseconds everywhere, as SSE timelines will need sub-second resolution.
- Refactor `VerificationStatus` to use an `as_str()` / `TryFrom<&str>` pair mirroring `AttemptStatus`, then drop the duplicated match in `attempt::set_verification` (review 4.2).
- Add crate-root re-exports for `AgentAdapter`, `AgentEvent`, `AgentError`, `RunContext`, `CheckResult` in `kulisawit-core/src/lib.rs` (review 4.1).

**Acceptance:**
- Integration test: plant a tandan of 3 MockKuli buah on a fixture repo; assert every buah has ≥5 events, all reach `Completed` status, each petak gets one commit.
- Cancellation test: cancel mid-run, assert buah terminal status = `Cancelled`, petak still removable.
- Semaphore test: cap at 2 concurrent; plant 4 buah, assert at most 2 are `Running` at any observation.

---

## Phase 3: Server + embedded UI (scope only — request detailed plan before execution)

**Goal:** `kulisawit start` boots the Axum daemon on `127.0.0.1:<port>`, opens the user's browser to a working kanban UI. User can create a kebun, create/drag lahan across columns, open a lahan detail, plant a tandan, and watch live SSE-driven buah output.

**New files (backend):**
- `crates/kulisawit-server/src/app.rs` — Axum router composition, static-file fallback.
- `crates/kulisawit-server/src/assets.rs` — `rust-embed` of `ui/dist/`.
- `crates/kulisawit-server/src/routes/{kebun,lahan,buah,stream}.rs` — REST + SSE handlers.
- `crates/kulisawit-server/src/state.rs` — `AppState { pool, orchestrator }`.
- `crates/kulisawit-server/src/error.rs` — `ApiError` → `IntoResponse` mapping.
- `crates/kulisawit-cli/src/commands/{start,stop,status}.rs`.

**New files (frontend, in `ui/`):**
- `ui/package.json`, `ui/vite.config.ts`, `ui/tsconfig.json`, `ui/tailwind.config.ts`, `ui/postcss.config.js`, `ui/index.html`
- `ui/src/main.tsx`, `ui/src/App.tsx`, router setup
- `ui/src/api/client.ts` — typed fetch wrappers; generates from OpenAPI or handwritten
- `ui/src/features/kebun/*` — kebun list + create modal
- `ui/src/features/kanban/*` — columns, lahan cards, drag-drop (`@dnd-kit/core`)
- `ui/src/features/lahan-detail/*` — tabs, live stream pane
- `ui/src/features/stream/useBuahStream.ts` — EventSource hook
- `ui/src/components/ui/*` — shadcn/ui primitives
- `ui/src/styles/globals.css` — Tailwind entry; palette from PRD §6.5

**Build integration:**
- `crates/kulisawit-server/build.rs` — fails build if `ui/dist/` missing in `--release`; logs warning (not error) in dev.
- Workspace-level `justfile` or `xtask` to run `cd ui && pnpm build` then `cargo build --release`.

**Acceptance:**
- Playwright smoke test: start daemon on random port, create kebun, create lahan, plant 1 MockKuli buah, observe SSE updates in DOM, buah transitions to Completed.
- Manual QA: dark mode default, live output auto-scrolls, tool calls render as collapsible blocks.

---

## Phase 4: Claude Code adapter + tandan UX (scope only — request detailed plan before execution)

**Goal:** Replace `MockKuli` as the default with a real `ClaudeCodeKuli` adapter. Side-by-side tandan comparison view works for real runs.

**New files:**
- `crates/kulisawit-kuli/src/claude_code/{mod.rs,parser.rs,process.rs}` — spawn `claude` CLI, parse JSON-stream output into `KuliEvent`s.
- `crates/kulisawit-kuli/src/claude_code/check.rs` — detect `claude` binary on PATH, report version.
- `crates/kulisawit-kuli/tests/claude_code_parser.rs` — parser unit tests against recorded fixtures in `crates/kulisawit-kuli/tests/fixtures/*.jsonl`.
- `ui/src/features/tandan/CompareView.tsx`, `ui/src/features/tandan/FileTabs.tsx` — grid of buah panels with file-tab sync.
- `ui/src/features/tandan/useTandan.ts` — TanStack Query hook merging lahan + all its buah.

**Acceptance:**
- Live integration test (manual, gated by `CLAUDE_CODE_CLI` env): tandan of 2 real Claude Code buah on a fixture repo runs to completion with distinct diffs.
- Compare view: 2-column grid, each buah shows diff, sortir placeholder, "Panen this" button.

---

## Phase 5: Sortir + Panen + CLI + config (scope only — request detailed plan before execution)

**Goal:** Production-complete v0.1 MVP.

**New files:**
- `crates/kulisawit-core/src/sortir/{mod.rs,runner.rs}` — parse `peta-kebun.toml` sortir section, spawn commands in petak after buah completion, capture output, update `buah.sortir_status`.
- `crates/kulisawit-git/src/panen.rs` — fast-forward merge where possible, else merge commit; optional `gh pr create`.
- `crates/kulisawit-git/src/bersihkan.rs` — GC petak older than `petak_retention_days`.
- `crates/kulisawit-core/src/config.rs` — full `peta-kebun.toml` schema with serde.
- `crates/kulisawit-cli/src/commands/{init,tanam,panen,bersihkan,dokter,telemetry}.rs`.
- `crates/kulisawit-cli/src/cli.rs` — `clap` derive with all PRD §7.2 subcommands.
- Binary symlink installation helper so `ksw` aliases `kulisawit` (handled by `cargo-dist` post-install script in Phase 6).

**Acceptance:**
- Full MVP user stories US1–US8 executable end to end.
- Sortir failure is clearly surfaced in UI with command output.
- Panen produces a fast-forward where possible, else a merge commit with `kulisawit: panen buah <short-id>` message.
- `kulisawit dokter` exits non-zero when any configured kuli `check()` fails.

---

## Phase 6: Release pipeline + launch (scope only — request detailed plan before execution)

**Goal:** Public v0.1 on GitHub Releases with prebuilt binaries for macOS (arm64/x64), Linux (x64/arm64), Windows (x64). Install via `cargo install`, Homebrew, shell installer, `cargo binstall`.

**New files:**
- `.github/workflows/release.yml` — generated by `cargo dist init`.
- `dist-workspace.toml` — `cargo-dist` config.
- `README.md` — install instructions, screenshot, 30-second usage demo.
- `CHANGELOG.md` — Keep a Changelog format.
- `LICENSE-MIT`, `LICENSE-APACHE`.
- `SECURITY.md`, `CONTRIBUTING.md`.
- `assets/logo.svg` — plantation-themed identity, palette from PRD §6.5.
- `install.sh` — wrapper over `cargo dist` install script hosted at `kulisawit.dev/install.sh`.

**Deliverables:**
- 60–90 second demo video (tandan of 3 buah racing, diff comparison, sortir passing, panen).
- Launch tweet (English + Bahasa), HN submission, `r/rust` post, This Week in Rust submission, dev.to writeup.

**Acceptance:**
- `cargo install kulisawit` works from a fresh machine.
- `curl -sSf https://kulisawit.dev/install.sh | sh` works on macOS + Linux.
- First-run path: `kulisawit init && kulisawit start` opens browser to the UI in under 2 seconds on an M2 MacBook.

---

## Self-Review Checklist (completed during plan writing)

**Spec coverage (PRD §3.1 MVP features):**
- F1 Kebun init — Phase 5 (`init` command), schema ready in Task 1.7
- F2 Kanban board + columns — schema Task 1.10, UI Phase 3
- F3 Lahan CRUD — DB Task 1.11, API Phase 3, UI Phase 3
- F4 Single buah execution — Phase 2 orchestrator, backed by Tasks 1.12–1.15
- F5 Live streaming to UI — Phase 3 SSE
- F6 Tandan of N buah — Phase 2 orchestrator + Phase 3 UI
- F7 Side-by-side diff — Phase 4
- F8 Claude Code adapter — Phase 4
- F9 Sortir hooks — Phase 5
- F10 Panen merge/PR — Phase 5
- F11 SQLite persistence — Tasks 1.7–1.13
- F12 Single-binary + embedded UI — Phase 3 (`rust-embed`), Phase 6 (`cargo-dist`)

**Placeholder scan:** No `TBD`, `implement later`, or hand-wave steps in Phase 1 tasks. Each step has either exact code, exact command with expected output, or exact commit message.

**Type consistency:** `BuahStatus::Queued|Running|Completed|Failed|Cancelled` used identically in `crates/kulisawit-core/src/status.rs` (Task 1.4), migration CHECK constraint (Task 1.7), `kulisawit-db/src/buah.rs` (Task 1.12). `KuliEvent` variants consistent across core definition (Task 1.6), MockKuli emission (Task 1.14), and event-log type names (Task 1.13). `LahanId`/`KebunId`/`BuahId`/`ColumnId` constructors uniform (Task 1.3).

**Known risk:** `sqlx::migrate!("../../migrations")` path is relative from the compiled crate. If this proves brittle in release builds, switch to `sqlx::migrate!()` with `SQLX_OFFLINE` metadata and a migrations dir symlinked inside `kulisawit-db/`. Flagged for Phase 2 acceptance testing.

---

## Execution Handoff

**Plan complete and saved to `docs/superpowers/plans/2026-04-18-kulisawit-implementation.md`.**

**Two execution options:**

**1. Subagent-Driven (recommended)** — I dispatch a fresh subagent per task, review between tasks, fast iteration.

**2. Inline Execution** — I execute tasks in this session using the `executing-plans` skill, batch execution with checkpoints.

**Which approach?**
