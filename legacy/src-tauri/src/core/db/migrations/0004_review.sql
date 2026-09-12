-- Review sessions: diff/log against the MR's real target branch, and
-- per-file "viewed" tracking for review progress.

ALTER TABLE worktrees ADD COLUMN base_ref TEXT;

CREATE TABLE IF NOT EXISTS reviewed_files (
    task_id   TEXT NOT NULL,
    repo_id   TEXT NOT NULL,
    file_path TEXT NOT NULL,
    PRIMARY KEY (task_id, repo_id, file_path)
);
