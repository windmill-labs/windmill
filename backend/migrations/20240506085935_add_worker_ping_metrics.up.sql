-- Add up migration script here
ALTER TABLE worker_ping 
ADD COLUMN current_job_id UUID,
ADD COLUMN current_job_workspace_id VARCHAR(50),
ADD COLUMN vcpus BIGINT,
ADD COLUMN memory BIGINT,
ADD COLUMN occupancy_rate REAL;