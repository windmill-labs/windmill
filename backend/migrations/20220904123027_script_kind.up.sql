-- Add up migration script here
CREATE TYPE SCRIPT_KIND AS ENUM ('script', 'trigger', 'failure', 'command');

ALTER TABLE script ADD COLUMN kind SCRIPT_KIND NOT NULL DEFAULT 'script';
ALTER TABLE script DROP COLUMN is_trigger;