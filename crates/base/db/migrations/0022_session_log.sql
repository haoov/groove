-- Every session keeps a log of what it did; a child writes to its parent's.
DROP TABLE maintenance_log;

CREATE TABLE session_log (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id  TEXT NOT NULL REFERENCES sessions(id)
                  ON DELETE CASCADE ON UPDATE CASCADE,
    from_child  TEXT,
    what        TEXT NOT NULL,
    why         TEXT,
    mr_url      TEXT,
    at          INTEGER NOT NULL
);
