-- Profiles and blocking rules
CREATE TABLE IF NOT EXISTS profiles (
    id          TEXT    PRIMARY KEY NOT NULL,
    name        TEXT    NOT NULL,
    revision    INTEGER NOT NULL DEFAULT 1,
    data        TEXT    NOT NULL DEFAULT '{}',
    created_at  TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at  TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS block_rules (
    id          TEXT    PRIMARY KEY NOT NULL,
    profile_id  TEXT    NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    kind        TEXT    NOT NULL CHECK (kind IN ('App', 'Domain', 'Category', 'Network')),
    target      TEXT    NOT NULL,
    enabled     INTEGER NOT NULL DEFAULT 1,
    created_at  TEXT    NOT NULL DEFAULT (datetime('now'))
);

-- Schedules
CREATE TABLE IF NOT EXISTS schedules (
    id              TEXT    PRIMARY KEY NOT NULL,
    profile_id      TEXT    NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    name            TEXT    NOT NULL,
    cron_expr       TEXT,
    duration_secs   INTEGER NOT NULL,
    enabled         INTEGER NOT NULL DEFAULT 1,
    created_at      TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT    NOT NULL DEFAULT (datetime('now'))
);

-- Sessions and event sourcing
CREATE TABLE IF NOT EXISTS sessions (
    id                      TEXT    PRIMARY KEY NOT NULL,
    profile_id              TEXT    REFERENCES profiles(id),
    mode                    TEXT    NOT NULL CHECK (mode IN ('Focus', 'ShortBreak', 'LongBreak', 'Strict')),
    state                   TEXT    NOT NULL DEFAULT 'Idle',
    planned_duration_secs   INTEGER NOT NULL,
    started_at              TEXT,
    completed_at            TEXT,
    created_at              TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS session_events (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id  TEXT    NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    event_type  TEXT    NOT NULL,
    payload     TEXT,
    occurred_at TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS session_snapshots (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id      TEXT    NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    state_snapshot  TEXT    NOT NULL,
    snapshotted_at  TEXT    NOT NULL DEFAULT (datetime('now'))
);

-- Device and system state
CREATE TABLE IF NOT EXISTS device_state (
    key         TEXT PRIMARY KEY NOT NULL,
    value       TEXT NOT NULL,
    updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS permission_state (
    name        TEXT PRIMARY KEY NOT NULL,
    status      TEXT NOT NULL CHECK (status IN ('Healthy', 'Degraded', 'Unavailable', 'PermissionDenied')),
    detail      TEXT,
    checked_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Enforcement tracking
CREATE TABLE IF NOT EXISTS enforcement_state (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id      TEXT    REFERENCES sessions(id) ON DELETE SET NULL,
    plan_revision   INTEGER NOT NULL,
    plan_hash       TEXT    NOT NULL,
    applied_at      TEXT    NOT NULL DEFAULT (datetime('now')),
    status          TEXT    NOT NULL CHECK (status IN ('Applied', 'Partial', 'Failed'))
);

CREATE TABLE IF NOT EXISTS tamper_events (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    event_type  TEXT    NOT NULL,
    detail      TEXT,
    detected_at TEXT    NOT NULL DEFAULT (datetime('now'))
);

-- Optional sync operation log (append-only, encrypted payloads)
CREATE TABLE IF NOT EXISTS sync_ops (
    id                  TEXT    PRIMARY KEY NOT NULL,
    entity_type         TEXT    NOT NULL,
    entity_id           TEXT    NOT NULL,
    operation           TEXT    NOT NULL CHECK (operation IN ('Upsert', 'Delete')),
    payload_encrypted   BLOB,
    lamport_ts          INTEGER NOT NULL,
    device_id           TEXT    NOT NULL,
    created_at          TEXT    NOT NULL DEFAULT (datetime('now')),
    synced_at           TEXT
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_session_events_session_id  ON session_events(session_id);
CREATE INDEX IF NOT EXISTS idx_sessions_state             ON sessions(state);
CREATE INDEX IF NOT EXISTS idx_block_rules_profile_id     ON block_rules(profile_id);
CREATE INDEX IF NOT EXISTS idx_sync_ops_unsynced          ON sync_ops(synced_at) WHERE synced_at IS NULL;
