-- When the concurrency limiter last re-queued this job for want of a free slot.
-- Records that a gate parked the job rather than a worker being unavailable.
-- A timestamp rather than a flag: the gate can free later, and only a fresh
-- mark means the limiter is still parking. Nullable so adding it is instant.
ALTER TABLE v2_job_queue ADD COLUMN IF NOT EXISTS concurrency_gated_at TIMESTAMPTZ;
