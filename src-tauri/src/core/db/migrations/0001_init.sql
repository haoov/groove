-- Platform Workbench — initial schema
-- 9 tables; WAL mode is set at connection time.

CREATE TABLE IF NOT EXISTS tasks (
    short_id        TEXT PRIMARY KEY,
    notion_page_id  TEXT NOT NULL,
    title           TEXT NOT NULL,
    status          TEXT NOT NULL,
    priority        TEXT,
    last_synced_at  INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS repos (
    id          TEXT PRIMARY KEY,
    host        TEXT NOT NULL,
    group_path  TEXT NOT NULL,
    project     TEXT NOT NULL,
    local_path  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS worktrees (
    id          TEXT PRIMARY KEY,
    task_id     TEXT NOT NULL REFERENCES tasks(short_id),
    repo_id     TEXT NOT NULL REFERENCES repos(id),
    branch      TEXT NOT NULL,
    path        TEXT NOT NULL,
    is_active   INTEGER NOT NULL DEFAULT 1,
    created_at  INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS mrs (
    id           TEXT PRIMARY KEY,
    worktree_id  TEXT NOT NULL REFERENCES worktrees(id),
    platform     TEXT NOT NULL,
    remote_id    TEXT NOT NULL,
    url          TEXT NOT NULL,
    state        TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS task_repos (
    task_id   TEXT NOT NULL REFERENCES tasks(short_id),
    repo_id   TEXT NOT NULL REFERENCES repos(id),
    added_at  INTEGER NOT NULL,
    PRIMARY KEY (task_id, repo_id)
);

CREATE TABLE IF NOT EXISTS tab_snapshots (
    id          TEXT PRIMARY KEY,
    task_id     TEXT NOT NULL REFERENCES tasks(short_id),
    repo_id     TEXT NOT NULL REFERENCES repos(id),
    file_path   TEXT NOT NULL,
    scroll_top  INTEGER NOT NULL DEFAULT 0,
    tab_order   INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS agent_sessions (
    id            TEXT PRIMARY KEY,
    task_id       TEXT NOT NULL REFERENCES tasks(short_id),
    session_name  TEXT NOT NULL,
    pty_type      TEXT NOT NULL,
    started_at    INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS pending_confirmations (
    id        TEXT PRIMARY KEY,
    task_id   TEXT REFERENCES tasks(short_id),
    op_type   TEXT NOT NULL,
    payload   TEXT NOT NULL,
    origin    TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS annotations (
    id          TEXT PRIMARY KEY,
    task_id     TEXT NOT NULL REFERENCES tasks(short_id),
    repo_id     TEXT NOT NULL REFERENCES repos(id),
    file_path   TEXT NOT NULL,
    line_num    INTEGER NOT NULL,
    content     TEXT NOT NULL,
    author      TEXT NOT NULL,
    status      TEXT NOT NULL DEFAULT 'open',
    created_at  INTEGER NOT NULL
);
