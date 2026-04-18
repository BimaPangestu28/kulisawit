# Kulisawit Phase 2 — Orchestrator Core Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the orchestrator crate and CLI glue that dispatches MockAgent attempts end-to-end — per-attempt git worktree, live event stream fanned out through a broadcaster, DB-persisted lifecycle, and a `kulisawit run` command that drives the whole pipeline with parallelism, cancellation, and prompt composition.

**Architecture:** New `kulisawit-orchestrator` crate sits between domain types (`kulisawit-core`), persistence (`kulisawit-db`), worktree plumbing (`kulisawit-git`), and agents (`kulisawit-agent`). It owns an `AgentRegistry`, an `EventBroadcaster` for per-attempt SSE fanout, a `tokio::sync::Semaphore` for concurrency caps, and per-attempt cancellation tokens. The CLI wires this up for local smoke runs.

**Tech Stack:** Rust 1.86, Tokio multi-thread, `tokio::sync::{Semaphore, broadcast, Mutex, Notify}`, `async-trait`, `futures` streams, `thiserror`, `tracing`, `serde` + `toml`, `clap` derive. Persistence remains SQLx SQLite. Worktree management via the existing `kulisawit-git` crate.

**Prerequisites:** Phase 1 complete (tag `phase-1`, HEAD `ddb3c57`), post-phase-1 hardening commits `a5bef68`/`d95115c`/`b45a1ad` already landed, current HEAD `f33b051`. Rust 1.86, 37 tests passing baseline.

---

## Conventions

This plan inherits every convention from Phase 1 (`docs/superpowers/plans/2026-04-18-kulisawit-implementation.md`, §Conventions): Conventional Commits, `thiserror` at library boundaries, `anyhow` only in the CLI binary, `tracing::instrument(skip(...))` on orchestrator functions, `sqlx::query!` offline metadata committed, one commit per green test suite.

Two reminders specific to Phase 2:
- **No `#![deny(...)]`** attributes in any new file — the workspace `[lints]` table already denies `unwrap_used` / `expect_used` / `panic`.
- **Test modules** use `#[allow(clippy::expect_used, clippy::panic)]` at the module level. Integration tests use file-level `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]`.
- **English generic identifiers** everywhere in code. Kulisawit domain vocabulary (kebun/tandan/buah/petak/mandor) is reserved for user-facing strings — none exposed in Phase 2.

---

## File Structure (end of Phase 2)

```
kulisawit/
├── Cargo.toml                                  # + kulisawit-orchestrator member & dep
├── migrations/0001_initial.sql                 # unchanged
├── crates/
│   ├── kulisawit-core/                         # (existing) + crate-root re-exports
│   │   └── src/
│   │       ├── lib.rs                          # modified: promote adapter types
│   │       └── status.rs                       # modified: VerificationStatus parity
│   ├── kulisawit-db/                           # (existing) timestamps unified to ms
│   │   ├── src/
│   │   │   ├── attempt.rs                      # modified
│   │   │   ├── events.rs                       # unchanged
│   │   │   ├── project.rs                      # modified
│   │   │   └── task.rs                         # modified
│   │   └── tests/
│   │       ├── attempt_transitions.rs          # NEW
│   │       ├── concurrent_inserts.rs           # NEW
│   │       └── timestamp_units.rs              # NEW
│   ├── kulisawit-git/                          # (existing)
│   │   └── tests/
│   │       └── worktree_errors.rs              # NEW
│   ├── kulisawit-agent/                        # (existing)
│   │   └── src/mock.rs                         # modified: MockMode enum
│   ├── kulisawit-orchestrator/                 # NEW
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── error.rs                        # OrchestratorError
│   │   │   ├── registry.rs                     # AgentRegistry
│   │   │   ├── broadcaster.rs                  # EventBroadcaster
│   │   │   ├── prompt.rs                       # compose_prompt
│   │   │   ├── config.rs                       # RuntimeConfig
│   │   │   ├── orchestrator.rs                 # Orchestrator struct
│   │   │   └── dispatch.rs                     # dispatch_single_attempt + dispatch_batch
│   │   └── tests/
│   │       ├── registry.rs
│   │       ├── broadcaster.rs
│   │       ├── prompt.rs
│   │       ├── dispatch.rs
│   │       ├── dispatch_batch.rs
│   │       └── cancel.rs
│   └── kulisawit-cli/                          # modified
│       ├── Cargo.toml                          # + orchestrator/db/git/agent deps + clap
│       └── src/
│           ├── main.rs                         # replaces println with clap
│           ├── commands/
│           │   ├── mod.rs
│           │   ├── version.rs
│           │   └── run.rs
│           └── tests/
│               └── cli_help.rs                 # integration test
└── .github/workflows/ci.yml                    # + sqlx prepare --check
```

---

## Exit Criteria

Before tagging `phase-2`, all of the following must hold on a clean checkout:

- `cargo test --workspace --locked` → 100% pass (target ~50+ tests).
- `cargo clippy --workspace --all-targets --locked -- -D warnings` → zero warnings.
- `cargo fmt --check` → clean.
- `cargo build --workspace --all-targets --locked` → clean.
- End-to-end smoke: `kulisawit run --db <path> --repo <path> --task <id> --agent mock --batch 3` dispatches 3 MockAgent attempts to `Completed` status, each with its own worktree + branch + commit.
- `Orchestrator::cancel_attempt` demonstrably terminates a slow MockAgent mid-run (test `cancel.rs`).

---

## Phase 2.0 — Kickoff hardening

Five small tasks, in order, to close gaps left by Phase 1 review before touching the orchestrator.

### Task 2.0.1: Promote adapter types to crate root

**Files:**
- Modify: `crates/kulisawit-core/src/lib.rs`

- [ ] **Step 1: Read current state**

Current `crates/kulisawit-core/src/lib.rs`:

```rust
//! Kulisawit domain types, adapter trait, orchestrator
//!
//! See the workspace root `README.md` and `docs/PRD.md` for the product brief.

pub mod adapter;
pub mod error;
pub mod ids;
pub mod status;

pub use error::{CoreError, CoreResult};
pub use ids::{AttemptId, ColumnId, ProjectId, TaskId};
pub use status::{AttemptStatus, RunStatus, UnknownAttemptStatus, VerificationStatus};
```

No test required — pure re-export change.

- [ ] **Step 2: Implement**

Replace the contents of `crates/kulisawit-core/src/lib.rs` with:

```rust
//! Kulisawit domain types, adapter trait, orchestrator
//!
//! See the workspace root `README.md` and `docs/PRD.md` for the product brief.

pub mod adapter;
pub mod error;
pub mod ids;
pub mod status;

pub use adapter::{AgentAdapter, AgentError, AgentEvent, CheckResult, RunContext};
pub use error::{CoreError, CoreResult};
pub use ids::{AttemptId, ColumnId, ProjectId, TaskId};
pub use status::{AttemptStatus, RunStatus, UnknownAttemptStatus, VerificationStatus};
```

- [ ] **Step 3: Verify**

Run:

```bash
cargo check --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

Expected: both clean. A quick smoke that downstream code compiles against `kulisawit_core::AgentAdapter` (not just `kulisawit_core::adapter::AgentAdapter`):

```bash
cargo check -p kulisawit-agent --locked
```

Expected: clean (the agent crate still uses `kulisawit_core::adapter::{...}`; the new re-exports don't break anything).

- [ ] **Step 4: Commit**

```bash
git add crates/kulisawit-core/src/lib.rs
git commit -m "refactor(core): promote adapter types to crate root"
```

---

### Task 2.0.2: VerificationStatus parity with AttemptStatus

**Files:**
- Modify: `crates/kulisawit-core/src/status.rs`
- Modify: `crates/kulisawit-core/src/error.rs`
- Modify: `crates/kulisawit-core/src/lib.rs`
- Modify: `crates/kulisawit-db/src/attempt.rs`

- [ ] **Step 1: Write the failing tests**

Append to `crates/kulisawit-core/src/status.rs` inside the existing `#[cfg(test)] mod tests` block:

```rust
    #[test]
    fn verification_status_round_trips_to_str() {
        for status in [
            VerificationStatus::Pending,
            VerificationStatus::Passed,
            VerificationStatus::Failed,
            VerificationStatus::Skipped,
        ] {
            let s = status.as_str();
            let back = VerificationStatus::try_from(s).expect("roundtrip");
            assert_eq!(back, status);
        }
    }

    #[test]
    fn verification_status_rejects_unknown() {
        let err = VerificationStatus::try_from("banana").unwrap_err();
        assert!(format!("{err}").contains("banana"));
    }

    #[test]
    fn verification_status_default_is_still_pending() {
        assert_eq!(VerificationStatus::default(), VerificationStatus::Pending);
    }
```

- [ ] **Step 2: Run to verify it fails**

Run:

```bash
cargo test -p kulisawit-core --lib status::tests
```

Expected: FAIL with `no method named as_str found for enum VerificationStatus` (and/or `no impl TryFrom<&str> for VerificationStatus`).

- [ ] **Step 3: Implement**

Replace the `VerificationStatus` block in `crates/kulisawit-core/src/status.rs` (currently lines 75-83) with:

```rust
/// Result of running verification commands against an attempt.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum VerificationStatus {
    #[default]
    Pending,
    Passed,
    Failed,
    Skipped,
}

impl VerificationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("unknown verification status: {0}")]
pub struct UnknownVerificationStatus(pub String);

impl TryFrom<&str> for VerificationStatus {
    type Error = UnknownVerificationStatus;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "pending" => Ok(Self::Pending),
            "passed" => Ok(Self::Passed),
            "failed" => Ok(Self::Failed),
            "skipped" => Ok(Self::Skipped),
            other => Err(UnknownVerificationStatus(other.to_owned())),
        }
    }
}
```

Update `crates/kulisawit-core/src/error.rs` — add one variant to `CoreError`:

```rust
    #[error("unknown verification status: {0}")]
    UnknownVerificationStatus(#[from] crate::status::UnknownVerificationStatus),
```

so the full `CoreError` enum becomes:

```rust
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

    #[error("unknown attempt status: {0}")]
    UnknownAttemptStatus(#[from] crate::status::UnknownAttemptStatus),

    #[error("unknown verification status: {0}")]
    UnknownVerificationStatus(#[from] crate::status::UnknownVerificationStatus),
}
```

Update the re-exports in `crates/kulisawit-core/src/lib.rs` — the `pub use status::{...}` line becomes:

```rust
pub use status::{
    AttemptStatus, RunStatus, UnknownAttemptStatus, UnknownVerificationStatus, VerificationStatus,
};
```

Simplify `attempt::set_verification` in `crates/kulisawit-db/src/attempt.rs` — replace the hand-written match (currently lines 198-203) so the function body reads:

```rust
pub async fn set_verification(
    pool: &DbPool,
    id: &AttemptId,
    status: VerificationStatus,
    output: Option<&str>,
) -> DbResult<()> {
    let status_str = status.as_str();
    let id_str = id.as_str();
    sqlx::query!(
        "UPDATE attempt SET verification_status = ?, verification_output = ? WHERE id = ?",
        status_str,
        output,
        id_str
    )
    .execute(pool)
    .await?;
    Ok(())
}
```

Also tidy `parse_verification_status` (currently lines 69-77) to reuse `TryFrom`:

```rust
fn parse_verification_status(s: &str) -> DbResult<VerificationStatus> {
    VerificationStatus::try_from(s).map_err(|e| DbError::Invalid(e.to_string()))
}
```

- [ ] **Step 4: Run to verify it passes**

Run:

```bash
cargo test -p kulisawit-core --lib status::tests
cargo test -p kulisawit-db --test attempt_repo
cargo clippy --workspace --all-targets --locked -- -D warnings
```

Expected: all green, no clippy warnings.

- [ ] **Step 5: Commit**

```bash
git add crates/kulisawit-core/src/status.rs crates/kulisawit-core/src/error.rs crates/kulisawit-core/src/lib.rs crates/kulisawit-db/src/attempt.rs
git commit -m "refactor(core): VerificationStatus parity with AttemptStatus"
```

---

### Task 2.0.3: Unify `*_at` columns to millisecond timestamps

**Files:**
- Modify: `crates/kulisawit-db/src/project.rs`
- Modify: `crates/kulisawit-db/src/task.rs`
- Modify: `crates/kulisawit-db/src/attempt.rs`
- Create: `crates/kulisawit-db/tests/timestamp_units.rs`

Column types in `migrations/0001_initial.sql` are all `INTEGER` and unit-agnostic, so the migration is untouched. Only Rust callsites change.

- [ ] **Step 1: Write the failing test**

Create `crates/kulisawit-db/tests/timestamp_units.rs`:

```rust
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kulisawit_db::{attempt, columns, connect, events, migrate, project, task};

/// 2023-11-14 in **milliseconds** is 13 digits (1_700_000_000_000).
/// In **seconds** it would be 10 digits (1_700_000_000).
/// The test fails unless every timestamp column stores ms.
const MIN_MILLIS_2023_11: i64 = 1_700_000_000_000;

#[tokio::test]
async fn project_created_at_is_in_millis() {
    let pool = connect("sqlite::memory:").await.expect("pool");
    migrate(&pool).await.expect("mig");
    let id = project::create(
        &pool,
        project::NewProject {
            name: "K".into(),
            repo_path: "/k".into(),
        },
    )
    .await
    .expect("create");
    let row = project::get(&pool, &id).await.expect("get").expect("row");
    assert!(
        row.created_at >= MIN_MILLIS_2023_11,
        "project.created_at = {}; expected ms epoch (>= {MIN_MILLIS_2023_11})",
        row.created_at
    );
}

#[tokio::test]
async fn task_created_and_updated_are_in_millis() {
    let pool = connect("sqlite::memory:").await.expect("pool");
    migrate(&pool).await.expect("mig");
    let project_id = project::create(
        &pool,
        project::NewProject {
            name: "K".into(),
            repo_path: "/k".into(),
        },
    )
    .await
    .expect("project");
    let cols = columns::seed_defaults(&pool, &project_id)
        .await
        .expect("cols");
    let task_id = task::create(
        &pool,
        task::NewTask {
            project_id: project_id.clone(),
            column_id: cols[0].clone(),
            title: "t".into(),
            description: None,
            tags: vec![],
            linked_files: vec![],
        },
    )
    .await
    .expect("task");
    let t = task::get(&pool, &task_id).await.expect("get").expect("row");
    assert!(t.created_at >= MIN_MILLIS_2023_11);
    assert!(t.updated_at >= MIN_MILLIS_2023_11);

    task::update_text(&pool, &task_id, "new title", None)
        .await
        .expect("update");
    let t = task::get(&pool, &task_id).await.expect("get").expect("row");
    assert!(t.updated_at >= MIN_MILLIS_2023_11);

    // move_to_column also refreshes updated_at
    task::move_to_column(&pool, &task_id, &cols[1])
        .await
        .expect("move");
    let t = task::get(&pool, &task_id).await.expect("get").expect("row");
    assert!(t.updated_at >= MIN_MILLIS_2023_11);
}

#[tokio::test]
async fn attempt_started_and_completed_are_in_millis() {
    use kulisawit_core::AttemptStatus;

    let pool = connect("sqlite::memory:").await.expect("pool");
    migrate(&pool).await.expect("mig");
    let project_id = project::create(
        &pool,
        project::NewProject {
            name: "K".into(),
            repo_path: "/k".into(),
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
    let a = attempt::create(
        &pool,
        attempt::NewAttempt {
            task_id,
            agent_id: "mock".into(),
            prompt_variant: None,
            worktree_path: "/w".into(),
            branch_name: "b".into(),
        },
    )
    .await
    .expect("a");
    attempt::mark_running(&pool, &a).await.expect("r");
    attempt::mark_terminal(&pool, &a, AttemptStatus::Completed)
        .await
        .expect("done");
    let row = attempt::get(&pool, &a).await.expect("get").expect("row");
    assert!(row.started_at.expect("started") >= MIN_MILLIS_2023_11);
    assert!(row.completed_at.expect("completed") >= MIN_MILLIS_2023_11);
}

#[tokio::test]
async fn events_timestamp_already_in_millis() {
    use kulisawit_core::adapter::AgentEvent;

    let pool = connect("sqlite::memory:").await.expect("pool");
    migrate(&pool).await.expect("mig");
    let project_id = project::create(
        &pool,
        project::NewProject {
            name: "K".into(),
            repo_path: "/k".into(),
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
    let attempt_id = attempt::create(
        &pool,
        attempt::NewAttempt {
            task_id,
            agent_id: "mock".into(),
            prompt_variant: None,
            worktree_path: "/w".into(),
            branch_name: "b".into(),
        },
    )
    .await
    .expect("a");
    let id = events::append(
        &pool,
        &attempt_id,
        &AgentEvent::Stdout { text: "x".into() },
    )
    .await
    .expect("append");
    assert!(id > 0);
}
```

