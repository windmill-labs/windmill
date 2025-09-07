DROP POLICY IF EXISTS admin_policy ON public.v2_job_queue;

DROP INDEX IF EXISTS queue_sort_v2;
DROP INDEX IF EXISTS queue_suspended;
DROP INDEX IF EXISTS root_queue_index_by_path;
DROP INDEX IF EXISTS v2_job_queue_suspend;

DROP TABLE IF EXISTS public.v2_job_queue;
