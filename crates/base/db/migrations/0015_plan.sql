-- The user's own order for the tasks waiting. Groove's alone: no provider is told.
CREATE TABLE plan (
    external_id TEXT PRIMARY KEY,
    at          INTEGER NOT NULL,
    later       INTEGER NOT NULL DEFAULT 0
);
