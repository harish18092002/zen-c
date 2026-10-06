-- Denormalize SessionState fields into explicit columns for queryability.
-- The `state` column now stores only the discriminant ("Idle", "Active", ...);
-- variant payloads live in dedicated columns alongside it.

ALTER TABLE sessions ADD COLUMN ends_at              TEXT;
ALTER TABLE sessions ADD COLUMN paused_remaining_secs INTEGER;
ALTER TABLE sessions ADD COLUMN aborted_reason       TEXT;
ALTER TABLE sessions ADD COLUMN updated_at           TEXT NOT NULL DEFAULT (datetime('now'));

CREATE INDEX IF NOT EXISTS idx_sessions_updated_at ON sessions(updated_at);