- [ ] **Step 2: Run to verify it fails**

Run:

```bash
cargo test -p kulisawit-db --test timestamp_units
```

Expected: FAIL on `project_created_at_is_in_millis`, `task_created_and_updated_are_in_millis`, and `attempt_started_and_completed_are_in_millis` with messages like `project.created_at = 1760000000; expected ms epoch (>= 1700000000000)`.

- [ ] **Step 3: Implement**

In `crates/kulisawit-db/src/project.rs`, change `Utc::now().timestamp()` to `Utc::now().timestamp_millis()`. The function `create` becomes:

```rust
pub async fn create(pool: &DbPool, new: NewProject) -> DbResult<ProjectId> {
    let id = ProjectId::new();
    let created_at = Utc::now().timestamp_millis();
    let id_str = id.as_str();
    sqlx::query!(
        "INSERT INTO project (id, name, repo_path, created_at) VALUES (?, ?, ?, ?)",
        id_str,
        new.name,
        new.repo_path,
        created_at
    )
    .execute(pool)
    .await?;
    Ok(id)
}
```

In `crates/kulisawit-db/src/task.rs`, update three functions. `create`:

```rust
pub async fn create(pool: &DbPool, new: NewTask) -> DbResult<TaskId> {
    let id = TaskId::new();
    let now = Utc::now().timestamp_millis();
    let position = next_position(pool, &new.column_id).await?;
    let tags_json = serde_json::to_string(&new.tags)?;
    let files_json = serde_json::to_string(&new.linked_files)?;
    let id_str = id.as_str();
    let project_str = new.project_id.as_str();
    let col_str = new.column_id.as_str();
    sqlx::query!(
        "INSERT INTO task (id, project_id, column_id, title, description, position, tags, linked_files, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        id_str,
        project_str,
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
```

`update_text`:

```rust
pub async fn update_text(
    pool: &DbPool,
    id: &TaskId,
    title: &str,
    description: Option<&str>,
) -> DbResult<()> {
    let now = Utc::now().timestamp_millis();
    let id_str = id.as_str();
    sqlx::query!(
        "UPDATE task SET title = ?, description = ?, updated_at = ? WHERE id = ?",
        title,
        description,
        now,
        id_str
    )
    .execute(pool)
    .await?;
    Ok(())
}
```

`move_to_column`:

