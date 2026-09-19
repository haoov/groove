-- Which files of a worktree the reader has marked read, kept per session.
CREATE TABLE read_files (
    session_id  TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE ON UPDATE CASCADE,
    worktree_id TEXT NOT NULL REFERENCES worktrees(id) ON DELETE CASCADE ON UPDATE CASCADE,
    path        TEXT NOT NULL,
    PRIMARY KEY (session_id, worktree_id, path)
);
