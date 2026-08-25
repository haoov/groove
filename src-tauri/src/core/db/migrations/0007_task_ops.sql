-- Approval ops renamed notion.* -> task.*, now that a task can come from more than
-- one provider. Queued confirmations are re-emitted at startup by
-- approvals::surface_pending, so a stored row under the old name would resolve to
-- "unknown op_type".
UPDATE pending_confirmations
   SET op_type = 'task.' || substr(op_type, 8)
 WHERE op_type IN ('notion.property', 'notion.hours', 'notion.body');