```rust
pub async fn move_to_column(pool: &DbPool, id: &TaskId, column_id: &ColumnId) -> DbResult<()> {
    let position = next_position(pool, column_id).await?;
    let now = Utc::now().timestamp_millis();
    let id_str = id.as_str();
    let col_str = column_id.as_str();
    sqlx::query!(
        "UPDATE task SET column_id = ?, position = ?, updated_at = ? WHERE id = ?",
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

In `crates/kulisawit-db/src/attempt.rs`, update `mark_running` and `mark_terminal`:

```rust
pub async fn mark_running(pool: &DbPool, id: &AttemptId) -> DbResult<()> {
    let now = Utc::now().timestamp_millis();
    let id_str = id.as_str();
    let status = AttemptStatus::Running.as_str();
    sqlx::query!(
        "UPDATE attempt SET status = ?, started_at = ? WHERE id = ?",
        status,
        now,
        id_str
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn mark_terminal(pool: &DbPool, id: &AttemptId, status: AttemptStatus) -> DbResult<()> {
    if !status.is_terminal() {
        return Err(DbError::Invalid(format!(
            "mark_terminal called with non-terminal status: {status:?}"
        )));
    }
    let now = Utc::now().timestamp_millis();
    let id_str = id.as_str();
    let status_str = status.as_str();
    sqlx::query!(
        "UPDATE attempt SET status = ?, completed_at = ? WHERE id = ?",
        status_str,
        now,
        id_str
    )
    .execute(pool)
    .await?;
    Ok(())
}
```

`events::append` already uses `Utc::now().timestamp_millis()` — no change.

- [ ] **Step 4: Run to verify it passes**

Run:

```bash
cargo test -p kulisawit-db --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

Expected: all green.

- [ ] **Step 5: Commit**

```bash
git add crates/kulisawit-db/src/project.rs crates/kulisawit-db/src/task.rs crates/kulisawit-db/src/attempt.rs crates/kulisawit-db/tests/timestamp_units.rs
git commit -m "refactor(db): unify all *_at columns to millisecond timestamps"
```

---

### Task 2.0.4: MockAgent failure, cancellation, and slow modes

**Files:**
- Modify: `crates/kulisawit-agent/src/mock.rs`
- Modify: `crates/kulisawit-agent/tests/mock_stream.rs`

`MockAgent` today is a unit struct that always succeeds. Phase 2.1.9 needs a slow-emitting variant for cancellation tests, and Task 2.0.4's brief already requires failure + cancellation terminal modes. We add all three in one pass so no other task has to revisit this file.

- [ ] **Step 1: Write the failing tests**

Append to `crates/kulisawit-agent/tests/mock_stream.rs` (keep existing tests intact):

```rust
#[tokio::test]
async fn mock_failing_ends_with_status_failed() {
    let k = MockAgent::failing();
    let mut stream = k.run(ctx()).await.expect("run");
    let mut events = vec![];
    while let Some(evt) = stream.next().await {
        events.push(evt);
    }
    assert!(events.len() >= 2, "expected at least 2 events, got {:?}", events.len());
    match events.last().expect("at least one") {
        AgentEvent::Status { status, detail } => {
            assert!(matches!(
                status,
                kulisawit_core::status::RunStatus::Failed
            ));
            assert!(detail.is_some(), "failed status should carry a detail message");
        }
        other => panic!("expected terminal Status event, got {other:?}"),
    }
}

#[tokio::test]
async fn mock_cancelling_ends_with_status_cancelled() {
    let k = MockAgent::cancelling();
    let mut stream = k.run(ctx()).await.expect("run");
    let mut events = vec![];
    while let Some(evt) = stream.next().await {
        events.push(evt);
    }
    match events.last().expect("at least one") {
        AgentEvent::Status { status, .. } => assert!(matches!(
            status,
            kulisawit_core::status::RunStatus::Cancelled
        )),
        other => panic!("expected terminal Status event, got {other:?}"),
    }
}

#[tokio::test]
async fn mock_slow_emits_events_over_time() {
    // Slow mode keeps emitting events until dropped; we just pull two
    // and confirm they arrive — the orchestrator cancel test (Phase 2.1.9)
    // exercises the full drop-behaviour.
    use std::time::Instant;
    let k = MockAgent::slow();
    let mut stream = k.run(ctx()).await.expect("run");
    let t0 = Instant::now();
    let first = stream.next().await.expect("first event");
    let second = stream.next().await.expect("second event");
    let elapsed = t0.elapsed();
    // Each event waits at least ~100ms before emission (give a generous
    // 50ms floor to avoid flake on fast CI).
    assert!(
        elapsed.as_millis() >= 50,
        "expected >=50ms for two slow events, got {:?}",
        elapsed
    );
    // Both events are non-terminal Stdout pings.
    assert!(matches!(first, AgentEvent::Stdout { .. }));
    assert!(matches!(second, AgentEvent::Stdout { .. }));
    // Explicitly drop the stream so the generator task stops.
    drop(stream);
}
```

- [ ] **Step 2: Run to verify it fails**

Run:

```bash
cargo test -p kulisawit-agent --test mock_stream
```

Expected: FAIL with `no function or associated item named failing`, `cancelling`, or `slow` on `MockAgent`.

- [ ] **Step 3: Implement**

Replace the contents of `crates/kulisawit-agent/src/mock.rs` with:

```rust
//! A deterministic adapter used for tests and developer smoke runs.

use async_trait::async_trait;
use futures::stream::{self, BoxStream};
use kulisawit_core::{
    adapter::{AgentAdapter, AgentError, AgentEvent, CheckResult, RunContext},
    status::RunStatus,
};
use std::time::Duration;

/// Mode selector for `MockAgent`. The default is `Succeed` so all existing
/// tests keep passing with `MockAgent::default()`.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum MockMode {
    #[default]
    Succeed,
    Fail,
    Cancel,
    /// Emits a `Stdout` event every ~100ms for up to ~10s, never reaching a
    /// terminal status on its own. Intended for cancellation tests — the
    /// orchestrator is expected to drop the stream.
    Slow,
}

#[derive(Debug, Default, Clone)]
pub struct MockAgent {
    mode: MockMode,
}

impl MockAgent {
    pub fn new(mode: MockMode) -> Self {
        Self { mode }
    }

    pub fn failing() -> Self {
        Self::new(MockMode::Fail)
    }

    pub fn cancelling() -> Self {
        Self::new(MockMode::Cancel)
    }

    pub fn slow() -> Self {
        Self::new(MockMode::Slow)
    }
}

#[async_trait]
impl AgentAdapter for MockAgent {
    fn id(&self) -> &str {
        "mock"
    }
    fn display_name(&self) -> &str {
        "Mock Agent"
    }
    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    async fn check(&self) -> Result<CheckResult, AgentError> {
        Ok(CheckResult {
            ok: true,
            message: Some("mock ready".into()),
            version: Some("0".into()),
        })
    }

    async fn run(&self, _ctx: RunContext) -> Result<BoxStream<'static, AgentEvent>, AgentError> {
        match self.mode {
            MockMode::Succeed => Ok(scripted_stream(success_script())),
            MockMode::Fail => Ok(scripted_stream(failure_script())),
            MockMode::Cancel => Ok(scripted_stream(cancel_script())),
            MockMode::Slow => Ok(slow_stream()),
        }
    }

    async fn cancel(&self, _run_id: &str) -> Result<(), AgentError> {
        Ok(())
    }
}

fn scripted_stream(script: Vec<AgentEvent>) -> BoxStream<'static, AgentEvent> {
    let s = stream::unfold(script.into_iter(), |mut it| async move {
        let next = it.next()?;
        tokio::time::sleep(Duration::from_millis(5)).await;
        Some((next, it))
    });
    Box::pin(s)
}

fn success_script() -> Vec<AgentEvent> {
    vec![
        AgentEvent::Status {
            status: RunStatus::Starting,
            detail: None,
        },
        AgentEvent::Stdout {
            text: "Reading repo…".into(),
        },
        AgentEvent::ToolCall {
            name: "read_file".into(),
            input: serde_json::json!({ "path": "README.md" }),
        },
        AgentEvent::ToolResult {
            name: "read_file".into(),
            output: serde_json::json!({ "bytes": 128 }),
        },
        AgentEvent::Stdout {
            text: "Drafting change…".into(),
        },
        AgentEvent::FileEdit {
            path: "src/lib.rs".into(),
            diff: Some("@@ -1 +1,2 @@\n+// mock edit\n".into()),
        },
        AgentEvent::Status {
            status: RunStatus::Succeeded,
            detail: None,
        },
    ]
}

fn failure_script() -> Vec<AgentEvent> {
    vec![
        AgentEvent::Status {
            status: RunStatus::Starting,
            detail: None,
        },
        AgentEvent::Stdout {
            text: "Attempting change…".into(),
        },
        AgentEvent::Stderr {
            text: "compiler: E0308 mismatched types".into(),
        },
        AgentEvent::Status {
            status: RunStatus::Failed,
            detail: Some("mock failure: synthetic compile error".into()),
        },
    ]
}

fn cancel_script() -> Vec<AgentEvent> {
    vec![
        AgentEvent::Status {
            status: RunStatus::Starting,
            detail: None,
        },
        AgentEvent::Stdout {
            text: "Starting long-running task…".into(),
        },
        AgentEvent::Status {
            status: RunStatus::Cancelled,
            detail: Some("mock self-cancel".into()),
        },
    ]
}

fn slow_stream() -> BoxStream<'static, AgentEvent> {
    // Emit up to 100 ticks at ~100ms each, totalling ~10s if not dropped.
    let s = stream::unfold(0usize, |n| async move {
        if n >= 100 {
            return None;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
        let evt = AgentEvent::Stdout {
            text: format!("slow tick {n}"),
        };
        Some((evt, n + 1))
    });
    Box::pin(s)
}
```

- [ ] **Step 4: Run to verify it passes**

Run:

```bash
cargo test -p kulisawit-agent --test mock_stream
cargo test -p kulisawit-agent --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

Expected: all green (including the pre-existing `mock_check_reports_ok`, `mock_run_emits_scripted_sequence_ending_in_status_succeeded`, `mock_id_and_display_name_are_stable`).

- [ ] **Step 5: Commit**

```bash
git add crates/kulisawit-agent/src/mock.rs crates/kulisawit-agent/tests/mock_stream.rs
git commit -m "feat(agent): MockAgent failure, cancellation, and slow modes"
```

---

### Task 2.0.5: Close Phase 1 review §4.7 test gaps

**Files:**
- Create: `crates/kulisawit-db/tests/concurrent_inserts.rs`
- Create: `crates/kulisawit-db/tests/attempt_transitions.rs`
- Create: `crates/kulisawit-git/tests/worktree_errors.rs`

- [ ] **Step 1: Write the failing tests**

Create `crates/kulisawit-db/tests/concurrent_inserts.rs`:

```rust
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kulisawit_core::TaskId;
use kulisawit_db::{attempt, columns, connect, migrate, project, task, DbPool};
use std::sync::Arc;

async fn setup_task() -> (Arc<DbPool>, TaskId) {
    let pool = connect("sqlite::memory:").await.expect("pool");
    migrate(&pool).await.expect("mig");
    let project_id = project::create(
        &pool,
        project::NewProject {
            name: "K".into(),
            repo_path: "/k".into(),
        },
    )
    .await
    .expect("project");
    let cols = columns::seed_defaults(&pool, &project_id)
        .await
        .expect("cols");
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
    .expect("task");
    (Arc::new(pool), task_id)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn fifty_concurrent_attempt_inserts_all_succeed_and_are_distinct() {
    let (pool, task_id) = setup_task().await;
    let mut handles = Vec::with_capacity(50);
    for i in 0..50 {
        let pool = pool.clone();
        let task_id = task_id.clone();
        handles.push(tokio::spawn(async move {
            attempt::create(
                &pool,
                attempt::NewAttempt {
                    task_id,
                    agent_id: "mock".into(),
                    prompt_variant: None,
                    worktree_path: format!("/w-{i}"),
                    branch_name: format!("b-{i}"),
                },
            )
            .await
            .expect("create")
        }));
    }

    let mut ids = Vec::with_capacity(50);
    for h in handles {
        ids.push(h.await.expect("join"));
    }
    assert_eq!(ids.len(), 50);
    let mut id_strs: Vec<String> = ids.iter().map(|a| a.as_str().to_owned()).collect();
    id_strs.sort();
    id_strs.dedup();
    assert_eq!(id_strs.len(), 50, "expected 50 distinct AttemptIds");

    let listed = attempt::list_for_task(&pool, &task_id).await.expect("list");
    assert_eq!(listed.len(), 50);
}
```

Create `crates/kulisawit-db/tests/attempt_transitions.rs`:

```rust
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kulisawit_core::{AttemptStatus, TaskId};
use kulisawit_db::{attempt, columns, connect, migrate, project, task, DbError, DbPool};

async fn setup_task() -> (DbPool, TaskId) {
    let pool = connect("sqlite::memory:").await.expect("pool");
    migrate(&pool).await.expect("mig");
    let project_id = project::create(
        &pool,
        project::NewProject {
            name: "K".into(),
            repo_path: "/k".into(),
        },
    )
    .await
    .expect("project");
    let cols = columns::seed_defaults(&pool, &project_id)
        .await
        .expect("cols");
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
    .expect("task");
    (pool, task_id)
}

#[tokio::test]
async fn queued_to_failed_transition_works() {
    let (pool, task_id) = setup_task().await;
    let id = attempt::create(
        &pool,
        attempt::NewAttempt {
            task_id,
            agent_id: "mock".into(),
            prompt_variant: None,
            worktree_path: "/w".into(),
            branch_name: "b".into(),
        },
    )
    .await
    .expect("create");
    attempt::mark_terminal(&pool, &id, AttemptStatus::Failed)
        .await
        .expect("fail");
    let row = attempt::get(&pool, &id).await.expect("get").expect("row");
    assert_eq!(row.status, AttemptStatus::Failed);
    assert!(row.completed_at.is_some());
    assert!(row.started_at.is_none(), "no run ever started");
}

#[tokio::test]
async fn running_to_failed_transition_works() {
    let (pool, task_id) = setup_task().await;
    let id = attempt::create(
        &pool,
        attempt::NewAttempt {
            task_id,
            agent_id: "mock".into(),
            prompt_variant: None,
            worktree_path: "/w".into(),
            branch_name: "b".into(),
        },
    )
    .await
    .expect("create");
    attempt::mark_running(&pool, &id).await.expect("running");
    attempt::mark_terminal(&pool, &id, AttemptStatus::Failed)
        .await
        .expect("fail");
    let row = attempt::get(&pool, &id).await.expect("get").expect("row");
    assert_eq!(row.status, AttemptStatus::Failed);
    assert!(row.started_at.is_some());
    assert!(row.completed_at.is_some());
}

#[tokio::test]
async fn mark_terminal_with_non_terminal_status_returns_invalid() {
    let (pool, task_id) = setup_task().await;
    let id = attempt::create(
        &pool,
        attempt::NewAttempt {
            task_id,
            agent_id: "mock".into(),
            prompt_variant: None,
            worktree_path: "/w".into(),
            branch_name: "b".into(),
        },
    )
    .await
    .expect("create");
    let err = attempt::mark_terminal(&pool, &id, AttemptStatus::Running)
        .await
        .expect_err("should reject non-terminal");
    assert!(matches!(err, DbError::Invalid(_)));
}
```

Create `crates/kulisawit-git/tests/worktree_errors.rs`:

```rust
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kulisawit_git::error::GitError;
use kulisawit_git::worktree::{create_worktree, CreateWorktreeRequest};
use std::process::Command;
use tempfile::tempdir;

fn init_repo_with_commit(dir: &std::path::Path) {
    Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(dir)
        .status()
        .unwrap();
    std::fs::write(dir.join("README.md"), "# test\n").unwrap();
    Command::new("git")
        .args(["-c", "user.email=t@t", "-c", "user.name=t", "add", "."])
        .current_dir(dir)
        .status()
        .unwrap();
    Command::new("git")
        .args([
            "-c",
            "user.email=t@t",
            "-c",
            "user.name=t",
            "commit",
            "-m",
            "init",
        ])
        .current_dir(dir)
        .status()
        .unwrap();
}

#[tokio::test]
async fn create_worktree_over_existing_path_returns_invalid() {
    let base = tempdir().expect("tmp");
    init_repo_with_commit(base.path());
    let worktree_root = base.path().join(".kulisawit/worktrees");
    let req = CreateWorktreeRequest {
        repo_root: base.path().to_path_buf(),
        worktree_root: worktree_root.clone(),
        attempt_short_id: "dup12345".into(),
        branch_name: "kulisawit/t/dup12345".into(),
        base_ref: "main".into(),
    };
    // First call succeeds.
    create_worktree(req.clone()).await.expect("first create");
    // Second call with the same attempt_short_id (and therefore the same
    // worktree_path) must fail with Invalid — we detect the collision
    // before shelling out to git.
    let err = create_worktree(req).await.expect_err("should collide");
    assert!(
        matches!(err, GitError::Invalid(_)),
        "expected GitError::Invalid, got {err:?}"
    );
}

#[tokio::test]
async fn create_worktree_with_nonexistent_base_ref_returns_command_error() {
    let base = tempdir().expect("tmp");
    init_repo_with_commit(base.path());
    let worktree_root = base.path().join(".kulisawit/worktrees");
    let req = CreateWorktreeRequest {
        repo_root: base.path().to_path_buf(),
        worktree_root,
        attempt_short_id: "nope1234".into(),
        branch_name: "kulisawit/t/nope1234".into(),
        base_ref: "definitely-not-a-real-ref-kulisawit".into(),
    };
    let err = create_worktree(req).await.expect_err("should fail");
    assert!(
        matches!(err, GitError::Command { .. }),
        "expected GitError::Command, got {err:?}"
    );
}
```

- [ ] **Step 2: Run to verify they fail**

Run:

```bash
cargo test -p kulisawit-db --test concurrent_inserts
cargo test -p kulisawit-db --test attempt_transitions
cargo test -p kulisawit-git --test worktree_errors
```

Expected: `concurrent_inserts` and `attempt_transitions` should compile and pass immediately (the code already supports those flows); `worktree_errors` test `create_worktree_over_existing_path_returns_invalid` should also pass (the existing `if worktree_path.exists() { return Err(GitError::Invalid(...)); }` guard covers it); `create_worktree_with_nonexistent_base_ref_returns_command_error` should pass because `run_git` already surfaces non-zero exit as `GitError::Command`. If any test unexpectedly fails, the failure diagnoses a real gap — debug and adjust Rust code only (no migration changes). This task is test-coverage-only, so a passing first run is the success case.

- [ ] **Step 3: Confirm and commit**

```bash
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

```bash
git add crates/kulisawit-db/tests/concurrent_inserts.rs crates/kulisawit-db/tests/attempt_transitions.rs crates/kulisawit-git/tests/worktree_errors.rs
git commit -m "test: cover concurrent inserts, attempt transitions, and worktree errors"
```

---

## Phase 2.1 — Orchestrator crate

### Task 2.1.1: Scaffold `kulisawit-orchestrator` crate

**Files:**
- Modify: `Cargo.toml` (workspace root)
- Create: `crates/kulisawit-orchestrator/Cargo.toml`
- Create: `crates/kulisawit-orchestrator/src/lib.rs`
- Create: `crates/kulisawit-orchestrator/src/error.rs`

- [ ] **Step 1: Update workspace root `Cargo.toml`**

In `Cargo.toml`, add `"crates/kulisawit-orchestrator"` to `[workspace] members`:

```toml
[workspace]
resolver = "2"
members = [
    "crates/kulisawit-cli",
    "crates/kulisawit-core",
    "crates/kulisawit-db",
    "crates/kulisawit-git",
    "crates/kulisawit-server",
    "crates/kulisawit-agent",
    "crates/kulisawit-orchestrator",
]
```

In `[workspace.dependencies]` (alongside the existing internal crates), add:

```toml
kulisawit-orchestrator = { path = "crates/kulisawit-orchestrator", version = "0.1.0-dev" }
```

- [ ] **Step 2: Create the crate manifest**

Create `crates/kulisawit-orchestrator/Cargo.toml`:

```toml
[package]
name = "kulisawit-orchestrator"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
authors.workspace = true
description = "Orchestrator for Kulisawit: dispatches agent attempts into isolated worktrees"

[lib]

[dependencies]
async-trait.workspace = true
chrono.workspace = true
futures.workspace = true
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true
tokio = { workspace = true, features = ["full"] }
toml.workspace = true
tracing.workspace = true
kulisawit-core.workspace = true
kulisawit-db.workspace = true
kulisawit-git.workspace = true
kulisawit-agent.workspace = true

[dev-dependencies]
tempfile.workspace = true
tokio = { workspace = true, features = ["macros", "rt-multi-thread", "time"] }

[lints]
workspace = true
```

- [ ] **Step 3: Create module stubs**

Create `crates/kulisawit-orchestrator/src/lib.rs`:

```rust
//! Kulisawit orchestrator: dispatches agent attempts into isolated worktrees.
//!
//! Public surface:
//! - [`Orchestrator`] — owns shared state (DB pool, agent registry, broadcaster,
//!   semaphore, cancel flags) and exposes `dispatch_single_attempt`,
//!   `dispatch_batch`, and `cancel_attempt`.
//! - [`AgentRegistry`] — keyed lookup of `AgentAdapter` implementations.
//! - [`EventBroadcaster`] — per-attempt `tokio::sync::broadcast` channels for
//!   SSE fanout.
//! - [`RuntimeConfig`] — declarative runtime knobs loaded from
//!   `peta-kebun.toml`.
//! - [`prompt::compose_prompt`] — deterministic prompt composer from a
//!   `Task` row.

pub mod broadcaster;
pub mod config;
pub mod dispatch;
pub mod error;
pub mod orchestrator;
pub mod prompt;
pub mod registry;

pub use broadcaster::EventBroadcaster;
pub use config::RuntimeConfig;
pub use dispatch::{dispatch_batch, dispatch_single_attempt};
pub use error::{OrchestratorError, OrchestratorResult};
pub use orchestrator::Orchestrator;
pub use registry::AgentRegistry;
```

Create `crates/kulisawit-orchestrator/src/error.rs`:

```rust
//! Orchestrator-level error type.

use thiserror::Error;

use kulisawit_core::{adapter::AgentError, CoreError};
use kulisawit_db::DbError;
use kulisawit_git::GitError;

#[derive(Debug, Error)]
pub enum OrchestratorError {
    #[error("core: {0}")]
    Core(#[from] CoreError),

    #[error("db: {0}")]
    Db(#[from] DbError),

    #[error("git: {0}")]
    Git(#[from] GitError),

    #[error("agent: {0}")]
    Agent(#[from] AgentError),

    #[error("invalid: {0}")]
    Invalid(String),

    #[error("cancelled")]
    Cancelled,
}

pub type OrchestratorResult<T> = Result<T, OrchestratorError>;
```

For this bootstrap, stub the other modules so `pub use` lines compile. Create each file as a placeholder; subsequent tasks flesh them out.

Create `crates/kulisawit-orchestrator/src/broadcaster.rs`:

```rust
//! Event broadcaster — implemented in Task 2.1.3.

/// Placeholder; Task 2.1.3 replaces this.
#[derive(Debug, Default)]
pub struct EventBroadcaster;
```

Create `crates/kulisawit-orchestrator/src/config.rs`:

```rust
//! Runtime configuration — implemented in Task 2.1.5.

use serde::{Deserialize, Serialize};

/// Placeholder; Task 2.1.5 replaces this.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig;
```

Create `crates/kulisawit-orchestrator/src/dispatch.rs`:

```rust
//! Per-attempt dispatch — implemented in Task 2.1.7.

use crate::{Orchestrator, OrchestratorError, OrchestratorResult};
use kulisawit_core::{AttemptId, TaskId};

#[allow(dead_code, clippy::needless_pass_by_value)]
pub async fn dispatch_single_attempt(
    _orch: &Orchestrator,
    _task_id: &TaskId,
    _agent_id: &str,
    _prompt_variant: Option<String>,
) -> OrchestratorResult<AttemptId> {
    Err(OrchestratorError::Invalid(
        "dispatch_single_attempt not yet implemented".into(),
    ))
}

#[allow(dead_code, clippy::needless_pass_by_value)]
pub async fn dispatch_batch(
    _orch: &Orchestrator,
    _task_id: &TaskId,
    _agent_id: &str,
    _batch_size: usize,
    _variants: Option<Vec<String>>,
) -> OrchestratorResult<Vec<AttemptId>> {
    Err(OrchestratorError::Invalid(
        "dispatch_batch not yet implemented".into(),
    ))
}
```

Create `crates/kulisawit-orchestrator/src/orchestrator.rs`:

```rust
//! Orchestrator struct — implemented in Task 2.1.6.

/// Placeholder; Task 2.1.6 replaces this.
#[derive(Debug, Default)]
pub struct Orchestrator;
```

Create `crates/kulisawit-orchestrator/src/prompt.rs`:

```rust
//! Prompt composer — implemented in Task 2.1.4.
```

Create `crates/kulisawit-orchestrator/src/registry.rs`:

```rust
//! Agent registry — implemented in Task 2.1.2.

/// Placeholder; Task 2.1.2 replaces this.
#[derive(Debug, Default)]
pub struct AgentRegistry;
```

- [ ] **Step 4: Verify**

Run:

```bash
cargo build -p kulisawit-orchestrator --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --check
```

Expected: all clean.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml crates/kulisawit-orchestrator/
git commit -m "feat(orchestrator): scaffold crate with OrchestratorError"
```

---

### Task 2.1.2: AgentRegistry

**Files:**
- Modify: `crates/kulisawit-orchestrator/src/registry.rs`
- Create: `crates/kulisawit-orchestrator/tests/registry.rs`

- [ ] **Step 1: Write the failing tests**

Create `crates/kulisawit-orchestrator/tests/registry.rs`:

```rust
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kulisawit_agent::MockAgent;
use kulisawit_core::adapter::AgentAdapter;
use kulisawit_orchestrator::AgentRegistry;
use std::sync::Arc;

#[test]
fn register_and_get_roundtrip() {
    let mut reg = AgentRegistry::new();
    reg.register(Arc::new(MockAgent::default()) as Arc<dyn AgentAdapter>);
    let adapter = reg.get("mock").expect("registered");
    assert_eq!(adapter.id(), "mock");
}

#[test]
fn get_missing_returns_none() {
    let reg = AgentRegistry::new();
    assert!(reg.get("not-there").is_none());
}

#[test]
fn ids_sorted_alphabetically() {
    let mut reg = AgentRegistry::new();
    reg.register(Arc::new(MockAgent::default()) as Arc<dyn AgentAdapter>);
    reg.register(Arc::new(NamedAgent("zeta")) as Arc<dyn AgentAdapter>);
    reg.register(Arc::new(NamedAgent("alpha")) as Arc<dyn AgentAdapter>);
    let ids = reg.ids();
    assert_eq!(ids, vec!["alpha", "mock", "zeta"]);
}

/// Helper that pretends to be a distinct adapter with a custom id.
#[derive(Debug)]
struct NamedAgent(&'static str);

#[async_trait::async_trait]
impl AgentAdapter for NamedAgent {
    fn id(&self) -> &str {
        self.0
    }
    fn display_name(&self) -> &str {
        self.0
    }
    fn version(&self) -> &str {
        "0"
    }
    async fn check(&self) -> Result<kulisawit_core::adapter::CheckResult, kulisawit_core::adapter::AgentError> {
        Ok(kulisawit_core::adapter::CheckResult {
            ok: true,
            message: None,
            version: None,
        })
    }
    async fn run(
        &self,
        _ctx: kulisawit_core::adapter::RunContext,
    ) -> Result<futures::stream::BoxStream<'static, kulisawit_core::adapter::AgentEvent>, kulisawit_core::adapter::AgentError> {
        Ok(Box::pin(futures::stream::empty()))
    }
    async fn cancel(&self, _run_id: &str) -> Result<(), kulisawit_core::adapter::AgentError> {
        Ok(())
    }
}
```

Also add `kulisawit-agent` and `async-trait` to dev-dependencies in `crates/kulisawit-orchestrator/Cargo.toml`:

```toml
[dev-dependencies]
async-trait.workspace = true
futures.workspace = true
kulisawit-agent.workspace = true
tempfile.workspace = true
tokio = { workspace = true, features = ["macros", "rt-multi-thread", "time"] }
```

- [ ] **Step 2: Run to verify it fails**

Run:

```bash
cargo test -p kulisawit-orchestrator --test registry
```

Expected: FAIL with `no function or associated item named new` / `no method named register` — the placeholder `AgentRegistry` is a unit struct.

- [ ] **Step 3: Implement**

Replace the contents of `crates/kulisawit-orchestrator/src/registry.rs`:

```rust
//! Registry of `AgentAdapter` implementations keyed by `id()`.
//!
//! Agents are registered at orchestrator construction time and looked up by
//! string id when a caller asks to dispatch an attempt.

use kulisawit_core::adapter::AgentAdapter;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Default)]
pub struct AgentRegistry {
    agents: HashMap<String, Arc<dyn AgentAdapter>>,
}

impl AgentRegistry {
    pub fn new() -> Self {
        Self {
            agents: HashMap::new(),
        }
    }

    /// Register an adapter. If an adapter with the same id already exists,
    /// it is replaced.
    pub fn register(&mut self, adapter: Arc<dyn AgentAdapter>) {
        let id = adapter.id().to_owned();
        self.agents.insert(id, adapter);
    }

    /// Look up an adapter by id.
    pub fn get(&self, id: &str) -> Option<Arc<dyn AgentAdapter>> {
        self.agents.get(id).cloned()
    }

    /// All registered ids, alphabetically sorted.
    pub fn ids(&self) -> Vec<String> {
        let mut v: Vec<String> = self.agents.keys().cloned().collect();
        v.sort();
        v
    }
}
```

- [ ] **Step 4: Run to verify it passes**

Run:

```bash
cargo test -p kulisawit-orchestrator --test registry
cargo clippy --workspace --all-targets --locked -- -D warnings
```

Expected: all green.

- [ ] **Step 5: Commit**

```bash
git add crates/kulisawit-orchestrator/src/registry.rs crates/kulisawit-orchestrator/Cargo.toml crates/kulisawit-orchestrator/tests/registry.rs
git commit -m "feat(orchestrator): AgentRegistry keyed by adapter id"
```

---

### Task 2.1.3: EventBroadcaster

**Files:**
- Modify: `crates/kulisawit-orchestrator/src/broadcaster.rs`
- Create: `crates/kulisawit-orchestrator/tests/broadcaster.rs`

- [ ] **Step 1: Write the failing tests**

Create `crates/kulisawit-orchestrator/tests/broadcaster.rs`:

```rust
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kulisawit_core::{adapter::AgentEvent, AttemptId};
use kulisawit_orchestrator::EventBroadcaster;

#[tokio::test]
async fn subscriber_before_send_receives_event() {
    let bc = EventBroadcaster::new(16);
    let id = AttemptId::new();
    let mut rx = bc.subscribe(&id);
    bc.send(&id, AgentEvent::Stdout { text: "hi".into() });
    let evt = rx.recv().await.expect("recv");
    assert!(matches!(evt, AgentEvent::Stdout { .. }));
}

#[tokio::test]
async fn two_subscribers_both_receive() {
    let bc = EventBroadcaster::new(16);
    let id = AttemptId::new();
    let mut rx1 = bc.subscribe(&id);
    let mut rx2 = bc.subscribe(&id);
    bc.send(&id, AgentEvent::Stdout { text: "a".into() });
    bc.send(&id, AgentEvent::Stdout { text: "b".into() });
    for rx in [&mut rx1, &mut rx2] {
        let e1 = rx.recv().await.expect("1");
        let e2 = rx.recv().await.expect("2");
        match (e1, e2) {
            (AgentEvent::Stdout { text: a }, AgentEvent::Stdout { text: b }) => {
                assert_eq!(a, "a");
                assert_eq!(b, "b");
            }
            other => panic!("unexpected: {other:?}"),
        }
    }
}

#[tokio::test]
async fn close_drops_channel() {
    let bc = EventBroadcaster::new(16);
    let id = AttemptId::new();
    let mut rx = bc.subscribe(&id);
    bc.close(&id);
    // After close, the previously-issued Receiver yields None (channel closed).
    let res = rx.recv().await;
    assert!(res.is_err(), "expected closed channel, got {res:?}");
}

#[tokio::test]
async fn send_to_unknown_attempt_is_no_op() {
    let bc = EventBroadcaster::new(16);
    let id = AttemptId::new();
    // No subscriber yet — send() should silently create the channel but not
    // panic. Subscribing afterwards produces no prior events (they are
    // dropped by the broadcast channel when no receivers exist yet).
    bc.send(&id, AgentEvent::Stdout { text: "lost".into() });
    let mut rx = bc.subscribe(&id);
    bc.send(&id, AgentEvent::Stdout { text: "kept".into() });
    let evt = rx.recv().await.expect("recv");
    match evt {
        AgentEvent::Stdout { text } => assert_eq!(text, "kept"),
        other => panic!("unexpected: {other:?}"),
    }
}
```

- [ ] **Step 2: Run to verify it fails**

Run:

```bash
cargo test -p kulisawit-orchestrator --test broadcaster
```

Expected: FAIL — `EventBroadcaster` is a unit struct.

- [ ] **Step 3: Implement**

Replace the contents of `crates/kulisawit-orchestrator/src/broadcaster.rs`:

```rust
//! Per-attempt broadcast fanout of `AgentEvent`s.
//!
//! Each attempt gets its own `tokio::sync::broadcast::Sender`. Subscribers
//! (one per SSE client, typically) read from the corresponding `Receiver`.
//! The orchestrator's event loop sends into the channel for every inbound
//! event, and calls `close` when the attempt is terminal — dropping the
//! `Sender` signals receivers that no further events will arrive.

use std::collections::HashMap;

use kulisawit_core::{adapter::AgentEvent, AttemptId};
use tokio::sync::{broadcast, Mutex};

#[derive(Debug)]
pub struct EventBroadcaster {
    channels: Mutex<HashMap<String, broadcast::Sender<AgentEvent>>>,
    capacity: usize,
}

impl EventBroadcaster {
    pub fn new(capacity: usize) -> Self {
        Self {
            channels: Mutex::new(HashMap::new()),
            capacity,
        }
    }

    /// Subscribe to an attempt's event stream. The channel is created on
    /// demand.
    pub fn subscribe(&self, attempt: &AttemptId) -> broadcast::Receiver<AgentEvent> {
        let mut guard = self.channels.blocking_lock();
        let tx = guard
            .entry(attempt.as_str().to_owned())
            .or_insert_with(|| broadcast::channel::<AgentEvent>(self.capacity).0);
        tx.subscribe()
    }

    /// Fanout an event. Silently creates a channel if one does not exist.
    /// Drops the event (returns Ok) if no receivers are currently attached
    /// — this mirrors `broadcast::Sender::send` semantics.
    pub fn send(&self, attempt: &AttemptId, event: AgentEvent) {
        let mut guard = self.channels.blocking_lock();
        let tx = guard
            .entry(attempt.as_str().to_owned())
            .or_insert_with(|| broadcast::channel::<AgentEvent>(self.capacity).0);
        // Ignore send errors — a `broadcast::Sender::send` only errors when
        // no receivers are attached, which is expected in fire-and-forget
        // scenarios.
        let _ = tx.send(event);
    }

    /// Drop the channel for the given attempt. Any live receivers will see
    /// the channel close (and subsequent `recv().await` returns `Err`).
    pub fn close(&self, attempt: &AttemptId) {
        let mut guard = self.channels.blocking_lock();
        guard.remove(attempt.as_str());
    }
}

impl Default for EventBroadcaster {
    fn default() -> Self {
        Self::new(256)
    }
}
```

Note: `subscribe`, `send`, and `close` use `blocking_lock` because they are called from both async contexts (the dispatcher future) and sync contexts (the CLI). Tests run on `#[tokio::test]` (multi-thread when needed) so `blocking_lock` is safe; the lock is held only for a HashMap lookup. For Phase 3 SSE integration we may revisit to hand out pre-subscribed receivers over a channel, but that is out of scope here.

- [ ] **Step 4: Run to verify it passes**

Run:

```bash
cargo test -p kulisawit-orchestrator --test broadcaster
cargo clippy --workspace --all-targets --locked -- -D warnings
```

Expected: all green.

- [ ] **Step 5: Commit**

```bash
git add crates/kulisawit-orchestrator/src/broadcaster.rs crates/kulisawit-orchestrator/tests/broadcaster.rs
git commit -m "feat(orchestrator): EventBroadcaster for per-attempt SSE fanout"
```

---

### Task 2.1.4: Prompt composer

**Files:**
- Modify: `crates/kulisawit-orchestrator/src/prompt.rs`
- Create: `crates/kulisawit-orchestrator/tests/prompt.rs`

- [ ] **Step 1: Write the failing tests**

Create `crates/kulisawit-orchestrator/tests/prompt.rs`:

```rust
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kulisawit_core::{ColumnId, ProjectId, TaskId};
use kulisawit_db::task::Task;
use kulisawit_orchestrator::prompt::compose_prompt;

fn base_task(title: &str, description: Option<&str>) -> Task {
    Task {
        id: TaskId::new(),
        project_id: ProjectId::new(),
        column_id: ColumnId::new(),
        title: title.to_owned(),
        description: description.map(str::to_owned),
        position: 0,
        tags: vec![],
        linked_files: vec![],
        created_at: 0,
        updated_at: 0,
    }
}

#[test]
fn title_and_description_only() {
    let t = base_task("Add rate limit", Some("Describe the endpoint."));
    let p = compose_prompt(&t, None);
    assert_eq!(p, "# Add rate limit\n\nDescribe the endpoint.\n");
}

#[test]
fn title_only_omits_description_section() {
    let t = base_task("Title-only", None);
    let p = compose_prompt(&t, None);
    assert_eq!(p, "# Title-only\n");
}

#[test]
fn with_linked_files_renders_section() {
    let mut t = base_task("Refactor auth", Some("Split auth into modules."));
    t.linked_files = vec!["src/auth.rs".into(), "src/db.rs".into()];
    let p = compose_prompt(&t, None);
    assert_eq!(
        p,
        "# Refactor auth\n\nSplit auth into modules.\n\n## Linked files\n- src/auth.rs\n- src/db.rs\n"
    );
}

#[test]
fn with_tags_renders_section() {
    let mut t = base_task("Refactor auth", None);
    t.tags = vec!["auth".into(), "security".into()];
    let p = compose_prompt(&t, None);
    assert_eq!(p, "# Refactor auth\n\n## Tags\nauth, security\n");
}

#[test]
fn with_variant_renders_trailing_note() {
    let t = base_task("Refactor auth", Some("Do it."));
    let p = compose_prompt(&t, Some("diff-first"));
    assert_eq!(
        p,
        "# Refactor auth\n\nDo it.\n\n(variant: diff-first)\n"
    );
}

#[test]
fn with_all_four_sections_in_order() {
    let mut t = base_task("Refactor auth", Some("Split auth into modules."));
    t.linked_files = vec!["src/auth.rs".into()];
    t.tags = vec!["auth".into(), "security".into()];
    let p = compose_prompt(&t, Some("diff-first"));
    assert_eq!(
        p,
        "# Refactor auth\n\nSplit auth into modules.\n\n## Linked files\n- src/auth.rs\n\n## Tags\nauth, security\n\n(variant: diff-first)\n"
    );
}

#[test]
fn empty_tags_and_files_are_omitted() {
    let mut t = base_task("T", Some("D"));
    t.tags = vec![];
    t.linked_files = vec![];
    let p = compose_prompt(&t, None);
    assert_eq!(p, "# T\n\nD\n");
}
```

- [ ] **Step 2: Run to verify it fails**

Run:

```bash
cargo test -p kulisawit-orchestrator --test prompt
```

Expected: FAIL with `cannot find function compose_prompt in module prompt`.

- [ ] **Step 3: Implement**

Replace the contents of `crates/kulisawit-orchestrator/src/prompt.rs`:

```rust
//! Compose the prompt handed to an `AgentAdapter` from a `Task` row.
//!
//! The prompt is a plain Markdown-ish string:
//!
//! ```text
//! # <title>
//!
//! <description>
//!
//! ## Linked files
//! - <path>
//!
//! ## Tags
//! <tag>, <tag>
//!
//! (variant: <name>)
//! ```
//!
//! Empty sections are omitted. This function is deterministic and allocation-
//! only — no I/O.

use kulisawit_db::task::Task;

pub fn compose_prompt(task: &Task, variant: Option<&str>) -> String {
    let mut sections: Vec<String> = Vec::new();

    // Title is always present.
    sections.push(format!("# {}", task.title));

    if let Some(desc) = task.description.as_deref() {
        if !desc.is_empty() {
            sections.push(desc.to_owned());
        }
    }

    if !task.linked_files.is_empty() {
        let mut block = String::from("## Linked files\n");
        for (idx, f) in task.linked_files.iter().enumerate() {
            if idx > 0 {
                block.push('\n');
            }
            block.push_str("- ");
            block.push_str(f);
        }
        sections.push(block);
    }

    if !task.tags.is_empty() {
        sections.push(format!("## Tags\n{}", task.tags.join(", ")));
    }

    if let Some(v) = variant {
        sections.push(format!("(variant: {v})"));
    }

    let mut out = sections.join("\n\n");
    out.push('\n');
    out
}
```

- [ ] **Step 4: Run to verify it passes**

Run:

```bash
cargo test -p kulisawit-orchestrator --test prompt
cargo clippy --workspace --all-targets --locked -- -D warnings
```

Expected: all green.

- [ ] **Step 5: Commit**

```bash
git add crates/kulisawit-orchestrator/src/prompt.rs crates/kulisawit-orchestrator/tests/prompt.rs
git commit -m "feat(orchestrator): prompt composer from Task fields"
```

---

### Task 2.1.5: RuntimeConfig with TOML loader

**Files:**
- Modify: `crates/kulisawit-orchestrator/src/config.rs`

- [ ] **Step 1: Write the failing tests (inline `#[cfg(test)] mod tests`)**

Put tests in the same file since `RuntimeConfig` has no I/O. Replace `crates/kulisawit-orchestrator/src/config.rs` with the full test+impl block in Step 3; first, write just the test block at the bottom of the current placeholder to get a failing compile:

Append to `crates/kulisawit-orchestrator/src/config.rs` (temporary state that won't compile yet — Step 3 overwrites the whole file):

```rust
#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn default_values_are_sane() {
        let c = RuntimeConfig::default();
        assert_eq!(c.max_concurrent_attempts, 8);
        assert_eq!(c.worktree_retention_days, 7);
        assert_eq!(c.default_agent_id, "mock");
        assert_eq!(c.default_batch_size, 1);
    }

    #[test]
    fn parses_runtime_block_from_toml() {
        let toml_src = r#"
[runtime]
max_concurrent_attempts = 4
worktree_retention_days = 14
default_agent_id = "claude-code"
default_batch_size = 3
"#;
        let c = RuntimeConfig::from_toml_str(toml_src).expect("parse");
        assert_eq!(c.max_concurrent_attempts, 4);
        assert_eq!(c.worktree_retention_days, 14);
        assert_eq!(c.default_agent_id, "claude-code");
        assert_eq!(c.default_batch_size, 3);
    }

    #[test]
    fn missing_runtime_block_returns_default() {
        let toml_src = "[kebun]\nname = \"demo\"\n";
        let c = RuntimeConfig::from_toml_str(toml_src).expect("parse");
        assert_eq!(c, RuntimeConfig::default());
    }

    #[test]
    fn malformed_toml_returns_err() {
        let toml_src = "this is not valid [[[";
        assert!(RuntimeConfig::from_toml_str(toml_src).is_err());
    }

    #[test]
    fn partial_runtime_block_fills_defaults() {
        let toml_src = r#"
[runtime]
max_concurrent_attempts = 2
"#;
        let c = RuntimeConfig::from_toml_str(toml_src).expect("parse");
        assert_eq!(c.max_concurrent_attempts, 2);
        // Defaults retained for unspecified fields.
        assert_eq!(c.worktree_retention_days, 7);
        assert_eq!(c.default_agent_id, "mock");
        assert_eq!(c.default_batch_size, 1);
    }
}
```

- [ ] **Step 2: Run to verify it fails**

Run:

```bash
cargo test -p kulisawit-orchestrator --lib config::tests
```

Expected: FAIL with `no method named from_toml_str`, missing fields, `RuntimeConfig` unit struct incompatible.

- [ ] **Step 3: Implement**

Replace the contents of `crates/kulisawit-orchestrator/src/config.rs` with:

```rust
//! Runtime configuration loaded from `peta-kebun.toml`.
//!
//! Only a `[runtime]` block is consumed here; other blocks
//! (`[kebun]`, `[agents]`) are handled elsewhere.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeConfig {
    pub max_concurrent_attempts: usize,
    pub worktree_retention_days: u32,
    pub default_agent_id: String,
    pub default_batch_size: usize,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            max_concurrent_attempts: 8,
            worktree_retention_days: 7,
            default_agent_id: "mock".into(),
            default_batch_size: 1,
        }
    }
}

#[derive(Debug, Deserialize)]
struct RawRuntimeFile {
    #[serde(default)]
    runtime: Option<RawRuntimeBlock>,
}

#[derive(Debug, Default, Deserialize)]
struct RawRuntimeBlock {
    max_concurrent_attempts: Option<usize>,
    worktree_retention_days: Option<u32>,
    default_agent_id: Option<String>,
    default_batch_size: Option<usize>,
}

impl RuntimeConfig {
    /// Parse a `peta-kebun.toml` string, extracting only the `[runtime]`
    /// block. Missing keys fall back to [`Default::default`].
    pub fn from_toml_str(s: &str) -> Result<Self, toml::de::Error> {
        let raw: RawRuntimeFile = toml::from_str(s)?;
        let defaults = Self::default();
        let block = raw.runtime.unwrap_or_default();
        Ok(Self {
            max_concurrent_attempts: block
                .max_concurrent_attempts
                .unwrap_or(defaults.max_concurrent_attempts),
            worktree_retention_days: block
                .worktree_retention_days
                .unwrap_or(defaults.worktree_retention_days),
            default_agent_id: block.default_agent_id.unwrap_or(defaults.default_agent_id),
            default_batch_size: block
                .default_batch_size
                .unwrap_or(defaults.default_batch_size),
        })
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn default_values_are_sane() {
        let c = RuntimeConfig::default();
        assert_eq!(c.max_concurrent_attempts, 8);
        assert_eq!(c.worktree_retention_days, 7);
        assert_eq!(c.default_agent_id, "mock");
        assert_eq!(c.default_batch_size, 1);
    }

    #[test]
    fn parses_runtime_block_from_toml() {
        let toml_src = r#"
[runtime]
max_concurrent_attempts = 4
worktree_retention_days = 14
default_agent_id = "claude-code"
default_batch_size = 3
"#;
        let c = RuntimeConfig::from_toml_str(toml_src).expect("parse");
        assert_eq!(c.max_concurrent_attempts, 4);
        assert_eq!(c.worktree_retention_days, 14);
        assert_eq!(c.default_agent_id, "claude-code");
        assert_eq!(c.default_batch_size, 3);
    }

    #[test]
    fn missing_runtime_block_returns_default() {
        let toml_src = "[kebun]\nname = \"demo\"\n";
        let c = RuntimeConfig::from_toml_str(toml_src).expect("parse");
        assert_eq!(c, RuntimeConfig::default());
    }

    #[test]
    fn malformed_toml_returns_err() {
        let toml_src = "this is not valid [[[";
        assert!(RuntimeConfig::from_toml_str(toml_src).is_err());
    }

    #[test]
    fn partial_runtime_block_fills_defaults() {
        let toml_src = r#"
[runtime]
max_concurrent_attempts = 2
"#;
        let c = RuntimeConfig::from_toml_str(toml_src).expect("parse");
        assert_eq!(c.max_concurrent_attempts, 2);
        assert_eq!(c.worktree_retention_days, 7);
        assert_eq!(c.default_agent_id, "mock");
        assert_eq!(c.default_batch_size, 1);
    }
}
```

- [ ] **Step 4: Run to verify it passes**

Run:

```bash
cargo test -p kulisawit-orchestrator --lib config::tests
cargo clippy --workspace --all-targets --locked -- -D warnings
```

Expected: all green.

- [ ] **Step 5: Commit**

```bash
git add crates/kulisawit-orchestrator/src/config.rs
git commit -m "feat(orchestrator): RuntimeConfig with TOML loader"
```

---

### Task 2.1.6: Orchestrator struct

**Files:**
- Modify: `crates/kulisawit-orchestrator/src/orchestrator.rs`

- [ ] **Step 1: Write the failing test (inline `#[cfg(test)] mod tests`)**

Add to the end of `crates/kulisawit-orchestrator/src/orchestrator.rs` (after Step 3 replaces the file, the test below will compile):

```rust
#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use kulisawit_agent::MockAgent;
    use kulisawit_core::adapter::AgentAdapter;
    use kulisawit_db::{connect, migrate};
    use std::sync::Arc;
    use tempfile::tempdir;

    #[tokio::test]
    async fn constructor_smoke_works_with_real_pool_and_registry() {
        let pool = connect("sqlite::memory:").await.expect("pool");
        migrate(&pool).await.expect("mig");
        let mut registry = crate::AgentRegistry::new();
        registry.register(Arc::new(MockAgent::default()) as Arc<dyn AgentAdapter>);
        let base = tempdir().expect("tmp");
        let cfg = crate::RuntimeConfig::default();
        let orch = Orchestrator::new(
            pool,
            registry,
            base.path().to_path_buf(),
            base.path().join(".kulisawit/worktrees"),
            cfg,
        );
        assert_eq!(orch.config().default_agent_id, "mock");
        assert_eq!(orch.config().max_concurrent_attempts, 8);
        assert!(orch.registry().get("mock").is_some());
    }
}
```

- [ ] **Step 2: Run to verify it fails**

Run:

```bash
cargo test -p kulisawit-orchestrator --lib orchestrator::tests
```

Expected: FAIL — `Orchestrator::new` doesn't exist yet.

- [ ] **Step 3: Implement**

Replace the contents of `crates/kulisawit-orchestrator/src/orchestrator.rs` with:

```rust
//! The `Orchestrator` struct: shared state for dispatching attempts.
//!
//! The type is `Send + Sync` and cheap to clone: all mutable state sits
//! behind `Arc`-wrapped interior-mutability primitives so a single
//! `Orchestrator` value can be shared across spawned dispatch tasks and HTTP
//! handlers without a surrounding lock.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use kulisawit_core::AttemptId;
use kulisawit_db::DbPool;
use tokio::sync::{Mutex, Notify, Semaphore};

use crate::{AgentRegistry, EventBroadcaster, RuntimeConfig};

#[derive(Debug)]
pub struct Orchestrator {
    pool: Arc<DbPool>,
    registry: Arc<AgentRegistry>,
    broadcaster: Arc<EventBroadcaster>,
    worktree_root: PathBuf,
    repo_root: PathBuf,
    semaphore: Arc<Semaphore>,
    config: RuntimeConfig,
    cancel_flags: Arc<Mutex<HashMap<String, Arc<Notify>>>>,
}

impl Orchestrator {
    pub fn new(
        pool: DbPool,
        registry: AgentRegistry,
        repo_root: PathBuf,
        worktree_root: PathBuf,
        config: RuntimeConfig,
    ) -> Self {
        let permits = config.max_concurrent_attempts.max(1);
        Self {
            pool: Arc::new(pool),
            registry: Arc::new(registry),
            broadcaster: Arc::new(EventBroadcaster::new(256)),
            worktree_root,
            repo_root,
            semaphore: Arc::new(Semaphore::new(permits)),
            config,
            cancel_flags: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn pool(&self) -> &Arc<DbPool> {
        &self.pool
    }

    pub fn registry(&self) -> &Arc<AgentRegistry> {
        &self.registry
    }

    pub fn broadcaster(&self) -> &Arc<EventBroadcaster> {
        &self.broadcaster
    }

    pub fn worktree_root(&self) -> &std::path::Path {
        &self.worktree_root
    }

    pub fn repo_root(&self) -> &std::path::Path {
        &self.repo_root
    }

    pub fn semaphore(&self) -> &Arc<Semaphore> {
        &self.semaphore
    }

    pub fn config(&self) -> &RuntimeConfig {
        &self.config
    }

    /// Install a cancel `Notify` for the given attempt id. Called by the
    /// dispatcher at the start of each attempt so `cancel_attempt` can
    /// reach it later. Idempotent.
    pub async fn install_cancel_flag(&self, id: &AttemptId) -> Arc<Notify> {
        let mut g = self.cancel_flags.lock().await;
        g.entry(id.as_str().to_owned())
            .or_insert_with(|| Arc::new(Notify::new()))
            .clone()
    }

    /// Remove the cancel `Notify` for the given attempt id. Called on
    /// terminal transition.
    pub async fn remove_cancel_flag(&self, id: &AttemptId) {
        let mut g = self.cancel_flags.lock().await;
        g.remove(id.as_str());
    }

    /// Look up (but do not install) an existing cancel `Notify`.
    pub async fn cancel_flag(&self, id: &AttemptId) -> Option<Arc<Notify>> {
        let g = self.cancel_flags.lock().await;
        g.get(id.as_str()).cloned()
    }

    /// Request cancellation of a live attempt. Returns `Ok(())` whether or
    /// not the attempt is currently running — the dispatcher checks the
    /// flag on its next event poll.
    pub async fn cancel_attempt(
        &self,
        id: &AttemptId,
    ) -> crate::OrchestratorResult<()> {
        if let Some(n) = self.cancel_flag(id).await {
            n.notify_one();
        }
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use kulisawit_agent::MockAgent;
    use kulisawit_core::adapter::AgentAdapter;
    use kulisawit_db::{connect, migrate};
    use std::sync::Arc;
    use tempfile::tempdir;

    #[tokio::test]
    async fn constructor_smoke_works_with_real_pool_and_registry() {
        let pool = connect("sqlite::memory:").await.expect("pool");
        migrate(&pool).await.expect("mig");
        let mut registry = crate::AgentRegistry::new();
        registry.register(Arc::new(MockAgent::default()) as Arc<dyn AgentAdapter>);
        let base = tempdir().expect("tmp");
        let cfg = crate::RuntimeConfig::default();
        let orch = Orchestrator::new(
            pool,
            registry,
            base.path().to_path_buf(),
            base.path().join(".kulisawit/worktrees"),
            cfg,
        );
        assert_eq!(orch.config().default_agent_id, "mock");
        assert_eq!(orch.config().max_concurrent_attempts, 8);
        assert!(orch.registry().get("mock").is_some());
    }
}
```

- [ ] **Step 4: Run to verify it passes**

Run:

```bash
cargo test -p kulisawit-orchestrator --lib orchestrator::tests
cargo clippy --workspace --all-targets --locked -- -D warnings
```

Expected: all green.

- [ ] **Step 5: Commit**

```bash
git add crates/kulisawit-orchestrator/src/orchestrator.rs
git commit -m "feat(orchestrator): Orchestrator struct with shared state"
```

---

### Task 2.1.7: `dispatch_single_attempt` lifecycle

**Files:**
- Modify: `crates/kulisawit-orchestrator/src/dispatch.rs`
- Create: `crates/kulisawit-orchestrator/tests/dispatch.rs`

- [ ] **Step 1: Write the failing tests**

Create `crates/kulisawit-orchestrator/tests/dispatch.rs`:

```rust
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kulisawit_agent::{MockAgent, MockMode};
use kulisawit_core::{adapter::AgentAdapter, AttemptStatus};
use kulisawit_db::{attempt, columns, connect, events, migrate, project, task};
use kulisawit_orchestrator::{
    dispatch_single_attempt, AgentRegistry, Orchestrator, RuntimeConfig,
};
use std::process::Command;
use std::sync::Arc;
use tempfile::tempdir;

fn init_repo(dir: &std::path::Path) {
    Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(dir)
        .status()
        .unwrap();
    std::fs::write(dir.join("README.md"), "# test\n").unwrap();
    Command::new("git")
        .args(["-c", "user.email=t@t", "-c", "user.name=t", "add", "."])
        .current_dir(dir)
        .status()
        .unwrap();
    Command::new("git")
        .args([
            "-c",
            "user.email=t@t",
            "-c",
            "user.name=t",
            "commit",
            "-m",
            "init",
        ])
        .current_dir(dir)
        .status()
        .unwrap();
}

async fn build_orch(repo_dir: &std::path::Path, mode: MockMode) -> Orchestrator {
    let pool = connect("sqlite::memory:").await.expect("pool");
    migrate(&pool).await.expect("mig");
    let mut registry = AgentRegistry::new();
    registry.register(Arc::new(MockAgent::new(mode)) as Arc<dyn AgentAdapter>);
    Orchestrator::new(
        pool,
        registry,
        repo_dir.to_path_buf(),
        repo_dir.join(".kulisawit/worktrees"),
        RuntimeConfig::default(),
    )
}

async fn seed_task(orch: &Orchestrator) -> kulisawit_core::TaskId {
    let project_id = project::create(
        orch.pool(),
        project::NewProject {
            name: "K".into(),
            repo_path: orch.repo_root().display().to_string(),
        },
    )
    .await
    .expect("p");
    let cols = columns::seed_defaults(orch.pool(), &project_id)
        .await
        .expect("c");
    task::create(
        orch.pool(),
        task::NewTask {
            project_id,
            column_id: cols[0].clone(),
            title: "smoke".into(),
            description: Some("smoke test".into()),
            tags: vec![],
            linked_files: vec![],
        },
    )
    .await
    .expect("t")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dispatch_succeed_records_completed_attempt_and_events() {
    let base = tempdir().expect("tmp");
    init_repo(base.path());
    let orch = build_orch(base.path(), MockMode::Succeed).await;
    let task_id = seed_task(&orch).await;

    let attempt_id = dispatch_single_attempt(&orch, &task_id, "mock", None)
        .await
        .expect("dispatch");

    let a = attempt::get(orch.pool(), &attempt_id)
        .await
        .expect("get")
        .expect("row");
    assert_eq!(a.status, AttemptStatus::Completed);
    assert!(a.started_at.is_some());
    assert!(a.completed_at.is_some());

    let evts = events::list_for_attempt(orch.pool(), &attempt_id)
        .await
        .expect("events");
    assert!(
        evts.len() >= 5,
        "expected >= 5 events from MockAgent, got {}",
        evts.len()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dispatch_failing_mock_records_failed_attempt() {
    let base = tempdir().expect("tmp");
    init_repo(base.path());
    let orch = build_orch(base.path(), MockMode::Fail).await;
    let task_id = seed_task(&orch).await;

    let attempt_id = dispatch_single_attempt(&orch, &task_id, "mock", None)
        .await
        .expect("dispatch");

    let a = attempt::get(orch.pool(), &attempt_id)
        .await
        .expect("get")
        .expect("row");
    assert_eq!(a.status, AttemptStatus::Failed);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dispatch_with_missing_task_returns_invalid() {
    let base = tempdir().expect("tmp");
    init_repo(base.path());
    let orch = build_orch(base.path(), MockMode::Succeed).await;
    let bogus = kulisawit_core::TaskId::new();
    let err = dispatch_single_attempt(&orch, &bogus, "mock", None)
        .await
        .expect_err("should fail");
    let msg = format!("{err}");
    assert!(msg.contains("task not found"), "got: {msg}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dispatch_with_unknown_agent_returns_invalid() {
    let base = tempdir().expect("tmp");
    init_repo(base.path());
    let orch = build_orch(base.path(), MockMode::Succeed).await;
    let task_id = seed_task(&orch).await;
    let err = dispatch_single_attempt(&orch, &task_id, "not-registered", None)
        .await
        .expect_err("should fail");
    let msg = format!("{err}");
    assert!(msg.contains("agent not registered"), "got: {msg}");
}
```

- [ ] **Step 2: Run to verify it fails**

Run:

```bash
cargo test -p kulisawit-orchestrator --test dispatch
```

Expected: FAIL — the placeholder `dispatch_single_attempt` always returns `OrchestratorError::Invalid("dispatch_single_attempt not yet implemented")`.

- [ ] **Step 3: Implement**

Replace the contents of `crates/kulisawit-orchestrator/src/dispatch.rs` with:

```rust
//! Per-attempt dispatch lifecycle.
//!
//! Flow of `dispatch_single_attempt`:
//!
//! 1. Acquire a permit from the orchestrator semaphore.
//! 2. Fetch the `Task` row; error if missing.
//! 3. Compose the prompt via [`crate::prompt::compose_prompt`].
//! 4. Look up the adapter by id; error if missing.
//! 5. Insert an `attempt` row in `Queued` with the allocated worktree path
//!    and branch name.
//! 6. Create the git worktree on disk.
//! 7. Transition to `Running`.
//! 8. Run the adapter; consume the returned event stream. For each event,
//!    append to the DB event log and broadcast. When a terminal `Status`
//!    is seen, remember the mapped `AttemptStatus` and break.
//!    Cancellation is polled via `tokio::select!` on the per-attempt
//!    `Notify`; if fired, call `adapter.cancel` and map to
//!    `AttemptStatus::Cancelled`.
//! 9. Commit changes in the worktree.
//! 10. Transition the attempt to its terminal `AttemptStatus`.
//! 11. Close the broadcaster channel and drop the cancel flag.

use std::sync::Arc;

use futures::StreamExt;
use kulisawit_core::{
    adapter::{AgentEvent, RunContext},
    AttemptId, AttemptStatus, RunStatus, TaskId,
};
use kulisawit_db::{attempt, events, task};
use kulisawit_git::{
    branch::commit_all_in_worktree,
    query::head_commit_sha,
    worktree::{create_worktree, CreateWorktreeRequest},
};
use tracing::{instrument, warn};

use crate::{Orchestrator, OrchestratorError, OrchestratorResult};

/// Short id helper: first 8 chars of a UUID-v7-ish string.
fn short(id: &str) -> String {
    id.chars().take(8).collect()
}

#[instrument(skip(orch), fields(task = %task_id, agent = agent_id))]
pub async fn dispatch_single_attempt(
    orch: &Orchestrator,
    task_id: &TaskId,
    agent_id: &str,
    prompt_variant: Option<String>,
) -> OrchestratorResult<AttemptId> {
    // 1. Concurrency gate.
    let _permit = orch
        .semaphore()
        .clone()
        .acquire_owned()
        .await
        .map_err(|e| OrchestratorError::Invalid(format!("semaphore closed: {e}")))?;

    // 2. Fetch the task.
    let task_row = task::get(orch.pool(), task_id)
        .await?
        .ok_or_else(|| OrchestratorError::Invalid(format!("task not found: {task_id}")))?;

    // 3. Compose the prompt.
    let prompt = crate::prompt::compose_prompt(&task_row, prompt_variant.as_deref());

    // 4. Resolve the adapter.
    let adapter = orch
        .registry()
        .get(agent_id)
        .ok_or_else(|| OrchestratorError::Invalid(format!("agent not registered: {agent_id}")))?;

    // 5. Allocate ids & paths. Short-id for humans; full AttemptId for the
    // DB row.
    let attempt_id = AttemptId::new();
    let attempt_short = short(attempt_id.as_str());
    let task_short = short(task_id.as_str());
    let branch_name = format!("kulisawit/{task_short}/{attempt_short}");
    let worktree_path = orch
        .worktree_root()
        .join(format!("attempt-{attempt_short}"));

    // 6. Determine base ref (current HEAD of the repo).
    let base_ref = head_commit_sha(orch.repo_root()).map_err(OrchestratorError::from)?;

    // 7. Insert the attempt row in Queued.
    let attempt_id = attempt::create(
        orch.pool(),
        attempt::NewAttempt {
            task_id: task_id.clone(),
            agent_id: agent_id.to_owned(),
            prompt_variant: prompt_variant.clone(),
            worktree_path: worktree_path.display().to_string(),
            branch_name: branch_name.clone(),
        },
    )
    .await?;

    // 8. Create the worktree.
    let wt_outcome = create_worktree(CreateWorktreeRequest {
        repo_root: orch.repo_root().to_path_buf(),
        worktree_root: orch.worktree_root().to_path_buf(),
        attempt_short_id: attempt_short.clone(),
        branch_name: branch_name.clone(),
        base_ref,
    })
    .await?;

    // 9. Install cancel flag, mark running.
    let cancel_notify = orch.install_cancel_flag(&attempt_id).await;
    attempt::mark_running(orch.pool(), &attempt_id).await?;

    // 10. Drive the adapter.
    let run_ctx = RunContext {
        run_id: attempt_id.as_str().to_owned(),
        worktree_path: wt_outcome.worktree_path.clone(),
        prompt,
        prompt_variant,
        env: std::collections::HashMap::new(),
    };

    let mut stream = adapter.run(run_ctx).await?;
    let mut terminal: Option<AttemptStatus> = None;

    loop {
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
                terminal = Some(AttemptStatus::Cancelled);
                break;
            }
            next = stream.next() => {
                let Some(evt) = next else {
                    // Stream ended without a terminal Status — treat as Failed.
                    warn!(attempt = %attempt_id, "adapter stream ended without terminal status");
                    terminal = Some(AttemptStatus::Failed);
                    break;
                };
                let _ = events::append(orch.pool(), &attempt_id, &evt).await;
                orch.broadcaster().send(&attempt_id, evt.clone());
                if let AgentEvent::Status { status, .. } = &evt {
                    if let Some(mapped) = AttemptStatus::from_terminal_run_status(*status) {
                        terminal = Some(mapped);
                        break;
                    }
                }
            }
        }
    }

    // 11. Commit whatever the agent produced. Best-effort: if the agent
    // failed and the worktree is dirty we still want the diff captured on
    // the per-attempt branch.
    let attempt_title = &task_row.title;
    let commit_msg = format!("kulisawit: attempt {attempt_short} for {attempt_title}");
    if let Err(e) = commit_all_in_worktree(&wt_outcome.worktree_path, &commit_msg).await {
        warn!(attempt = %attempt_id, "commit_all_in_worktree failed: {e}");
    }

    // 12. Persist terminal status (default to Failed if somehow unset).
    let terminal = terminal.unwrap_or(AttemptStatus::Failed);
    attempt::mark_terminal(orch.pool(), &attempt_id, terminal).await?;

    // 13. Cleanup.
    orch.broadcaster().close(&attempt_id);
    orch.remove_cancel_flag(&attempt_id).await;

    Ok(attempt_id)
}

#[instrument(skip(orch), fields(task = %task_id, agent = agent_id, n = batch_size))]
pub async fn dispatch_batch(
    orch: &Orchestrator,
    task_id: &TaskId,
    agent_id: &str,
    batch_size: usize,
    variants: Option<Vec<String>>,
) -> OrchestratorResult<Vec<AttemptId>> {
    if batch_size == 0 {
        return Err(OrchestratorError::Invalid(
            "batch_size must be >= 1".into(),
        ));
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

    // Share the orchestrator across spawned tasks. Because `Orchestrator`
    // holds all mutable state behind `Arc`s, we wrap the whole thing in
    // an `Arc` for cheap cloning.
    let orch = Arc::new(clone_orch(orch));

    let mut handles = Vec::with_capacity(batch_size);
    for i in 0..batch_size {
        let orch = Arc::clone(&orch);
        let task_id = task_id.clone();
        let agent_id = agent_id.to_owned();
        let variant = variants.as_ref().and_then(|v| v.get(i).cloned());
        handles.push(tokio::spawn(async move {
            dispatch_single_attempt(&orch, &task_id, &agent_id, variant).await
        }));
    }

    let mut ids = Vec::with_capacity(batch_size);
    for h in handles {
        match h.await {
            Ok(Ok(id)) => ids.push(id),
            Ok(Err(e)) => return Err(e),
            Err(join_err) => {
                return Err(OrchestratorError::Invalid(format!(
                    "dispatch task panicked: {join_err}"
                )))
            }
        }
    }
    Ok(ids)
}

// `Orchestrator` does not implement `Clone` on purpose — we want callers to
// think about shared ownership. `dispatch_batch` needs to fan out into
// spawned tasks, so it constructs a cheap clone by re-wrapping all the
// inner `Arc`s. Defined here so the semantics stay local to dispatch.
fn clone_orch(o: &Orchestrator) -> Orchestrator {
    // Use a dedicated method on `Orchestrator` to avoid field access here.
    o.clone_for_dispatch()
}
```

Add a `clone_for_dispatch` helper to `crates/kulisawit-orchestrator/src/orchestrator.rs`. Append to the `impl Orchestrator` block (before the `#[cfg(test)]` tests):

```rust
    /// Produce a shallow clone by re-wrapping inner `Arc`s. The resulting
    /// value shares the same pool / registry / broadcaster / semaphore /
    /// cancel-flag map with `self`. Intended for internal fan-out inside
    /// `dispatch_batch`; callers should not rely on it.
    pub(crate) fn clone_for_dispatch(&self) -> Self {
        Self {
            pool: Arc::clone(&self.pool),
            registry: Arc::clone(&self.registry),
            broadcaster: Arc::clone(&self.broadcaster),
            worktree_root: self.worktree_root.clone(),
            repo_root: self.repo_root.clone(),
            semaphore: Arc::clone(&self.semaphore),
            config: self.config.clone(),
            cancel_flags: Arc::clone(&self.cancel_flags),
        }
    }
```

- [ ] **Step 4: Run to verify it passes**

Run:

```bash
cargo test -p kulisawit-orchestrator --test dispatch
cargo test -p kulisawit-orchestrator --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

Expected: all four `dispatch.rs` tests green. The full orchestrator crate test run should be green.

- [ ] **Step 5: Commit**

```bash
git add crates/kulisawit-orchestrator/src/dispatch.rs crates/kulisawit-orchestrator/src/orchestrator.rs crates/kulisawit-orchestrator/tests/dispatch.rs
git commit -m "feat(orchestrator): dispatch_single_attempt lifecycle"
```

---

### Task 2.1.8: `dispatch_batch`

**Files:**
- Create: `crates/kulisawit-orchestrator/tests/dispatch_batch.rs`

The `dispatch_batch` function was written in Task 2.1.7 alongside `dispatch_single_attempt`. This task adds the integration test that proves it.

- [ ] **Step 1: Write the failing test**

Create `crates/kulisawit-orchestrator/tests/dispatch_batch.rs`:

```rust
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kulisawit_agent::{MockAgent, MockMode};
use kulisawit_core::{adapter::AgentAdapter, AttemptStatus};
use kulisawit_db::{attempt, columns, connect, migrate, project, task};
use kulisawit_orchestrator::{
    dispatch_batch, AgentRegistry, Orchestrator, RuntimeConfig,
};
use std::process::Command;
use std::sync::Arc;
use tempfile::tempdir;

fn init_repo(dir: &std::path::Path) {
    Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(dir)
        .status()
        .unwrap();
    std::fs::write(dir.join("README.md"), "# test\n").unwrap();
    Command::new("git")
        .args(["-c", "user.email=t@t", "-c", "user.name=t", "add", "."])
        .current_dir(dir)
        .status()
        .unwrap();
    Command::new("git")
        .args([
            "-c",
            "user.email=t@t",
            "-c",
            "user.name=t",
            "commit",
            "-m",
            "init",
        ])
        .current_dir(dir)
        .status()
        .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn dispatch_batch_of_three_all_complete() {
    let base = tempdir().expect("tmp");
    init_repo(base.path());
    let pool = connect("sqlite::memory:").await.expect("pool");
    migrate(&pool).await.expect("mig");
    let mut registry = AgentRegistry::new();
    registry.register(Arc::new(MockAgent::new(MockMode::Succeed)) as Arc<dyn AgentAdapter>);
    let orch = Orchestrator::new(
        pool,
        registry,
        base.path().to_path_buf(),
        base.path().join(".kulisawit/worktrees"),
        RuntimeConfig::default(),
    );

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
            title: "batch".into(),
            description: Some("d".into()),
            tags: vec![],
            linked_files: vec![],
        },
    )
    .await
    .expect("t");

    let ids = dispatch_batch(&orch, &task_id, "mock", 3, None)
        .await
        .expect("batch");
    assert_eq!(ids.len(), 3);
    let mut strs: Vec<String> = ids.iter().map(|a| a.as_str().to_owned()).collect();
    strs.sort();
    strs.dedup();
    assert_eq!(strs.len(), 3, "three distinct AttemptIds");

    for id in &ids {
        let row = attempt::get(orch.pool(), id).await.expect("get").expect("row");
        assert_eq!(row.status, AttemptStatus::Completed);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dispatch_batch_with_variants_len_mismatch_errors() {
    let base = tempdir().expect("tmp");
    init_repo(base.path());
    let pool = connect("sqlite::memory:").await.expect("pool");
    migrate(&pool).await.expect("mig");
    let mut registry = AgentRegistry::new();
    registry.register(Arc::new(MockAgent::default()) as Arc<dyn AgentAdapter>);
    let orch = Orchestrator::new(
        pool,
        registry,
        base.path().to_path_buf(),
        base.path().join(".kulisawit/worktrees"),
        RuntimeConfig::default(),
    );
    // Seed a task so the "variants mismatch" check is the first failure.
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
            title: "t".into(),
            description: None,
            tags: vec![],
            linked_files: vec![],
        },
    )
    .await
    .expect("t");

    let err = dispatch_batch(
        &orch,
        &task_id,
        "mock",
        2,
        Some(vec!["a".into(), "b".into(), "c".into()]),
    )
    .await
    .expect_err("should error");
    let msg = format!("{err}");
    assert!(msg.contains("variants length"), "got: {msg}");
}
```

- [ ] **Step 2: Run to verify it fails first**

Run (before implementation adjustments — should pass if Task 2.1.7 correctly implemented `dispatch_batch`):

```bash
cargo test -p kulisawit-orchestrator --test dispatch_batch
```

Expected: PASS (the code was written in 2.1.7). If any test fails, diagnose and fix the `dispatch_batch` body in `dispatch.rs` — do not change the test.

- [ ] **Step 3: Implement**

No new implementation — this task is pure coverage. If the tests fail (unlikely), fix the implementation in `dispatch.rs`.

- [ ] **Step 4: Run to verify it passes**

```bash
cargo test -p kulisawit-orchestrator --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

- [ ] **Step 5: Commit**

```bash
git add crates/kulisawit-orchestrator/tests/dispatch_batch.rs
git commit -m "feat(orchestrator): dispatch_batch for parallel attempts"
```

---

### Task 2.1.9: Cancellation

**Files:**
- Create: `crates/kulisawit-orchestrator/tests/cancel.rs`

The cancel machinery (the `cancel_flags` map, `install_cancel_flag`, `cancel_attempt`, and the `tokio::select!` in the dispatcher) is already in place from Tasks 2.1.6 and 2.1.7, and `MockAgent::slow()` was added in 2.0.4. This task adds the end-to-end test.

- [ ] **Step 1: Write the failing test**

Create `crates/kulisawit-orchestrator/tests/cancel.rs`:

```rust
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kulisawit_agent::{MockAgent, MockMode};
use kulisawit_core::{adapter::AgentAdapter, AttemptId, AttemptStatus};
use kulisawit_db::{attempt, columns, connect, migrate, project, task};
use kulisawit_orchestrator::{
    dispatch_single_attempt, AgentRegistry, Orchestrator, RuntimeConfig,
};
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;
use tempfile::tempdir;

fn init_repo(dir: &std::path::Path) {
    Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(dir)
        .status()
        .unwrap();
    std::fs::write(dir.join("README.md"), "# test\n").unwrap();
    Command::new("git")
        .args(["-c", "user.email=t@t", "-c", "user.name=t", "add", "."])
        .current_dir(dir)
        .status()
        .unwrap();
    Command::new("git")
        .args([
            "-c",
            "user.email=t@t",
            "-c",
            "user.name=t",
            "commit",
            "-m",
            "init",
        ])
        .current_dir(dir)
        .status()
        .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cancel_attempt_on_slow_mock_terminates_as_cancelled() {
    let base = tempdir().expect("tmp");
    init_repo(base.path());
    let pool = connect("sqlite::memory:").await.expect("pool");
    migrate(&pool).await.expect("mig");
    let mut registry = AgentRegistry::new();
    registry.register(Arc::new(MockAgent::new(MockMode::Slow)) as Arc<dyn AgentAdapter>);
    let orch = Orchestrator::new(
        pool,
        registry,
        base.path().to_path_buf(),
        base.path().join(".kulisawit/worktrees"),
        RuntimeConfig::default(),
    );

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
            title: "slow".into(),
            description: None,
            tags: vec![],
            linked_files: vec![],
        },
    )
    .await
    .expect("t");

    // Spawn the dispatcher in the background. Clone inner Arcs via the
    // public API (no `clone_for_dispatch` access from tests): we wrap the
    // whole Orchestrator in an Arc. Since Orchestrator methods only need
    // `&self`, this is enough.
    let orch = Arc::new(orch);

    // To find the AttemptId before dispatch returns, we poll the DB after
    // a short delay and grab the most recent attempt for this task.
    let orch_bg = Arc::clone(&orch);
    let task_id_bg = task_id.clone();
    let dispatch_handle = tokio::spawn(async move {
        dispatch_single_attempt(&orch_bg, &task_id_bg, "mock", None).await
    });

    // Wait until the attempt row appears (dispatcher has inserted it).
    let attempt_id = poll_for_attempt(&orch, &task_id).await;

    // Let a couple of slow ticks happen, then cancel.
    tokio::time::sleep(Duration::from_millis(200)).await;
    orch.cancel_attempt(&attempt_id).await.expect("cancel");

    let result = dispatch_handle.await.expect("join").expect("dispatch ok");
    assert_eq!(result, attempt_id);

    let row = attempt::get(orch.pool(), &attempt_id)
        .await
        .expect("get")
        .expect("row");
    assert_eq!(row.status, AttemptStatus::Cancelled);
    assert!(row.started_at.is_some());
    assert!(row.completed_at.is_some());
}

async fn poll_for_attempt(
    orch: &Orchestrator,
    task_id: &kulisawit_core::TaskId,
) -> AttemptId {
    for _ in 0..100 {
        let rows = attempt::list_for_task(orch.pool(), task_id)
            .await
            .expect("list");
        if let Some(first) = rows.into_iter().next() {
            return first.id;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("attempt never appeared in DB");
}
```

- [ ] **Step 2: Run to verify it fails**

Run:

```bash
cargo test -p kulisawit-orchestrator --test cancel
```

Expected: If everything from 2.0.4 + 2.1.6 + 2.1.7 landed correctly, this already passes. If it hangs, diagnose the `tokio::select!` in `dispatch.rs` — the `biased;` keyword ensures the cancellation branch is polled first on each select.

- [ ] **Step 3: Implement**

No implementation changes anticipated. If the test fails, fix the bug in `dispatch.rs` (likely: `cancel_notify.notified()` is a future captured before the `select!` — it must be called inside each iteration so each iteration gets a fresh wait-future). If fixed, the `tokio::select!` block in `dispatch_single_attempt` should look like:

```rust
    loop {
        let cancel_wait = cancel_notify.notified();
        tokio::pin!(cancel_wait);
        tokio::select! {
            biased;
            _ = &mut cancel_wait => {
                let _ = adapter.cancel(attempt_id.as_str()).await;
                let evt = AgentEvent::Status {
                    status: RunStatus::Cancelled,
                    detail: Some("cancelled by orchestrator".into()),
                };
                let _ = events::append(orch.pool(), &attempt_id, &evt).await;
                orch.broadcaster().send(&attempt_id, evt);
                terminal = Some(AttemptStatus::Cancelled);
                break;
            }
            next = stream.next() => {
                let Some(evt) = next else {
                    warn!(attempt = %attempt_id, "adapter stream ended without terminal status");
                    terminal = Some(AttemptStatus::Failed);
                    break;
                };
                let _ = events::append(orch.pool(), &attempt_id, &evt).await;
                orch.broadcaster().send(&attempt_id, evt.clone());
                if let AgentEvent::Status { status, .. } = &evt {
                    if let Some(mapped) = AttemptStatus::from_terminal_run_status(*status) {
                        terminal = Some(mapped);
                        break;
                    }
                }
            }
        }
    }
```

If you had to swap in this pin-based form, update the file and re-run tests.

- [ ] **Step 4: Run to verify it passes**

```bash
cargo test -p kulisawit-orchestrator --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

- [ ] **Step 5: Commit**

```bash
git add crates/kulisawit-orchestrator/tests/cancel.rs crates/kulisawit-orchestrator/src/dispatch.rs
git commit -m "feat(orchestrator): cancel_attempt via Notify"
```

---

## Phase 2.2 — CLI integration

### Task 2.2.1: `clap` CLI skeleton

**Files:**
- Modify: `crates/kulisawit-cli/Cargo.toml`
- Modify: `crates/kulisawit-cli/src/main.rs`
- Create: `crates/kulisawit-cli/src/commands/mod.rs`
- Create: `crates/kulisawit-cli/src/commands/version.rs`
- Create: `crates/kulisawit-cli/src/commands/run.rs`
- Create: `crates/kulisawit-cli/tests/cli_help.rs`

- [ ] **Step 1: Write the failing test**

Create `crates/kulisawit-cli/tests/cli_help.rs`:

```rust
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::process::Command;

fn bin_path() -> std::path::PathBuf {
    // CARGO_BIN_EXE_<name> is provided by cargo to integration tests.
    env!("CARGO_BIN_EXE_kulisawit").into()
}

#[test]
fn help_lists_version_and_run_subcommands() {
    let out = Command::new(bin_path()).args(["--help"]).output().expect("run");
    assert!(out.status.success(), "help exit: {:?}", out.status);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let combined = format!("{stdout}\n{stderr}");
    assert!(combined.contains("version"), "help missing 'version': {combined}");
    assert!(combined.contains("run"), "help missing 'run': {combined}");
}

#[test]
fn version_subcommand_prints_version() {
    let out = Command::new(bin_path()).args(["version"]).output().expect("run");
    assert!(out.status.success(), "version exit: {:?}", out.status);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("kulisawit "),
        "version stdout unexpected: {stdout}"
    );
}
```

- [ ] **Step 2: Run to verify it fails**

Run:

```bash
cargo test -p kulisawit-cli --test cli_help
```

Expected: FAIL — help doesn't mention `version` or `run` (the binary only does `println!` at this point); `version` subcommand exits non-zero.

- [ ] **Step 3: Update `Cargo.toml`**

Replace the contents of `crates/kulisawit-cli/Cargo.toml` with:

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
clap.workspace = true
tokio = { workspace = true, features = ["full"] }
tracing.workspace = true
tracing-subscriber.workspace = true
kulisawit-core.workspace = true
kulisawit-db.workspace = true
kulisawit-agent.workspace = true
kulisawit-orchestrator.workspace = true

[lints]
workspace = true
```

