-- Add table to store OTel traces from auto-instrumented scripts
CREATE TABLE IF NOT EXISTS job_otel_traces (
    id BIGSERIAL PRIMARY KEY,
    job_id UUID NOT NULL,
    workspace_id VARCHAR(50) NOT NULL,
    trace_id VARCHAR(32) NOT NULL,
    span_id VARCHAR(16) NOT NULL,
    parent_span_id VARCHAR(16),
    operation_name VARCHAR(255) NOT NULL,
    service_name VARCHAR(255),
    start_time_unix_nano BIGINT NOT NULL,
    end_time_unix_nano BIGINT NOT NULL,
    duration_ns BIGINT GENERATED ALWAYS AS (end_time_unix_nano - start_time_unix_nano) STORED,
    status_code SMALLINT DEFAULT 0,
    status_message TEXT,
    attributes JSONB DEFAULT '{}',
    events JSONB DEFAULT '[]',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Create indexes for efficient querying
CREATE INDEX IF NOT EXISTS idx_job_otel_traces_job_id ON job_otel_traces(job_id);
CREATE INDEX IF NOT EXISTS idx_job_otel_traces_workspace_id ON job_otel_traces(workspace_id);
CREATE INDEX IF NOT EXISTS idx_job_otel_traces_trace_id ON job_otel_traces(trace_id);
CREATE INDEX IF NOT EXISTS idx_job_otel_traces_created_at ON job_otel_traces(created_at);

-- Composite index for common query patterns
CREATE INDEX IF NOT EXISTS idx_job_otel_traces_job_workspace ON job_otel_traces(job_id, workspace_id);
