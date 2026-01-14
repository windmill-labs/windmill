-- OpenTelemetry Span storage.
--
-- OTEL COMPATIBILITY:
-- All columns are 1-to-1 compatible with the OpenTelemetry Span proto spec.
-- See: https://opentelemetry.io/docs/specs/otel/trace/api/#span
--
-- Additional OTEL Span fields can be added as needed (all are spec-compatible):
--   trace_state TEXT, flags INTEGER, events JSONB, links JSONB,
--   dropped_attributes_count INTEGER, dropped_events_count INTEGER, dropped_links_count INTEGER
--
-- This table is intentionally generic and not tied to jobs. It can store:
--   - Job HTTP request traces
--   - System traces
--   - Upstream traces that span across jobs/workspaces

CREATE TABLE IF NOT EXISTS otel_traces (
    trace_id             VARCHAR(32) NOT NULL,   -- 16 bytes hex-encoded
    span_id              VARCHAR(16) NOT NULL,   -- 8 bytes hex-encoded
    parent_span_id       VARCHAR(16),            -- 8 bytes hex-encoded
    name                 TEXT NOT NULL,          -- operation name (e.g., "GET example.com")
    kind                 SMALLINT NOT NULL,      -- SpanKind enum (1=Internal, 2=Server, 3=Client, etc.)
    start_time_unix_nano BIGINT NOT NULL,
    end_time_unix_nano   BIGINT NOT NULL,
    status               JSONB,                  -- {"code": int, "message": string}
    attributes           JSONB NOT NULL DEFAULT '{}',
    PRIMARY KEY (trace_id, span_id)
);

-- Query spans by trace_id, ordered by time
CREATE INDEX IF NOT EXISTS otel_traces_trace_time_idx ON otel_traces (trace_id, start_time_unix_nano);

-- Time-based cleanup (retention policy)
CREATE INDEX IF NOT EXISTS otel_traces_time_idx ON otel_traces (start_time_unix_nano);


-- Linking table to associate jobs with their trace_id.
-- This allows querying traces for a specific job via LEFT JOIN.
-- A job may have one trace, and a trace may span multiple jobs in the future
-- (e.g., upstream traces that start before job execution).

CREATE TABLE IF NOT EXISTS job_trace (
    job_id    UUID PRIMARY KEY,
    trace_id  VARCHAR(32) NOT NULL
);

CREATE INDEX IF NOT EXISTS job_trace_trace_id_idx ON job_trace (trace_id);
