-- Sessions split: `tasks` becomes `sessions` (local state) + `notion_tasks`
-- (queue mirror). Children gain real foreign keys with cascades. Dead tables
-- (tab_snapshots, agent_sessions, reviewed_files) are dropped, task_time
-- becomes a per-day ledger, review identity becomes columns.
--
-- Old review sessions are NOT carried over: their (project, iid) identity is
-- unrecoverable from the hash-derived ids. Reopening from the review queue
-- recreates them.

PRAGMA defer_foreign_keys = ON;

ALTER TABLE tasks RENAME TO old_tasks;
ALTER TABLE task_repos RENAME TO old_task_repos;
ALTER TABLE worktrees RENAME TO old_worktrees;
ALTER TABLE mrs RENAME TO old_mrs;
ALTER TABLE annotations RENAME TO old_annotations;
ALTER TABLE task_time RENAME TO old_task_time;
ALTER TABLE pending_confirmations RENAME TO old_pending_confirmations;

CREATE TABLE sessions (
    id              TEXT PRIMARY KEY,
    kind            TEXT NOT NULL CHECK (kind IN ('task','explorer','review')),
    state           TEXT NOT NULL DEFAULT 'open' CHECK (state IN ('open','paused')),
    title           TEXT NOT NULL,
    notion_page_id  TEXT,
    review_project  TEXT,
    review_iid      INTEGER,
    created_at      INTEGER NOT NULL,
    CHECK ((notion_page_id IS NOT NULL) = (kind = 'task')),
    CHECK ((review_project IS NOT NULL) = (kind = 'review')),
    CHECK ((review_iid     IS NOT NULL) = (kind = 'review')),
    UNIQUE (review_project, review_iid)
);

CREATE TABLE notion_tasks (
    page_id    TEXT PRIMARY KEY,
    short_id   TEXT NOT NULL UNIQUE,
    title      TEXT NOT NULL,
    status     TEXT NOT NULL,
    priority   TEXT,
    synced_at  INTEGER NOT NULL
);

CREATE TABLE session_repos (
    session_id  TEXT NOT NULL REFERENCES sessions(id)
                  ON DELETE CASCADE ON UPDATE CASCADE,
    repo_id     TEXT NOT NULL REFERENCES repos(id),
    added_at    INTEGER NOT NULL,
    PRIMARY KEY (session_id, repo_id)
);

CREATE TABLE worktrees (
    id          TEXT PRIMARY KEY,
    session_id  TEXT NOT NULL REFERENCES sessions(id)
                  ON DELETE CASCADE ON UPDATE CASCADE,
    repo_id     TEXT NOT NULL REFERENCES repos(id),
    branch      TEXT NOT NULL,
    path        TEXT NOT NULL UNIQUE,
    base_ref    TEXT,
    created_at  INTEGER NOT NULL,
    UNIQUE (session_id, repo_id, branch)
);

CREATE TABLE mrs (
    id           TEXT PRIMARY KEY,
    worktree_id  TEXT NOT NULL REFERENCES worktrees(id) ON DELETE CASCADE,
    platform     TEXT NOT NULL CHECK (platform IN ('gitlab','github')),
    remote_id    TEXT NOT NULL,
    url          TEXT NOT NULL,
    state        TEXT NOT NULL,
    UNIQUE (worktree_id, remote_id)
);

CREATE TABLE annotations (
    id          TEXT PRIMARY KEY,
    session_id  TEXT NOT NULL REFERENCES sessions(id)
                  ON DELETE CASCADE ON UPDATE CASCADE,
    repo_id     TEXT NOT NULL REFERENCES repos(id),
    file_path   TEXT NOT NULL,
    start_line  INTEGER NOT NULL,
    end_line    INTEGER NOT NULL,
    content     TEXT NOT NULL,
    author      TEXT NOT NULL,
    status      TEXT NOT NULL DEFAULT 'open' CHECK (status IN ('open','resolved')),
    created_at  INTEGER NOT NULL,
    CHECK (start_line <= end_line)
);
CREATE INDEX ix_annotations_session_file ON annotations (session_id, file_path);

CREATE TABLE time_entries (
    session_id  TEXT NOT NULL REFERENCES sessions(id)
                  ON DELETE CASCADE ON UPDATE CASCADE,
    day         TEXT NOT NULL,
    seconds     INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (session_id, day)
);

CREATE TABLE time_logs (
    id          TEXT PRIMARY KEY,
    session_id  TEXT NOT NULL REFERENCES sessions(id)
                  ON DELETE CASCADE ON UPDATE CASCADE,
    seconds     INTEGER NOT NULL,
    logged_at   INTEGER NOT NULL
);

