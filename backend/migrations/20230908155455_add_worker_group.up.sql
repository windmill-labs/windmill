-- Add up migration script here
CREATE TABLE worker_group_config (
    name VARCHAR(255) PRIMARY KEY,
    config JSONB DEFAULT '{}'::jsonb
);

ALTER TABLE worker_ping ADD COLUMN IF NOT EXISTS worker_group VARCHAR(255) NOT NULL DEFAULT 'default';
ALTER TABLE worker_ping ADD COLUMN IF NOT EXISTS dedicated_worker VARCHAR(255);