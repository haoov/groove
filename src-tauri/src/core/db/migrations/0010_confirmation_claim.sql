-- A claimed confirmation keeps its row until its outcome is delivered, and one
-- identity can be pending only once.

ALTER TABLE pending_confirmations ADD COLUMN claimed_at INTEGER;

-- Duplicates the old check-then-insert could already have queued.
DELETE FROM pending_confirmations
WHERE rowid NOT IN (
    SELECT MIN(rowid) FROM pending_confirmations
    GROUP BY op_type, IFNULL(session_id, ''), payload
);

CREATE UNIQUE INDEX ux_confirmations_pending
    ON pending_confirmations (op_type, IFNULL(session_id, ''), payload)
    WHERE claimed_at IS NULL;
