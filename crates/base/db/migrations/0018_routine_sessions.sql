-- The session each standalone routine runs in, beside its `sessions` row of kind explorer.
CREATE TABLE routine_sessions (
    session_id  TEXT PRIMARY KEY REFERENCES sessions(id) ON DELETE CASCADE ON UPDATE CASCADE,
    routine     TEXT NOT NULL UNIQUE
);
