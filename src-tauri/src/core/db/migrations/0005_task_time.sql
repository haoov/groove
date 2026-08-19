-- Time spent on a task, accumulated locally and logged to Notion deliberately.
--
-- Two counters, never one: `tracked_seconds` is what the app measured,
-- `logged_seconds` is how much of that has been written to the Notion "Hours
-- spent" property. The difference is what there is left to log, which is what
-- makes logging idempotent — pressing the button twice can't double-count.
--
-- `today_*` is display sugar (the number you usually want to log), reset by the
-- writer when the date rolls over.
CREATE TABLE IF NOT EXISTS task_time (
    task_id         TEXT PRIMARY KEY,
    tracked_seconds INTEGER NOT NULL DEFAULT 0,
    logged_seconds  INTEGER NOT NULL DEFAULT 0,
    today_date      TEXT    NOT NULL DEFAULT '',
    today_seconds   INTEGER NOT NULL DEFAULT 0,
    updated_at      INTEGER NOT NULL
);
