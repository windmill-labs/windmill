DROP INDEX IF EXISTS job_trace_trace_id_idx;
DROP TABLE IF EXISTS job_trace;

DROP INDEX IF EXISTS otel_traces_time_idx;
DROP INDEX IF EXISTS otel_traces_trace_time_idx;
DROP TABLE IF EXISTS otel_traces;
