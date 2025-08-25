-- Add up migration script here
ALTER TABLE worker_ping ALTER COLUMN worker TYPE VARCHAR(255);
ALTER TABLE worker_ping ALTER COLUMN worker_instance TYPE VARCHAR(255);