-- The five Groove reads, kept beside the task so a restart shows it before a sync.
ALTER TABLE provider_tasks ADD COLUMN intent TEXT;
ALTER TABLE provider_tasks ADD COLUMN start_day TEXT;
ALTER TABLE provider_tasks ADD COLUMN due_day TEXT;
ALTER TABLE provider_tasks ADD COLUMN estimate REAL;
