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

_Filled in Task 6._

## Plantation Glossary

_Filled in Task 6._

## Feature Matrix

_Filled in Task 6._

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
