-- When the concurrency limiter last re-queued this job for want of a free slot,
-- and under which key. Records that a gate parked the job rather than a worker
-- being unavailable. The key is carried here so freshness can be judged per
-- gate: gates on one tag free independently of each other.
ALTER TABLE v2_job_queue ADD COLUMN IF NOT EXISTS concurrency_gated_at TIMESTAMPTZ;
ALTER TABLE v2_job_queue ADD COLUMN IF NOT EXISTS concurrency_gated_key VARCHAR(255);
