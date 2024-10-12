-- Add up migration script here
-- Add down migration script here
-- Add up migration script here
CREATE TABLE job_params (
  id UUID PRIMARY KEY,
  raw_code TEXT,
  raw_flow jsonb NULL,
  tag VARCHAR(50),
  workspace_id VARCHAR(50)
);

-- Add up migration script here
CREATE TABLE job_args (
  id UUID PRIMARY KEY,
  args JSONB,
  tag VARCHAR(50),
  workspace_id VARCHAR(50)
);

CREATE TABLE completed_jobs_result (
  id UUID PRIMARY KEY,
  result JSONB,
  tag VARCHAR(50),
  workspace_id VARCHAR(50)
);
