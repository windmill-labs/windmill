-- Add up migration script here
CREATE TABLE IF NOT EXISTS job_perms (
    job_id UUID NOT NULL,
    email VARCHAR(255) NOT NULL,
    username VARCHAR(50) NOT NULL,
    is_admin BOOLEAN NOT NULL,
    is_operator BOOLEAN NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    workspace_id VARCHAR(50) NOT NULL,
    groups TEXT[] NOT NULL,
    folders JSONB[] NOT NULL,
    CONSTRAINT job_perms_pk PRIMARY KEY (job_id)
);