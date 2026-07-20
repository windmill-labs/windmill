-- When the concurrency limiter first parked this job behind a gate. Unlike
-- concurrency_gated_at, which the limiter overwrites on every re-queue, this is
-- set once and never moves, so it marks a job's arrival at the gate rather than
-- its last rejection. The oversubscription alert measures arrival rate from it:
-- created_at misses jobs pushed with a future scheduled_for that only reach the
-- gate long after creation. Nullable so adding it is instant.
ALTER TABLE v2_job_queue ADD COLUMN IF NOT EXISTS concurrency_first_gated_at TIMESTAMPTZ;
