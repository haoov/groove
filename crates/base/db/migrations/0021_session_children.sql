-- A maintenance task, the session that spawned a child, and the log its children write.
ALTER TABLE session_state ADD COLUMN maintenance INTEGER NOT NULL DEFAULT 0;
ALTER TABLE session_state ADD COLUMN parent_id TEXT
    REFERENCES sessions(id) ON DELETE SET NULL ON UPDATE CASCADE;

CREATE TABLE maintenance_log (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id  TEXT NOT NULL REFERENCES sessions(id)
                  ON DELETE CASCADE ON UPDATE CASCADE,
    child       TEXT NOT NULL,
    problem     TEXT NOT NULL,
    action      TEXT NOT NULL,
    mr_url      TEXT,
    at          INTEGER NOT NULL
);
