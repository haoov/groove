-- The volatile part of a session, one leaf row, so `sessions` is never altered again.
CREATE TABLE session_state (
    session_id            TEXT PRIMARY KEY REFERENCES sessions(id)
                            ON DELETE CASCADE ON UPDATE CASCADE,
    opened_at             INTEGER,
    seen_at               INTEGER,
    auto_approve          INTEGER NOT NULL DEFAULT 0,
    selected_worktree_id  TEXT REFERENCES worktrees(id) ON DELETE SET NULL
);