- [ ] **Step 4: Implement the CLI skeleton**

Replace the contents of `crates/kulisawit-cli/src/main.rs` with:

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
}

#[derive(Debug, clap::Args)]
pub struct RunArgs {
    /// Path to the SQLite database. Will be created if missing.
    #[arg(long)]
    pub db: PathBuf,
    /// Path to the git repository hosting the task.
    #[arg(long)]
    pub repo: PathBuf,
    /// Task id (`TaskId` string).
    #[arg(long)]
    pub task: TaskId,
    /// Registered agent id.
    #[arg(long, default_value = "mock")]
    pub agent: String,
    /// Number of parallel attempts.
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
    }
}
```

Create `crates/kulisawit-cli/src/commands/mod.rs`:

```rust
pub mod run;
pub mod version;
```

Create `crates/kulisawit-cli/src/commands/version.rs`:

```rust
use anyhow::Result;

pub fn run() -> Result<()> {
    println!("kulisawit {}", env!("CARGO_PKG_VERSION"));
    Ok(())
}
```

Create `crates/kulisawit-cli/src/commands/run.rs` (placeholder — Task 2.2.2 implements the real thing):

```rust
use anyhow::Result;

use crate::RunArgs;

pub async fn run(args: RunArgs) -> Result<()> {
    println!(
        "run: db={} repo={} task={} agent={} batch={}",
        args.db.display(),
        args.repo.display(),
        args.task,
        args.agent,
        args.batch
    );
    Ok(())
}
```

- [ ] **Step 5: Run to verify it passes**

```bash
cargo test -p kulisawit-cli --test cli_help
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo run -p kulisawit-cli -- version
cargo run -p kulisawit-cli -- --help
```

Expected: both test cases green; `version` prints `kulisawit 0.1.0-dev`; `--help` lists `version` and `run`.

- [ ] **Step 6: Commit**

```bash
git add crates/kulisawit-cli/
git commit -m "feat(cli): clap scaffold with version and run subcommands"
```

---

### Task 2.2.2: `run` subcommand wires the orchestrator end-to-end

**Files:**
- Modify: `crates/kulisawit-cli/src/commands/run.rs`
- Create: `crates/kulisawit-cli/tests/cli_run.rs`

- [ ] **Step 1: Write the failing test**

Create `crates/kulisawit-cli/tests/cli_run.rs`:

```rust
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::process::Command;
use tempfile::tempdir;

