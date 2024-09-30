-- Add up migration script here
-- Add down migration script here

CREATE TABLE job_params (
  id UUID PRIMARY KEY,
  args JSONB,
  raw_code TEXT,
  raw_flow jsonb NULL
);