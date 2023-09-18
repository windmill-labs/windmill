-- Add up migration script here
ALTER TABLE IF exists worker_group_config RENAME TO config;
UPDATE config SET name = 'worker__' || name;