-- Kulisawit initial schema.

PRAGMA foreign_keys = ON;

CREATE TABLE project (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    repo_path   TEXT NOT NULL,
    created_at  INTEGER NOT NULL
);

CREATE TABLE columns (
    id         TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES project(id) ON DELETE CASCADE,
    name       TEXT NOT NULL,
    position   INTEGER NOT NULL
);

CREATE INDEX idx_columns_project ON columns(project_id, position);

CREATE TABLE task (
    id            TEXT PRIMARY KEY,
    project_id    TEXT NOT NULL REFERENCES project(id) ON DELETE CASCADE,
    column_id     TEXT NOT NULL REFERENCES columns(id) ON DELETE RESTRICT,
    title         TEXT NOT NULL,
    description   TEXT,
    position      INTEGER NOT NULL,
    tags          TEXT,            -- JSON array
    linked_files  TEXT,            -- JSON array of repo-relative paths
    created_at    INTEGER NOT NULL,
    updated_at    INTEGER NOT NULL
);

CREATE INDEX idx_task_project ON task(project_id, column_id, position);

CREATE TABLE attempt (
    id                  TEXT PRIMARY KEY,
    task_id             TEXT NOT NULL REFERENCES task(id) ON DELETE CASCADE,
    agent_id            TEXT NOT NULL,
    prompt_variant      TEXT,
    worktree_path       TEXT NOT NULL,
    branch_name         TEXT NOT NULL,
    status              TEXT NOT NULL CHECK (status IN ('queued','running','completed','failed','cancelled')),
    started_at          INTEGER,
    completed_at        INTEGER,
    verification_status TEXT CHECK (verification_status IN ('pending','passed','failed','skipped')),
    verification_output TEXT
);

CREATE INDEX idx_attempt_task ON attempt(task_id);
CREATE INDEX idx_attempt_status ON attempt(status);

CREATE TABLE events (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    attempt_id TEXT NOT NULL REFERENCES attempt(id) ON DELETE CASCADE,
    timestamp  INTEGER NOT NULL,
    type       TEXT NOT NULL,
    payload    TEXT NOT NULL      -- JSON
);

CREATE INDEX idx_events_attempt ON events(attempt_id, timestamp);
