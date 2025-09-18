-- Add up migration script here
-- Critical queue performance indexes for existing shard tables only

-- Ensure optimal indexes exist on v2_job_queue (already created in base migration but ensuring they're optimal)
DROP INDEX IF EXISTS queue_sort_v2;
CREATE INDEX IF NOT EXISTS queue_sort_v2 
    ON public.v2_job_queue (priority DESC NULLS LAST, scheduled_for, tag) 
    WHERE (running = false);

-- Suspended job handling (critical for performance)
DROP INDEX IF EXISTS queue_suspended;
CREATE INDEX IF NOT EXISTS queue_suspended 
    ON public.v2_job_queue (priority DESC NULLS LAST, created_at, suspend_until, suspend, tag) 
    WHERE (suspend_until IS NOT NULL);

-- Ensure existing indexes are optimized
CREATE INDEX IF NOT EXISTS root_queue_index_by_path 
    ON public.v2_job_queue (workspace_id, created_at);

CREATE INDEX IF NOT EXISTS v2_job_queue_suspend 
    ON public.v2_job_queue (workspace_id, suspend) 
    WHERE (suspend > 0);

-- Job completion indexes for performance
CREATE INDEX IF NOT EXISTS ix_completed_job_workspace_id_started_at_new_2 
    ON public.v2_job_completed (workspace_id, started_at DESC);

CREATE INDEX IF NOT EXISTS ix_job_completed_completed_at 
    ON public.v2_job_completed (completed_at DESC);

-- Labeled jobs performance (for job results with labels)
CREATE INDEX IF NOT EXISTS labeled_jobs_on_jobs 
    ON public.v2_job_completed USING gin (((result -> 'wm_labels'::text))) 
    WHERE (result ? 'wm_labels'::text);

-- V2_job indexes for lookups (used by triggers)
CREATE INDEX IF NOT EXISTS ix_v2_job_workspace_id_created_at 
    ON public.v2_job (workspace_id, created_at DESC) 
    WHERE ((kind = ANY (ARRAY['script'::job_kind, 'flow'::job_kind, 'singlescriptflow'::job_kind])) 
           AND (parent_job IS NULL));

CREATE INDEX IF NOT EXISTS ix_v2_job_labels 
    ON public.v2_job USING gin (labels) 
    WHERE (labels IS NOT NULL);

-- Outstanding wait time performance
CREATE INDEX IF NOT EXISTS outstanding_wait_time_job_id_idx 
    ON public.outstanding_wait_time (job_id);

-- Job perms lookup performance
CREATE INDEX IF NOT EXISTS job_perms_job_id_idx 
    ON public.job_perms (job_id);

CREATE INDEX IF NOT EXISTS job_perms_workspace_created_idx 
    ON public.job_perms (workspace_id, created_at DESC);