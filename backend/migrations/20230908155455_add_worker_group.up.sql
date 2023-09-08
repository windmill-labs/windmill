-- Add up migration script here
CREATE TABLE worker_group_config (
    name VARCHAR(255) PRIMARY KEY,
    config JSONB DEFAULT '{}'::jsonb
)