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
