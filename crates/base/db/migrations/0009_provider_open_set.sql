-- The provider column stops enumerating providers.
--
-- 0008 gave it CHECK (provider IN ('notion','github')) and DEFAULT 'notion':
-- the CHECK makes every new provider a schema migration, and the DEFAULT can
-- silently mislabel a row. The app validates the value (ProviderId::parse) and
-- every insert binds it explicitly, so the column is a plain NOT NULL TEXT.
-- SQLite cannot drop a CHECK in place — rebuild the table. It has no foreign
-- keys in either direction and only its two implicit unique indexes.

CREATE TABLE provider_tasks_new (
    external_id TEXT PRIMARY KEY,
    short_id    TEXT NOT NULL UNIQUE,
    title       TEXT NOT NULL,
    status      TEXT NOT NULL,
    priority    TEXT,
    synced_at   INTEGER NOT NULL,
    provider    TEXT NOT NULL,
    url         TEXT,
    board       TEXT,
    branch_tag  TEXT
);

INSERT INTO provider_tasks_new
    SELECT external_id, short_id, title, status, priority, synced_at,
           provider, url, board, branch_tag
    FROM provider_tasks;

DROP TABLE provider_tasks;
ALTER TABLE provider_tasks_new RENAME TO provider_tasks;
