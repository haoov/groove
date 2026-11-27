-- Local annotations are gone: Groove keeps only the MR threads.
DROP INDEX IF EXISTS ix_annotations_session_file;
DROP TABLE annotations;
