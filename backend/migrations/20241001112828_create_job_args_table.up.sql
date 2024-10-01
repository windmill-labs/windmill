-- Add up migration script here
CREATE TABLE job_args (
  id UUID PRIMARY KEY,
  args JSONB,
  tag VARCHAR(50),
  workspace_id VARCHAR(50)
);