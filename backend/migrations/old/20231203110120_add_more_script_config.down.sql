-- Add down migration script here
ALTER TABLE script DROP COLUMN timeout;
ALTER TABLE script DROP COLUMN delete_after_use;
ALTER TABLE script DROP COLUMN restart_unless_cancelled;
ALTER TABLE flow DROP COLUMN timeout;
