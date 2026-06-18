-- OpenTelemetry Span storage (all fields from proto::Span).
-- See: https://github.com/open-telemetry/opentelemetry-proto/blob/main/opentelemetry/proto/trace/v1/trace.proto

CREATE TABLE IF NOT EXISTS otel_traces (
    -- Identity fields (BYTEA for efficient storage and querying)
    trace_id             BYTEA NOT NULL,              -- 16 bytes (proto: bytes)
    span_id              BYTEA NOT NULL,              -- 8 bytes (proto: bytes)
    trace_state          TEXT NOT NULL DEFAULT '',    -- W3C trace-context (proto: string)
    parent_span_id       BYTEA NOT NULL DEFAULT '', -- 8 bytes, empty if root span (proto: bytes)
    flags                INTEGER NOT NULL DEFAULT 0, -- W3C trace flags (proto: fixed32)
    -- Core fields
    name                 TEXT NOT NULL,              -- operation name (proto: string)
    kind                 INTEGER NOT NULL,           -- SpanKind enum (proto: int32)
    start_time_unix_nano BIGINT NOT NULL,            -- (proto: fixed64, postgres has no u64)
    end_time_unix_nano   BIGINT NOT NULL,            -- (proto: fixed64, postgres has no u64)
    -- Attributes
    attributes                 JSONB NOT NULL DEFAULT '[]', -- (proto: repeated KeyValue)
    dropped_attributes_count   INTEGER NOT NULL DEFAULT 0,  -- (proto: uint32)
    -- Events
    events                     JSONB NOT NULL DEFAULT '[]', -- (proto: repeated Event)
    dropped_events_count       INTEGER NOT NULL DEFAULT 0,  -- (proto: uint32)
    -- Links
    links                      JSONB NOT NULL DEFAULT '[]', -- (proto: repeated Link)
    dropped_links_count        INTEGER NOT NULL DEFAULT 0,  -- (proto: uint32)
    -- Status
    status               JSONB,                      -- (proto: optional Status message)
    PRIMARY KEY (trace_id, span_id)
);

-- Query spans by trace_id, ordered by time
CREATE INDEX IF NOT EXISTS otel_traces_trace_time_idx ON otel_traces (trace_id, start_time_unix_nano);

-- Time-based cleanup (retention policy)
CREATE INDEX IF NOT EXISTS otel_traces_time_idx ON otel_traces (start_time_unix_nano);

-- trace_id = job_id.as_bytes()
