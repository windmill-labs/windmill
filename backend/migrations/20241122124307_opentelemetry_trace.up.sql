-- Add up migration script here
CREATE TABLE opentelemetry_spans (
    parent_job_id UUID PRIMARY KEY,
    trace_id VARCHAR(32) NOT NULL,
    span_id VARCHAR(16) NOT NULL
);