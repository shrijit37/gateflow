-- Gateflow schema v1

CREATE TABLE IF NOT EXISTS workflows (
    id         TEXT PRIMARY KEY,
    name       TEXT NOT NULL,
    slug       TEXT NOT NULL UNIQUE,
    definition TEXT NOT NULL,              -- JSON: WorkflowDag
    version    INTEGER NOT NULL DEFAULT 1,
    deployed   INTEGER NOT NULL DEFAULT 0, -- 1 = compiled and in cache
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE TABLE IF NOT EXISTS executions (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    request_id  TEXT NOT NULL,
    workflow_id TEXT NOT NULL,
    slug        TEXT NOT NULL,
    protocol    TEXT,
    status      TEXT NOT NULL,   -- ok | upstream_error | timeout | internal_error | client_disconnect
    status_code INTEGER,
    latency_ms  INTEGER,
    bytes_in    INTEGER DEFAULT 0,
    bytes_out   INTEGER DEFAULT 0,
    started_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE INDEX IF NOT EXISTS idx_executions_workflow
    ON executions(workflow_id, started_at DESC);