-- Drop the ON DELETE CASCADE foreign keys from v2_job's sparse side tables.
-- These cascades made bulk retention deletes (DELETE FROM v2_job WHERE id = ANY(...))
-- fire a per-row RI trigger for each FK; for flow_conversation_message, whose job_id
-- column is unindexed, that meant a sequential scan of the whole table per deleted row
-- (benchmarked at ~14x the base delete time). Deletion of these tables is now handled
-- explicitly by windmill_common::jobs::delete_jobs and the workspace/export delete paths,
-- following the existing no-FK precedent of job_logs / job_stats / native_retry_attempt.

ALTER TABLE dispatch_event DROP CONSTRAINT IF EXISTS dispatch_event_producer_job_id_fkey;
ALTER TABLE flow_conversation_message DROP CONSTRAINT IF EXISTS flow_conversation_message_job_id_fkey;
ALTER TABLE zombie_job_counter DROP CONSTRAINT IF EXISTS zombie_job_counter_job_id_fkey;
