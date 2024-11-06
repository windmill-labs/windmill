CREATE TABLE job_definition (
  id UUID PRIMARY KEY,
  raw_code TEXT,
  raw_lock TEXT,
  raw_flow jsonb NULL,
  tag VARCHAR(50),
  workspace_id VARCHAR(50)
);
