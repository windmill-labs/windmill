-- Add up migration script here

CREATE TABLE completed_jobs_result (
  id UUID PRIMARY KEY,
  result JSONB,
  tag VARCHAR(50),
  workspace_id VARCHAR(50)
);
