-- Add unique constraints required for upsert patterns.
-- Safe to run on empty tables (dev environment).

CREATE UNIQUE INDEX IF NOT EXISTS ux_worktrees_task_repo
    ON worktrees (task_id, repo_id);

CREATE UNIQUE INDEX IF NOT EXISTS ux_tab_snapshots_task_repo_file
    ON tab_snapshots (task_id, repo_id, file_path);
