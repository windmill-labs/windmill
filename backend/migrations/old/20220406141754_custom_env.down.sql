-- Add down migration script here
CREATE TABLE pipenv (
    id SERIAL PRIMARY KEY,
	timestamp TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    created_by VARCHAR(50) NOT NULL,
    python_version VARCHAR(20),
    dependencies VARCHAR(255)[] NOT NULL DEFAULT array[]::varchar[],
    pipfile_lock TEXT,
	job_id UUID
);

ALTER TABLE script
DROP COLUMN lock;

ALTER TABLE script
DROP COLUMN lock_error_logs;

ALTER TABLE worker_ping
ADD COLUMN env_id INTEGER NOT NULL DEFAULT -1;

