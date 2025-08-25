-- Add up migration script here
DROP TABLE pipenv;

ALTER TABLE script
ADD COLUMN lock TEXT;

ALTER TABLE script
ADD COLUMN lock_error_logs TEXT;

ALTER TABLE worker_ping
DROP COLUMN env_id;
