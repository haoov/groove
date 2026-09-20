-- Time on a task, measured here and logged to the source deliberately. Two counters,
-- never one: the difference is what is left to log, so logging twice cannot double it.
CREATE TABLE ledger (
    external_id     TEXT PRIMARY KEY,
    tracked_seconds INTEGER NOT NULL DEFAULT 0,
    logged_seconds  INTEGER NOT NULL DEFAULT 0,
    today_day       TEXT NOT NULL DEFAULT '',
    today_seconds   INTEGER NOT NULL DEFAULT 0,
    updated_at      INTEGER NOT NULL
);
