-- Multiline annotations: a range [start_line, end_line] on the new (working-tree) side.
-- `line_num` remains the anchor (== end_line) for back-compat with existing reads.
ALTER TABLE annotations ADD COLUMN start_line INTEGER NOT NULL DEFAULT 0;
ALTER TABLE annotations ADD COLUMN end_line INTEGER NOT NULL DEFAULT 0;
UPDATE annotations SET start_line = line_num, end_line = line_num;
