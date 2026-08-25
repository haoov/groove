-- Tasks gain a provider. Notion is no longer the only source, so the mirror table
-- and the session's pointer stop naming it.
--
-- ALTER-only: sessions is the parent of six ON UPDATE CASCADE children, and
-- rebuilding it under SQLite needs PRAGMA foreign_keys=OFF, which cannot run inside
-- sqlx's per-migration transaction. RENAME COLUMN rewrites the table's own CHECK,
-- so the task <-> external_id invariant follows the rename untouched.

ALTER TABLE notion_tasks RENAME TO provider_tasks;
ALTER TABLE provider_tasks RENAME COLUMN page_id TO external_id;

ALTER TABLE provider_tasks ADD COLUMN provider TEXT NOT NULL DEFAULT 'notion'
    CHECK (provider IN ('notion', 'github'));
-- Where the task lives, for the MR footer and "open in browser".
ALTER TABLE provider_tasks ADD COLUMN url TEXT;
-- The Projects v2 board that supplied the fields, when several could have.
ALTER TABLE provider_tasks ADD COLUMN board TEXT;
-- Appended to branch names; NULL means use short_id.
ALTER TABLE provider_tasks ADD COLUMN branch_tag TEXT;

ALTER TABLE sessions RENAME COLUMN notion_page_id TO external_id;
