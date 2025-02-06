-- Add up migration script here
UPDATE v2_job_queue SET
    running = false,
    started_at = NULL
WHERE running = true AND NOT EXISTS (SELECT 1 FROM v2_job_runtime WHERE id = v2_job_queue.id);

INSERT INTO v2_job_runtime (id, ping, memory_peak)
SELECT id, __last_ping, __mem_peak
FROM v2_job_queue
     -- Locked ones will sync within triggers
    FOR UPDATE SKIP LOCKED
ON CONFLICT (id) DO NOTHING;
