-- When the limiter first parked this job behind a gate; set once, never moved
-- (unlike concurrency_gated_at, overwritten each re-queue). The oversubscription
-- alert measures arrival rate from it because created_at misses jobs pushed with
-- a future scheduled_for that reach the gate long after creation.
ALTER TABLE v2_job_queue ADD COLUMN IF NOT EXISTS concurrency_first_gated_at TIMESTAMPTZ;
