-- Drop indexes
DROP INDEX IF EXISTS idx_job_otel_traces_job_workspace;
DROP INDEX IF EXISTS idx_job_otel_traces_created_at;
DROP INDEX IF EXISTS idx_job_otel_traces_trace_id;
DROP INDEX IF EXISTS idx_job_otel_traces_workspace_id;
DROP INDEX IF EXISTS idx_job_otel_traces_job_id;

-- Drop table
DROP TABLE IF EXISTS job_otel_traces;
