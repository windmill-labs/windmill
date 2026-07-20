-- Set by the concurrency limiter when it re-queues a job it could not admit.
-- Records that the job is parked behind its own concurrency gate rather than
-- waiting for a worker, which the queue row cannot otherwise show: the
-- limiter's admission test spans concurrency_counter, the completed-window rows
-- in concurrency_key, per-version setting fallbacks and several bypass paths.
-- Nullable so adding it does not rewrite the table.
ALTER TABLE v2_job_queue ADD COLUMN IF NOT EXISTS concurrency_gated BOOLEAN;
