-- One line per thing worth remembering about a session: what it was, when, and the
-- payload the surface reads it back with. Rows go with the session they belong to.
CREATE TABLE timeline (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL REFERENCES sessions(id)
                 ON DELETE CASCADE ON UPDATE CASCADE,
    at         INTEGER NOT NULL,
    kind       TEXT NOT NULL,
    subject    TEXT NOT NULL DEFAULT '',
    payload    TEXT NOT NULL DEFAULT '{}'
);
CREATE INDEX ix_timeline_session_at ON timeline (session_id, at DESC, id DESC);