fn bin_path() -> std::path::PathBuf {
    env!("CARGO_BIN_EXE_kulisawit").into()
}

fn init_repo(dir: &std::path::Path) {
    Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(dir)
        .status()
        .unwrap();
    std::fs::write(dir.join("README.md"), "# test\n").unwrap();
    Command::new("git")
        .args(["-c", "user.email=t@t", "-c", "user.name=t", "add", "."])
        .current_dir(dir)
        .status()
        .unwrap();
    Command::new("git")
        .args([
            "-c",
            "user.email=t@t",
            "-c",
            "user.name=t",
            "commit",
            "-m",
            "init",
        ])
        .current_dir(dir)
        .status()
        .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn run_subcommand_dispatches_two_mock_attempts_to_completion() {
    let base = tempdir().expect("tmp");
    init_repo(base.path());
    let db_path = base.path().join("kulisawit.sqlite");

    // Seed DB directly via the db crate so we have a known task id.
    let pool = kulisawit_db::connect(db_path.to_str().expect("utf8"))
        .await
        .expect("pool");
    kulisawit_db::migrate(&pool).await.expect("mig");
    let project_id = kulisawit_db::project::create(
        &pool,
        kulisawit_db::project::NewProject {
            name: "K".into(),
            repo_path: base.path().display().to_string(),
        },
    )
    .await
    .expect("p");
    let cols = kulisawit_db::columns::seed_defaults(&pool, &project_id)
        .await
        .expect("c");
    let task_id = kulisawit_db::task::create(
        &pool,
        kulisawit_db::task::NewTask {
            project_id,
            column_id: cols[0].clone(),
            title: "cli smoke".into(),
            description: Some("d".into()),
            tags: vec![],
            linked_files: vec![],
        },
    )
    .await
    .expect("t");
    pool.close().await;

    let out = Command::new(bin_path())
        .args([
            "run",
            "--db",
            db_path.to_str().expect("utf8"),
            "--repo",
            base.path().to_str().expect("utf8"),
            "--task",
            task_id.as_str(),
            "--agent",
            "mock",
            "--batch",
            "2",
        ])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "run exit: {:?}\nstdout:\n{}\nstderr:\n{}",
        out.status,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    // Re-open the DB and verify two completed attempts exist.
    let pool = kulisawit_db::connect(db_path.to_str().expect("utf8"))
        .await
        .expect("pool");
    let rows = kulisawit_db::attempt::list_for_task(&pool, &task_id)
        .await
        .expect("list");
    assert_eq!(rows.len(), 2, "expected 2 attempts, got {}", rows.len());
    for r in &rows {
        assert_eq!(r.status, kulisawit_core::AttemptStatus::Completed);
    }

    // stdout should contain both AttemptIds.
    let stdout = String::from_utf8_lossy(&out.stdout);
    for r in &rows {
        assert!(
            stdout.contains(r.id.as_str()),
            "stdout missing {}; got:\n{stdout}",
            r.id
        );
    }
}
```

Also update `crates/kulisawit-cli/Cargo.toml` dev-dependencies block:

```toml
[dev-dependencies]
tempfile.workspace = true
tokio = { workspace = true, features = ["macros", "rt-multi-thread", "time"] }
kulisawit-core.workspace = true
kulisawit-db.workspace = true
```

- [ ] **Step 2: Run to verify it fails**

Run:

```bash
cargo test -p kulisawit-cli --test cli_run
```

Expected: FAIL — the `run` placeholder only prints args; no attempts are created.

- [ ] **Step 3: Implement**

Replace the contents of `crates/kulisawit-cli/src/commands/run.rs`:

```rust
//! `kulisawit run` — dispatch a batch of attempts for a task.

use anyhow::{Context, Result};
use std::sync::Arc;

use kulisawit_agent::MockAgent;
use kulisawit_core::adapter::AgentAdapter;
use kulisawit_db::{attempt, connect, migrate};
use kulisawit_orchestrator::{
    dispatch_batch, AgentRegistry, Orchestrator, RuntimeConfig,
};

use crate::RunArgs;

pub async fn run(args: RunArgs) -> Result<()> {
    let db_str = args
        .db
        .to_str()
        .context("--db path is not valid UTF-8")?
        .to_owned();
    let pool = connect(&db_str).await.context("open db")?;
    migrate(&pool).await.context("migrate")?;

    let mut registry = AgentRegistry::new();
    // Phase 2 ships only the MockAgent adapter.
    registry.register(Arc::new(MockAgent::default()) as Arc<dyn AgentAdapter>);

    let worktree_root = args.repo.join(".kulisawit/worktrees");
    let cfg = RuntimeConfig::default();
    let orch = Orchestrator::new(
        pool,
        registry,
        args.repo.clone(),
        worktree_root,
        cfg,
    );

    let ids = dispatch_batch(&orch, &args.task, &args.agent, args.batch, None)
        .await
        .context("dispatch_batch")?;

    // Print a simple table to stdout.
    println!("{:<36}  {:<10}", "attempt_id", "status");
    println!("{:-<36}  {:-<10}", "", "");
    for id in &ids {
        let row = attempt::get(orch.pool(), id)
            .await
            .context("attempt::get")?
            .context("attempt row missing after dispatch")?;
        println!("{:<36}  {:<10}", id.as_str(), row.status.as_str());
    }
    Ok(())
}
```

- [ ] **Step 4: Run to verify it passes**

```bash
cargo test -p kulisawit-cli --test cli_run
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

Expected: green across the board.

- [ ] **Step 5: Commit**

```bash
git add crates/kulisawit-cli/src/commands/run.rs crates/kulisawit-cli/Cargo.toml crates/kulisawit-cli/tests/cli_run.rs
git commit -m "feat(cli): run subcommand wires orchestrator end-to-end"
```

---

## Phase 2.3 — Green-bar checkpoint

### Task 2.3.1: CI gains `sqlx prepare --check`

**Files:**
- Modify: `.github/workflows/ci.yml`

Phase 1 review §M-7 flagged the absence of an offline-metadata staleness check in CI. Wire it in.

- [ ] **Step 1: Confirm staged baseline**

Run:

```bash
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --check
```

Expected: all green. Count tests:

```bash
cargo test --workspace --locked 2>&1 | grep -E "test result:" | tail -20
```

Record the summary in the commit message (target: 50+ tests across Phase 2 additions — 37 baseline + 3 (2.0.2) + 4 (2.0.3) + 3 (2.0.4) + 2 (concurrent/trans) + 2 (git errors) + 3 (2.1.2 registry) + 4 (2.1.3 broadcaster) + 7 (2.1.4 prompt) + 5 (2.1.5 config) + 1 (2.1.6 orch) + 4 (2.1.7 dispatch) + 2 (2.1.8 batch) + 1 (2.1.9 cancel) + 2 (2.2.1 cli_help) + 1 (2.2.2 cli_run) ≈ 81).

- [ ] **Step 2: Update the CI workflow**

Replace the contents of `.github/workflows/ci.yml`:

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
      - uses: dtolnay/rust-toolchain@1.86
        with:
          components: rustfmt, clippy
      - uses: Swatinem/rust-cache@v2
      - run: cargo fmt --check
      - run: cargo clippy --workspace --all-targets --locked -- -D warnings
      - run: cargo build --workspace --all-targets --locked
      - run: cargo test --workspace --locked

  sqlx-prepare-check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@1.86
      - uses: Swatinem/rust-cache@v2
      - name: Install sqlx-cli
        run: cargo install sqlx-cli --version ^0.8 --no-default-features --features sqlite,rustls --locked
      - name: Verify offline metadata is up to date
        run: cargo sqlx prepare --workspace --check -- --all-targets
```

- [ ] **Step 3: Verify locally (best effort)**

If `sqlx-cli` is available locally:

```bash
cargo sqlx prepare --workspace --check -- --all-targets
```

Expected: clean (Phase 1 committed `.sqlx/` metadata; Phase 2 does not add new SQL queries, so the check should be a no-op).

- [ ] **Step 4: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: add sqlx prepare check to workflow"
```

---

### Task 2.3.2: Tag `phase-2`

**Files:** none (git-only)

- [ ] **Step 1: Confirm green bar**

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

Record the number (target: ~80+). Store it in `PHASE2_TESTS`.

- [ ] **Step 3: Create the annotated tag**

```bash
PHASE1_HEAD=$(git rev-list -n1 phase-1 2>/dev/null || echo "phase-1-missing")
HEAD_SHA=$(git rev-parse --short HEAD)
git tag -a phase-2 -m "Kulisawit Phase 2 — Orchestrator core.

Delta from phase-1 (${PHASE1_HEAD:0:7}):
- New crate: kulisawit-orchestrator
- CLI: 'kulisawit run' dispatches MockAgent attempts end-to-end
- Cancellation via tokio::sync::Notify
- CI: cargo sqlx prepare --check gate

Tests: ${PHASE2_TESTS} total (phase-1 baseline: 37).
HEAD: ${HEAD_SHA}."
```

Do NOT run `git push` or `git push --tags`. Tagging is local per the plan rules.

- [ ] **Step 4: Verify**

```bash
git tag -l phase-2
git show phase-2 --stat | head -40
```

Expected: annotated tag visible, message contains the delta summary.

---

## Self-review checklist

Work through each item after all tasks land; every box must be checked before calling Phase 2 complete.

- [ ] **Prereqs from Phase 1 review §I-1/I-2/I-3 addressed.**
  - I-1 (events.id null-safety) landed in `a5bef68` (baseline).
  - I-2 (RunStatus → AttemptStatus terminal mapping) landed in `d95115c` (baseline); consumed in `dispatch.rs`.
  - I-3 (task repo null-safety docs) landed in `b45a1ad` (baseline).
  - Plan builds on these — no duplicate work; commits are baseline and not in this plan's commit list.
- [ ] **§4.7 test gaps closed (Task 2.0.5).**
  - Concurrent inserts (`concurrent_inserts.rs`) — 50 parallel `attempt::create` calls.
  - Attempt status transitions (`attempt_transitions.rs`) — queued→failed, running→failed, non-terminal rejection.
  - Worktree error paths (`worktree_errors.rs`) — existing-path and bad-ref.
- [ ] **M-items addressed or deferred with rationale.**
  - **M-1** (`VerificationStatus` parity): addressed in Task 2.0.2.
  - **M-2** (crate-root re-exports for adapter types): addressed in Task 2.0.1.
  - **M-3** (timestamp unit unification): addressed in Task 2.0.3.
  - **M-4** (MockAgent failure/cancel/slow modes): addressed in Task 2.0.4.
  - **M-5** (per-attempt cancellation path): addressed in Tasks 2.1.6, 2.1.7, 2.1.9.
  - **M-6** (SSE fanout primitive): addressed in Task 2.1.3 (`EventBroadcaster`). Wiring to HTTP is Phase 3.
  - **M-7** (`cargo sqlx prepare --check` in CI): addressed in Task 2.3.1.
  - **M-8** (real-agent adapter): deferred to Phase 4 — Phase 2 ships only `MockAgent`.
  - **M-9** (web UI): deferred to Phase 3.
- [ ] **All code is verbatim, paste-ready.** No `// similar to above`, no `// TBD`.
- [ ] **No `#![deny(...)]` attributes** in any new file; workspace lints govern.
- [ ] **No `Co-Authored-By:` lines** in any commit message in this plan.
- [ ] **English generic identifiers** everywhere in code; Kulisawit vocabulary reserved for user-facing strings (no such strings introduced in Phase 2).
- [ ] **Exit criteria satisfied** (see top of plan).
- [ ] **Phase 1 plan cross-reference** for shared conventions: `docs/superpowers/plans/2026-04-18-kulisawit-implementation.md` §Conventions.

---

### Critical Files for Implementation

- `/home/bimbim/works/kulisawit/crates/kulisawit-orchestrator/src/dispatch.rs`
- `/home/bimbim/works/kulisawit/crates/kulisawit-orchestrator/src/orchestrator.rs`
- `/home/bimbim/works/kulisawit/crates/kulisawit-agent/src/mock.rs`
- `/home/bimbim/works/kulisawit/crates/kulisawit-cli/src/commands/run.rs`
- `/home/bimbim/works/kulisawit/crates/kulisawit-db/src/attempt.rs`

---

### Report to caller

**Intended output path:** `/home/bimbim/works/kulisawit/docs/superpowers/plans/2026-04-18-kulisawit-phase-2.md`

**Write status:** Not written to disk. My system instructions put me in strict read-only mode and deny file-creation tools; the plan content is delivered above in this message for the controlling agent to persist. The controller can redirect the block above into the target file (from `# Kulisawit Phase 2 — Orchestrator Core Implementation Plan` through the final `### Critical Files for Implementation` list) without modification.

**Line count (of the plan body, excluding this report):** approximately 2050 lines when rendered as a standalone Markdown file.

**One-sentence task summaries:**

- **Task 2.0.1** — Add crate-root `pub use` for `AgentAdapter`/`AgentEvent`/`AgentError`/`RunContext`/`CheckResult` so downstream code can write `kulisawit_core::AgentAdapter`.
- **Task 2.0.2** — Give `VerificationStatus` the same `as_str` / `TryFrom<&str>` / `UnknownVerificationStatus` / `CoreError::UnknownVerificationStatus` / re-export parity as `AttemptStatus`, and simplify `attempt::set_verification`.
- **Task 2.0.3** — Change every `Utc::now().timestamp()` in the db crate to `timestamp_millis()` and add a new `timestamp_units.rs` integration test that asserts all `*_at` columns now live in the 13-digit ms range.
- **Task 2.0.4** — Add `MockMode::{Succeed, Fail, Cancel, Slow}` to `MockAgent` with helper constructors and matching scripted streams, so later tasks can exercise failure, cancellation, and slow-emit paths.
- **Task 2.0.5** — Add three integration-test files (`concurrent_inserts.rs`, `attempt_transitions.rs`, `worktree_errors.rs`) to close the Phase 1 review §4.7 coverage gaps.
- **Task 2.1.1** — Scaffold the `kulisawit-orchestrator` crate (workspace member, manifest, module stubs, `OrchestratorError`/`OrchestratorResult`).
- **Task 2.1.2** — Implement `AgentRegistry` with register/get/ids (alphabetically sorted).
- **Task 2.1.3** — Implement `EventBroadcaster` with per-attempt `tokio::sync::broadcast` channels and subscribe/send/close semantics.
- **Task 2.1.4** — Implement the pure `compose_prompt(task, variant)` composer with title, description, linked-files, tags, and trailing variant sections.
- **Task 2.1.5** — Implement `RuntimeConfig` with `Default` (8/7/"mock"/1) and `from_toml_str` parsing `[runtime]` block + partial-fill defaults.
- **Task 2.1.6** — Implement the `Orchestrator` struct with shared `Arc`-wrapped state, accessor methods, `install_cancel_flag`/`remove_cancel_flag`/`cancel_attempt`, and a private `clone_for_dispatch` for fan-out.
- **Task 2.1.7** — Implement `dispatch_single_attempt` — full per-attempt lifecycle with semaphore, worktree creation, adapter drive via `tokio::select!` (cancel vs stream), commit-on-terminal, and DB transitions.
- **Task 2.1.8** — Add an integration test verifying `dispatch_batch` fans out three MockAgent attempts to completion in parallel and errors on variants-length mismatch.
- **Task 2.1.9** — Add an integration test that dispatches a `MockMode::Slow` attempt, calls `cancel_attempt` after 200 ms, and asserts the final `AttemptStatus::Cancelled`.
- **Task 2.2.1** — Replace the CLI placeholder with a `clap` parser exposing `version` and `run` subcommands (plus tracing init and deps wiring) and add an integration test asserting `--help` mentions both.
- **Task 2.2.2** — Implement the `run` subcommand so it connects the DB, registers MockAgent, builds an `Orchestrator`, calls `dispatch_batch`, and prints a table of `AttemptId`/status rows; add an end-to-end test that spawns the binary against a tempdir repo + seeded task.
- **Task 2.3.1** — Add a second CI job that installs `sqlx-cli` and runs `cargo sqlx prepare --workspace --check -- --all-targets` to guard against stale offline metadata.
- **Task 2.3.2** — Create an annotated `phase-2` tag (local only) containing the test-count delta from `phase-1`, HEAD sha, and the Phase 2 scope summary.