-- Add up migration script here
ALTER TABLE script ADD COLUMN timeout INTEGER;
ALTER TABLE flow ADD COLUMN timeout INTEGER;
ALTER TABLE script ADD COLUMN delete_after_use BOOLEAN;
ALTER TABLE script ADD COLUMN restart_unless_cancelled SMALLINT;
