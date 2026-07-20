-- When the concurrency limiter last re-queued this job for want of a free slot,
-- and the gate it was rejected by. The gate identity is the whole admission
-- policy (key, limit, window), not just the key: one key may carry several
-- policies, and only jobs under the same one share a gate.
ALTER TABLE v2_job_queue ADD COLUMN IF NOT EXISTS concurrency_gated_at TIMESTAMPTZ;
ALTER TABLE v2_job_queue ADD COLUMN IF NOT EXISTS concurrency_gate_id VARCHAR(320);

-- Serves the queue-depth alert's per-gate liveness aggregate. Partial so it only
-- covers jobs a gate actually parked, leaving the ordinary queue path unindexed.
CREATE INDEX IF NOT EXISTS v2_job_queue_concurrency_gate
    ON v2_job_queue (concurrency_gate_id, concurrency_gated_at DESC)
    WHERE running = false AND concurrency_gate_id IS NOT NULL;
