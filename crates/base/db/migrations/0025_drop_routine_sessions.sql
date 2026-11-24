-- Routine sessions are gone: their rows go, their table with them.
DELETE FROM sessions WHERE id IN (SELECT session_id FROM routine_sessions);
DROP TABLE routine_sessions;
