-- Where the user put a session on the rail: its section, and its place in it.
ALTER TABLE session_state ADD COLUMN section TEXT NOT NULL DEFAULT 'active';
ALTER TABLE session_state ADD COLUMN rank INTEGER;
