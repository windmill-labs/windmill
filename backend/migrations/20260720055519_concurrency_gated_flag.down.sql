ALTER TABLE v2_job_queue DROP COLUMN IF EXISTS concurrency_gated_at;
ALTER TABLE v2_job_queue DROP COLUMN IF EXISTS concurrency_gate_id;
