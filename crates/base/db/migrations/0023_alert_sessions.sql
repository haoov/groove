-- The child a maintenance task spawned for an alert, by the alert's deduplication key.
CREATE TABLE alert_sessions (
    session_id  TEXT PRIMARY KEY REFERENCES sessions(id)
                  ON DELETE CASCADE ON UPDATE CASCADE,
    parent_id   TEXT NOT NULL REFERENCES sessions(id)
                  ON DELETE CASCADE ON UPDATE CASCADE,
    dedup_key   TEXT NOT NULL,
    title       TEXT NOT NULL,
    priority    TEXT,
    cluster     TEXT,
    namespace   TEXT,
    UNIQUE (parent_id, dedup_key)
);