CREATE TABLE pending_confirmations (
    id          TEXT PRIMARY KEY,
    session_id  TEXT REFERENCES sessions(id)
                  ON DELETE CASCADE ON UPDATE CASCADE,
    op_type     TEXT NOT NULL,
    payload     TEXT NOT NULL,
    origin      TEXT NOT NULL CHECK (origin IN ('ui','mcp')),
    created_at  INTEGER NOT NULL
);
CREATE INDEX ix_confirmations_session ON pending_confirmations (session_id);

-- Sessions: explorers plus any real task that has a worktree. Review sessions
-- are dropped (see header); the old desk row is not carried over.
INSERT INTO sessions (id, kind, state, title, notion_page_id, created_at)
SELECT
    t.short_id,
    CASE WHEN t.notion_page_id = '' THEN 'explorer' ELSE 'task' END,
    CASE
        WHEN EXISTS (SELECT 1 FROM old_worktrees w
                     WHERE w.task_id = t.short_id AND w.is_active = 0)
         AND NOT EXISTS (SELECT 1 FROM old_worktrees w
                     WHERE w.task_id = t.short_id AND w.is_active = 1)
        THEN 'paused' ELSE 'open'
    END,
    t.title,
    NULLIF(t.notion_page_id, ''),
    t.last_synced_at
FROM old_tasks t
WHERE t.short_id NOT LIKE 'review-%'
  AND t.short_id != 'desk'
  AND (t.notion_page_id = ''
       OR t.short_id IN (SELECT task_id FROM old_worktrees));

INSERT INTO notion_tasks (page_id, short_id, title, status, priority, synced_at)
SELECT notion_page_id, short_id, title, status, priority, last_synced_at
FROM old_tasks
WHERE notion_page_id != '';

INSERT INTO session_repos (session_id, repo_id, added_at)
SELECT task_id, repo_id, added_at FROM old_task_repos
WHERE task_id IN (SELECT id FROM sessions)
  AND repo_id IN (SELECT id FROM repos);

INSERT OR IGNORE INTO worktrees (id, session_id, repo_id, branch, path, base_ref, created_at)
SELECT id, task_id, repo_id, branch, path, base_ref, created_at
FROM old_worktrees
WHERE task_id IN (SELECT id FROM sessions)
  AND repo_id IN (SELECT id FROM repos);

INSERT OR IGNORE INTO mrs (id, worktree_id, platform, remote_id, url, state)
SELECT id, worktree_id,
       CASE WHEN platform IN ('gitlab','github') THEN platform ELSE 'gitlab' END,
       remote_id, url, state
FROM old_mrs
WHERE worktree_id IN (SELECT id FROM worktrees);

INSERT INTO annotations (id, session_id, repo_id, file_path, start_line, end_line,
                         content, author, status, created_at)
SELECT id, task_id, repo_id, file_path,
       MIN(start_line, end_line), MAX(start_line, end_line),
       content, author,
       CASE WHEN status IN ('open','resolved') THEN status ELSE 'open' END,
       created_at
FROM old_annotations
WHERE task_id IN (SELECT id FROM sessions)
  AND repo_id IN (SELECT id FROM repos);

INSERT INTO time_entries (session_id, day, seconds)
SELECT task_id, COALESCE(NULLIF(today_date, ''), '1970-01-01'), tracked_seconds
FROM old_task_time
WHERE tracked_seconds > 0 AND task_id IN (SELECT id FROM sessions);

INSERT INTO time_logs (id, session_id, seconds, logged_at)
SELECT lower(hex(randomblob(16))), task_id, logged_seconds, updated_at
FROM old_task_time
WHERE logged_seconds > 0 AND task_id IN (SELECT id FROM sessions);

INSERT INTO pending_confirmations (id, session_id, op_type, payload, origin, created_at)
SELECT id,
       CASE WHEN task_id IN (SELECT id FROM sessions) THEN task_id ELSE NULL END,
       op_type, payload,
       CASE WHEN origin IN ('ui','mcp') THEN origin ELSE 'ui' END,
       unixepoch()
FROM old_pending_confirmations;

DROP TABLE old_pending_confirmations;
DROP TABLE old_task_time;
DROP TABLE old_annotations;
DROP TABLE old_mrs;
DROP TABLE old_worktrees;
DROP TABLE old_task_repos;
DROP TABLE old_tasks;
DROP TABLE IF EXISTS tab_snapshots;
DROP TABLE IF EXISTS agent_sessions;
DROP TABLE IF EXISTS reviewed_files;
