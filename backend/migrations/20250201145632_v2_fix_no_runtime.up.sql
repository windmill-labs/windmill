-- Add up migration script here
-- Migrate `v2_job_queue` moved columns to `v2_job_runtime`:
INSERT INTO v2_job_runtime (id, ping, memory_peak)
SELECT id, __last_ping, __mem_peak
FROM v2_job_queue
   -- Locked ones will sync within triggers
    FOR UPDATE SKIP LOCKED
ON CONFLICT (id) DO NOTHING;
