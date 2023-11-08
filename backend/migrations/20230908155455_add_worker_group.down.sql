-- Add down migration script here
DROP TABLE worker_group_config;

ALTER TABLE worker_ping DROP COLUMN worker_group;